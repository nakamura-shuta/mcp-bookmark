use mcp_bookmark::bookmark::BookmarkReader;
use mcp_bookmark::config::Config;
use mcp_bookmark::content::ContentFetcher;
use mcp_bookmark::mcp_server::BookmarkServer;
use mcp_bookmark::search::HybridSearchManager;
use std::sync::Arc;

#[tokio::test]
async fn test_resource_listing() {
    // Create a test configuration
    let mut config = Config::default();
    config.include_folders = vec![vec!["Bookmarks Bar".to_string(), "test-mcp".to_string()]];

    // Create server components
    let reader = Arc::new(BookmarkReader::with_config(config).expect("Failed to create reader"));
    let fetcher = Arc::new(ContentFetcher::new().expect("Failed to create fetcher"));
    let search_manager = Arc::new(
        HybridSearchManager::new(reader.clone(), fetcher.clone())
            .await
            .expect("Failed to create search manager"),
    );
    let server = BookmarkServer::new(reader.clone(), fetcher, search_manager);

    // Test list_resources - simplified without context
    let result = server.list_resources(None, Default::default()).await;

    assert!(result.is_ok(), "list_resources should succeed");

    let resources = result.unwrap();
    assert!(
        !resources.resources.is_empty(),
        "Should have at least one resource"
    );

    // Check for bookmark://tree resource
    let has_tree = resources.resources.iter().any(|r| {
        // Access the inner RawResource fields
        let inner = r.inner();
        inner.uri == "bookmark://tree"
    });
    assert!(has_tree, "Should have bookmark://tree resource");

    println!("✅ Found {} resources", resources.resources.len());
    for res in &resources.resources[..5.min(resources.resources.len())] {
        let inner = res.inner();
        println!("  - {}: {}", inner.uri, inner.name);
    }
}

#[tokio::test]
async fn test_read_tree_resource() {
    // Create a test configuration
    let mut config = Config::default();
    config.include_folders = vec![vec!["Bookmarks Bar".to_string(), "test-mcp".to_string()]];

    // Create server components
    let reader = Arc::new(BookmarkReader::with_config(config).expect("Failed to create reader"));
    let fetcher = Arc::new(ContentFetcher::new().expect("Failed to create fetcher"));
    let search_manager = Arc::new(
        HybridSearchManager::new(reader.clone(), fetcher.clone())
            .await
            .expect("Failed to create search manager"),
    );
    let server = BookmarkServer::new(reader.clone(), fetcher, search_manager);

    // Test read_resource for bookmark://tree
    use rmcp::{ServerHandler, model::ReadResourceRequestParam};

    let request = ReadResourceRequestParam {
        uri: "bookmark://tree".to_string(),
    };

    let result = server.read_resource(request, Default::default()).await;

    assert!(result.is_ok(), "read_resource should succeed");

    let content = result.unwrap();
    assert!(!content.contents.is_empty(), "Should have content");

    // Access text content differently based on ResourceContents structure
    let first_content = &content.contents[0];

    // Get text from the content - need to check the actual structure
    let text_content = match first_content {
        rmcp::model::ResourceContents::Text { text, .. } => text.as_str(),
        _ => panic!("Expected text content"),
    };

    assert!(text_content.len() > 0, "Content should not be empty");

    // Verify it's valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(text_content).expect("Content should be valid JSON");

    assert!(parsed.get("roots").is_some(), "Should have roots field");

    println!("✅ Read bookmark://tree resource successfully");
    println!("  Content length: {} chars", text_content.len());
}
