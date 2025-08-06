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
use search::HybridSearchManager;
use rmcp::{ServiceExt, transport::stdio};
use std::env;
use std::sync::Arc;
use tracing_subscriber::{self, EnvFilter};

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

    Ok(config)
}

/// Print help message
fn print_help() {
    println!("Chrome Bookmark MCP Server\n");
    println!("Usage: mcp-bookmark [folder] [max_count]\n");
    println!("Examples:");
    println!("  mcp-bookmark                    # All bookmarks");
    println!("  mcp-bookmark Development         # Only Development folder");
    println!("  mcp-bookmark Development 10      # Max 10 bookmarks from Development");
    println!("  mcp-bookmark Work,Tech 20        # Max 20 bookmarks from Work and Tech\n");
    println!("Advanced options:");
    println!("  --exclude <folders>  Exclude specified folders");
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
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // Parse command-line arguments
    let config = parse_args()?;

    tracing::info!("Starting Chrome Bookmark MCP Server");
    if !config.include_folders.is_empty() {
        tracing::info!("Including folders: {:?}", config.include_folders);
    }
    if !config.exclude_folders.is_empty() {
        tracing::info!("Excluding folders: {:?}", config.exclude_folders);
    }
    if config.max_bookmarks > 0 {
        tracing::info!("Max bookmarks: {}", config.max_bookmarks);
    }

    // Create MCP server components
    let reader = Arc::new(BookmarkReader::with_config(config.clone())?);
    let fetcher = Arc::new(ContentFetcher::new()?);
    
    // ハイブリッド検索マネージャーを初期化
    tracing::info!("初期化中...");
    let search_manager = HybridSearchManager::new(reader.clone(), fetcher.clone()).await?;
    let search_manager = Arc::new(search_manager);
    
    tracing::info!("✅ サーバー準備完了！検索可能です");
    tracing::info!("{}", search_manager.get_indexing_status());
    
    let server = BookmarkServer::new(reader, fetcher, search_manager);

    // Serve the MCP server
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
