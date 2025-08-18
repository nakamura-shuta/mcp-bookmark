mod bookmark;
mod config;
mod content;
mod mcp_server;
mod search;

use anyhow::Result;
use bookmark::BookmarkReader;
use config::Config;
use content::ContentFetcher;
use mcp_server::BookmarkServer;
use rmcp::{ServiceExt, transport::stdio};
use search::{
    readonly_index::ReadOnlyIndexManager,
    search_manager_trait::SearchManagerTrait,
};
use std::env;
use std::sync::Arc;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{self, EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Parse command-line arguments and build configuration
fn parse_args() -> Result<Config> {
    let args: Vec<String> = env::args().collect();
    let mut config = Config::default();
    let mut i = 1;

    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--list-indexes" => {
                list_indexes();
                std::process::exit(0);
            }
            "--clear-index" => {
                if i + 1 < args.len() {
                    clear_index(Some(&args[i + 1]));
                    i += 1;
                } else {
                    println!("Error: --clear-index requires an index name");
                    std::process::exit(1);
                }
                std::process::exit(0);
            }
            "--clear-all-indexes" => {
                clear_all_indexes();
                std::process::exit(0);
            }
            _ => {
                // Try to parse as number (max bookmarks)
                if let Ok(max) = arg.parse::<usize>() {
                    config.max_bookmarks = max;
                }
            }
        }
        i += 1;
    }

    // Read INDEX_NAME from environment variable (required)
    if let Ok(index_name) = env::var("INDEX_NAME") {
        tracing::info!("Using index: {}", index_name);
        config.index_name = Some(index_name);
    } else {
        eprintln!("Error: INDEX_NAME environment variable is required");
        eprintln!();
        eprintln!("Please specify the index to use:");
        eprintln!("  export INDEX_NAME=your_index_name");
        eprintln!();
        eprintln!("Available indexes:");
        list_available_indexes();
        std::process::exit(1);
    }

    Ok(config)
}

/// Print help message
fn print_help() {
    println!("Chrome Bookmark MCP Server (Simplified)\n");
    println!("Usage: mcp-bookmark [options]\n");
    println!("Environment variables:");
    println!("  INDEX_NAME       Name of the index to use (required)\n");
    println!("Options:");
    println!("  --help, -h            Show this help message");
    println!("  --list-indexes        List all available indexes");
    println!("  --clear-index <name>  Clear specific index");
    println!("  --clear-all-indexes   Clear all indexes\n");
    println!("Examples:");
    println!("  INDEX_NAME=my_work_bookmarks mcp-bookmark");
    println!("  INDEX_NAME=Extension_Development mcp-bookmark");
}

/// List available indexes (simplified output)
fn list_available_indexes() {
    let base_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("mcp-bookmark");

    if !base_dir.exists() {
        println!("  No indexes found. Use the Chrome extension to create one.");
        return;
    }

    let mut found = false;
    if let Ok(entries) = std::fs::read_dir(&base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.file_name().unwrap() != "logs" {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Check if it's a valid index
                    if path.join("meta.json").exists() {
                        found = true;
                        println!("  - {name}");
                    }
                }
            }
        }
    }

    if !found {
        println!("  No indexes found. Use the Chrome extension to create one.");
    }
}

/// List all available indexes
fn list_indexes() {
    let base_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("mcp-bookmark");

    println!("Available indexes:");
    println!("==================\n");

    if !base_dir.exists() {
        println!("No indexes found.");
        return;
    }

    let mut found = false;
    if let Ok(entries) = std::fs::read_dir(&base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.file_name().unwrap() != "logs" {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Check if it's a valid index
                    if path.join("meta.json").exists() {
                        found = true;
                        print!("  {name}");

                        // Read metadata if exists
                        let meta_path = path.join("meta.json");
                        if let Ok(content) = std::fs::read_to_string(meta_path) {
                            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(count) = meta["bookmark_count"].as_u64() {
                                    print!(" ({count} bookmarks");
                                }
                                if let Some(updated) = meta["last_updated"].as_str() {
                                    print!(", updated: {updated}");
                                }
                                print!(")");
                            }
                        }

                        // Show size
                        if let Ok(size) = get_dir_size(&path) {
                            let (size_str, unit) = if size < 1024 {
                                (size as f64, "B")
                            } else if size < 1024 * 1024 {
                                (size as f64 / 1024.0, "KB")
                            } else {
                                (size as f64 / 1024.0 / 1024.0, "MB")
                            };
                            print!(" [{size_str:.1}{unit}]");
                        }

                        println!();
                    }
                }
            }
        }
    }

    if !found {
        println!("No indexes found.");
    }
}

