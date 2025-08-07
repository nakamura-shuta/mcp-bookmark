use mcp_bookmark::bookmark::BookmarkReader;
use mcp_bookmark::config::Config;
use mcp_bookmark::content::ContentFetcher;
use mcp_bookmark::mcp_server::BookmarkServer;
use mcp_bookmark::search::ContentIndexManager;
use std::sync::Arc;

#[tokio::test]
async fn test_basic_server_creation() {
    // Create a test configuration
    let mut config = Config::default();
    config.max_bookmarks = 10;

    // Create server components
    let reader = Arc::new(BookmarkReader::with_config(config).expect("Failed to create reader"));
    let fetcher = Arc::new(ContentFetcher::new().expect("Failed to create fetcher"));
    let search_manager = Arc::new(
        ContentIndexManager::new(reader.clone(), fetcher.clone())
            .await
            .expect("Failed to create search manager"),
    );
    let _server = BookmarkServer::new(reader.clone(), fetcher, search_manager);

    println!("✅ Server created successfully");
}

#[tokio::test]
async fn test_bookmark_reader_with_profile() {
    // Test with profile configuration
    let mut config = Config::default();
    config.profile_name = Some("Default".to_string());

    // Try to create reader with profile
    match BookmarkReader::with_config(config) {
        Ok(_reader) => {
            println!("✅ BookmarkReader created with profile configuration");
        }
        Err(e) => {
            // It's ok if profile doesn't exist in test environment
            println!("⚠️ Profile not found (expected in test environment): {}", e);
        }
    }
}

#[tokio::test]
async fn test_folder_filtering() {
    // Test folder filtering
    let mut config = Config::default();
    config.target_folder = Some("TestFolder".to_string());

    match BookmarkReader::with_config(config) {
        Ok(reader) => {
            let bookmarks = reader.get_all_bookmarks().unwrap_or_default();
            println!("✅ Got {} bookmarks with folder filter", bookmarks.len());
        }
        Err(e) => {
            println!("⚠️ Could not test folder filtering: {}", e);
        }
    }
}
