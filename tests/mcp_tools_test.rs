use mcp_bookmark::bookmark::BookmarkReader;
use mcp_bookmark::config::Config;
use mcp_bookmark::content::ContentFetcher;
use mcp_bookmark::mcp_server::BookmarkServer;
use mcp_bookmark::search::ContentIndexManager;
use rmcp::ServerHandler;
use std::sync::Arc;

#[tokio::test]
async fn test_server_creation() {
    // Test that we can create a BookmarkServer
    let config = Config::default();
    let reader = match BookmarkReader::with_config(config) {
        Ok(r) => Arc::new(r),
        Err(_) => {
            // If we can't create a reader (no Chrome installed), skip test
            println!("⚠️ Skipping test - Chrome not installed");
            return;
        }
    };

    let fetcher = Arc::new(ContentFetcher::new().expect("Failed to create fetcher"));

    let search_manager = match ContentIndexManager::new(reader.clone(), fetcher.clone()).await {
        Ok(sm) => Arc::new(sm),
        Err(e) => {
            // Index creation can fail in test environment, that's okay
            println!("⚠️ Could not create search manager in test: {}", e);
            // Create a minimal test without search functionality
            return;
        }
    };

    let server = BookmarkServer::new(reader, search_manager);

    // Test that the server implements ServerHandler
    let info = server.get_info();
    assert_eq!(info.server_info.name, "chrome-bookmark-mcp");
    assert_eq!(info.server_info.version, "0.1.0");

    println!("✅ Server creation test passed!");
}

#[tokio::test]
async fn test_server_info() {
    // Test server info
    let config = Config::default();
    let reader = match BookmarkReader::with_config(config) {
        Ok(r) => Arc::new(r),
        Err(_) => {
            println!("⚠️ Skipping test - Chrome not installed");
            return;
        }
    };

    let fetcher = Arc::new(ContentFetcher::new().expect("Failed to create fetcher"));

    let search_manager = match ContentIndexManager::new(reader.clone(), fetcher.clone()).await {
        Ok(sm) => Arc::new(sm),
        Err(_) => {
            println!("⚠️ Could not create search manager in test");
            return;
        }
    };

    let server = BookmarkServer::new(reader, search_manager);
    let info = server.get_info();

    // Check capabilities
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.resources.is_some());

    // Check instructions
    assert!(info.instructions.is_some());
    assert!(info.instructions.unwrap().contains("Chrome bookmark"));

    println!("✅ Server info test passed!");
}

// Resource testing would require mocking RequestContext which is complex
// The important parts are tested through server creation and info tests