/// Clear specific index
fn clear_index(index_name: Option<&str>) {
    let Some(name) = index_name else {
        println!("Error: Index name is required");
        return;
    };

    let base_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("mcp-bookmark");

    let index_dir = base_dir.join(name);

    if !index_dir.exists() {
        println!("Index not found: {name}");
        return;
    }

    match std::fs::remove_dir_all(&index_dir) {
        Ok(_) => println!("Index cleared: {name}"),
        Err(e) => println!("Failed to clear index: {e}"),
    }
}

/// Clear all indexes
fn clear_all_indexes() {
    let base_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("mcp-bookmark");

    if !base_dir.exists() {
        println!("No indexes found.");
        return;
    }

    let mut cleared = 0;
    if let Ok(entries) = std::fs::read_dir(&base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.file_name().unwrap() != "logs" {
                if let Err(e) = std::fs::remove_dir_all(&path) {
                    println!("Failed to clear {path:?}: {e}");
                } else {
                    cleared += 1;
                }
            }
        }
    }

    println!("Cleared {cleared} indexes.");
}

/// Get directory size recursively
fn get_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut size = 0;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                size += get_dir_size(&path)?;
            } else {
                size += entry.metadata()?.len();
            }
        }
    }
    Ok(size)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with file output
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("mcp-bookmark")
        .join("logs");

    // Create log directory if it doesn't exist
    std::fs::create_dir_all(&log_dir).ok();

    // Create file appender with daily rotation
    let file_appender = rolling::daily(log_dir.clone(), "mcp-bookmark.log");
    let (non_blocking_file, _guard) = non_blocking(file_appender);

    // Create console writer for stderr
    let (non_blocking_console, _guard2) = non_blocking(std::io::stderr());

    // Set up logging to both file and console
    let env_filter = EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into())
        .add_directive("tantivy=warn".parse().unwrap())
        .add_directive("mcp_bookmark::search::indexer=debug".parse().unwrap())
        .add_directive("mcp_bookmark::search::content_index=info".parse().unwrap());

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false);

    let console_layer = fmt::layer()
        .with_writer(non_blocking_console)
        .with_ansi(false)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer)
        .init();

    tracing::debug!("Logging to: {}", log_dir.display());

    // Parse command-line arguments
    let config = parse_args()?;

    tracing::info!("Starting Chrome Bookmark MCP Server (Simplified)");
    if let Some(index_name) = &config.index_name {
        tracing::info!("Using index: {}", index_name);
    }
    if config.max_bookmarks > 0 {
        tracing::debug!("Max bookmarks: {}", config.max_bookmarks);
    }

    // Create MCP server components
    let reader = Arc::new(BookmarkReader::with_config(config.clone())?);
    let fetcher = Arc::new(ContentFetcher::new()?);

    // Initialize search manager (always use read-only mode for pre-built indexes)
    tracing::debug!("Initializing search index...");

    let search_manager: Arc<dyn SearchManagerTrait> =
        match ReadOnlyIndexManager::new_with_index_name(config.index_name.as_deref().unwrap()).await
        {
            Ok(manager) => {
                tracing::info!("Using index in read-only mode (lock-free)");
                Arc::new(manager)
            }
            Err(e) => {
                tracing::error!("Failed to open index: {}", e);
                eprintln!(
                    "Error: Failed to open index '{}': {}",
                    config.index_name.as_deref().unwrap_or(""),
                    e
                );
                eprintln!("\nPlease check:");
                eprintln!("  1. The index exists (use --list-indexes to see available indexes)");
                eprintln!("  2. The index was created using the Chrome extension");
                eprintln!("  3. The index name is correct");
                std::process::exit(1);
            }
        };

    tracing::info!("Server ready");
    tracing::info!("{}", search_manager.get_indexing_status());

    let server = BookmarkServer::new(reader, search_manager);

    // Serve the MCP server
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
