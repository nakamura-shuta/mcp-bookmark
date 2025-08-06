mod bookmark;
mod config;
mod content;
mod mcp_server;

use anyhow::Result;
use bookmark::BookmarkReader;
use config::Config;
use content::ContentFetcher;
use mcp_server::BookmarkServer;
use rmcp::{ServiceExt, transport::stdio};
use std::env;
use std::sync::Arc;
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // コマンドライン引数から設定を作成
    let args: Vec<String> = env::args().collect();
    let mut config = Config::default();

    // シンプルな引数パース
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];

        // ヘルプ
        if arg == "--help" || arg == "-h" {
            println!("Chrome Bookmark MCP Server\n");
            println!("Usage: mcp-bookmark [folder] [max_count]\n");
            println!("Examples:");
            println!("  mcp-bookmark                    # 全ブックマーク");
            println!("  mcp-bookmark Development         # Developmentフォルダのみ");
            println!("  mcp-bookmark Development 10      # Developmentフォルダの最大10件");
            println!("  mcp-bookmark Work,Tech 20        # WorkとTechフォルダの最大20件\n");
            println!("Advanced options:");
            println!("  --exclude <folders>  除外するフォルダ");
            return Ok(());
        }

        // --excludeオプション
        if arg == "--exclude" && i + 1 < args.len() {
            let folders = parse_folder_argument(&args[i + 1]);
            config.exclude_folders = folders;
            i += 2;
            continue;
        }

        // 数字なら最大数として扱う
        if let Ok(max) = arg.parse::<usize>() {
            config.max_bookmarks = max;
            i += 1;
            continue;
        }

        // それ以外はフォルダ名として扱う（最初の引数のみ）
        if i == 1 && !arg.starts_with("-") {
            config.include_folders = parse_folder_argument(arg);
        }

        i += 1;
    }

    // フォルダ引数をパースする補助関数
    fn parse_folder_argument(arg: &str) -> Vec<Vec<String>> {
        arg.split(',')
            .map(|folder_name| {
                if !folder_name.contains('/') {
                    // シンプルなフォルダ名は Bookmarks Bar/ブックマーク バー 配下として扱う（日本語環境対応）
                    vec!["Bookmarks Bar".to_string(), "ブックマーク バー".to_string(), folder_name.to_string()]
                } else {
                    folder_name.split('/').map(String::from).collect()
                }
            })
            .collect()
    }

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
    let reader = Arc::new(BookmarkReader::with_config(config)?);
    let fetcher = Arc::new(ContentFetcher::new()?);
    let server = BookmarkServer::new(reader, fetcher);

    // Serve the MCP server
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
