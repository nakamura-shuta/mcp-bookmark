mod bookmark;
mod chrome_profile;
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
use search::ContentIndexManager;
use std::env;
use std::sync::Arc;
use tracing_subscriber::{self, EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::{non_blocking, rolling};

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
            "--profile" if i + 1 < args.len() => {
                config.profile_name = Some(args[i + 1].clone());
                i += 2;
                continue;
            }
            "--folder" if i + 1 < args.len() => {
                config.target_folder = Some(args[i + 1].clone());
                i += 2;
                continue;
            }
            "--exclude" if i + 1 < args.len() => {
                config.exclude_folders = parse_folder_argument(&args[i + 1]);
                i += 2;
                continue;
            }
            _ => {
                // Try to parse as number (max bookmarks)
                if let Ok(max) = arg.parse::<usize>() {
                    config.max_bookmarks = max;
                } else if i == 1 && !arg.starts_with('-') {
                    // First non-flag argument is folder name(s)
                    config.include_folders = parse_folder_argument(arg);
                }
            }
        }
        i += 1;
    }

    // 環境変数からも読み込み
    if config.profile_name.is_none() {
        config.profile_name = env::var("CHROME_PROFILE_NAME").ok();
    }
    if config.target_folder.is_none() {
        config.target_folder = env::var("CHROME_TARGET_FOLDER").ok();
    }

    Ok(config)
}

/// Print help message
fn print_help() {
    println!("Chrome Bookmark MCP Server\n");
    println!("Usage: mcp-bookmark [options]\n");
    println!("Examples:");
    println!("  mcp-bookmark                    # All bookmarks");
    println!("  mcp-bookmark Development         # Only Development folder");
    println!("  mcp-bookmark Development 10      # Max 10 bookmarks from Development");
    println!("  mcp-bookmark Work,Tech 20        # Max 20 bookmarks from Work and Tech\n");
    println!("Advanced options:");
    println!("  --profile <name>     Chrome profile name (e.g., 'Work' or 'Profile 1')");
    println!("  --folder <name>      Target folder name (language independent)");
    println!("  --exclude <folders>  Exclude specified folders");
    println!("\nEnvironment variables:");
    println!("  CHROME_PROFILE_NAME  Chrome profile name");
    println!("  CHROME_TARGET_FOLDER Target folder name");
}

/// Parse folder argument into folder paths
/// Handles both simple folder names and full paths
fn parse_folder_argument(arg: &str) -> Vec<Vec<String>> {
    arg.split(',')
        .map(|folder_name| {
            if !folder_name.contains('/') {
                // Simple folder name: assume under "Bookmarks Bar" with Japanese support
                // This handles Japanese Chrome where bookmark bar is named "ブックマーク バー"
                vec![
                    "Bookmarks Bar".to_string(),
                    "ブックマーク バー".to_string(),
                    folder_name.to_string(),
                ]
            } else {
                // Full path provided
                folder_name.split('/').map(String::from).collect()
            }
        })
        .collect()
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

    tracing::info!("Starting Chrome Bookmark MCP Server");
    if let Some(profile_name) = &config.profile_name {
        tracing::debug!("Using profile: {}", profile_name);
    }
    if let Some(target_folder) = &config.target_folder {
        tracing::debug!("Target folder: {}", target_folder);
    }
    if !config.include_folders.is_empty() {
        tracing::debug!("Including folders: {:?}", config.include_folders);
    }
    if !config.exclude_folders.is_empty() {
        tracing::debug!("Excluding folders: {:?}", config.exclude_folders);
    }
    if config.max_bookmarks > 0 {
        tracing::debug!("Max bookmarks: {}", config.max_bookmarks);
    }

    // Create MCP server components
    let reader = Arc::new(BookmarkReader::with_config(config.clone())?);
    let fetcher = Arc::new(ContentFetcher::new()?);

    // 検索マネージャーを初期化
    tracing::debug!("検索インデックスを初期化中...");
    let search_manager = ContentIndexManager::new(reader.clone(), fetcher.clone()).await?;
    let search_manager = Arc::new(search_manager);

    tracing::info!("サーバー準備完了");
    tracing::debug!("{}", search_manager.get_indexing_status());

    let server = BookmarkServer::new(reader, fetcher, search_manager);

    // Serve the MCP server
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
