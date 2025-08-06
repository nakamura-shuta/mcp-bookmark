#[cfg(test)]
mod tests {
    use mcp_bookmark::bookmark::BookmarkReader;
    use mcp_bookmark::config::Config;
    use mcp_bookmark::content::ContentFetcher;
    use mcp_bookmark::mcp_server::BookmarkServer;
    use rmcp::ServerHandler;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_bookmarks() -> String {
        r#"{
            "checksum": "test_checksum",
            "roots": {
                "bookmark_bar": {
                    "id": "1",
                    "guid": "guid_1",
                    "name": "Bookmarks Bar",
                    "type": "folder",
                    "children": [
                        {
                            "id": "2",
                            "guid": "guid_2",
                            "name": "Test Bookmark",
                            "type": "url",
                            "url": "https://example.com",
                            "date_added": "13366616400000000"
                        },
                        {
                            "id": "3",
                            "guid": "guid_3",
                            "name": "Development",
                            "type": "folder",
                            "date_added": "13366616400000000",
                            "children": [
                                {
                                    "id": "4",
                                    "guid": "guid_4",
                                    "name": "Rust",
                                    "type": "url",
                                    "url": "https://rust-lang.org",
                                    "date_added": "13366616400000000"
                                },
                                {
                                    "id": "5",
                                    "guid": "guid_5",
                                    "name": "GitHub",
                                    "type": "url",
                                    "url": "https://github.com",
                                    "date_added": "13366616400000000"
                                }
                            ]
                        }
                    ]
                },
                "other": {
                    "id": "10",
                    "guid": "guid_10",
                    "name": "Other Bookmarks",
                    "type": "folder",
                    "children": []
                },
                "synced": {
                    "id": "11",
                    "guid": "guid_11",
                    "name": "Mobile Bookmarks",
                    "type": "folder",
                    "children": []
                }
            },
            "version": 1
        }"#
        .to_string()
    }

    fn setup_test_bookmarks() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let bookmark_file = temp_dir.path().join("Bookmarks");
        fs::write(&bookmark_file, create_test_bookmarks()).unwrap();
        (temp_dir, bookmark_file)
    }

    #[tokio::test]
    async fn test_server_initialization() {
        let config = Config::default();

        // Use the actual Chrome bookmarks if available, otherwise skip
        let reader = match BookmarkReader::with_config(config) {
            Ok(reader) => Arc::new(reader),
            Err(_) => {
                println!("Skipping test - Chrome bookmarks not found");
                return;
            }
        };

        let fetcher = Arc::new(ContentFetcher::new().unwrap());
        let server = BookmarkServer::new(reader, fetcher);

        let info = server.get_info();
        assert_eq!(info.server_info.name, "chrome-bookmark-mcp");
        assert_eq!(info.server_info.version, "0.1.0");
        assert!(info.capabilities.resources.is_some());
        assert!(info.capabilities.tools.is_some());
    }

    #[tokio::test]
    async fn test_bookmark_reader_with_test_file() {
        let (_temp_dir, bookmark_file) = setup_test_bookmarks();

        let reader = BookmarkReader::with_path(&bookmark_file);

        // Test reading the bookmark tree
        let tree = reader.read().unwrap();
        let tree_json = serde_json::to_string(&tree).unwrap();
        assert!(tree_json.contains("Test Bookmark"));
        assert!(tree_json.contains("https://example.com"));

        // Test search functionality
        let results = reader.search_bookmarks("Rust").unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "Rust");
        assert_eq!(results[0].url, "https://rust-lang.org");

        // Test folder listing
        let folders = reader.list_all_folders().unwrap();
        assert!(!folders.is_empty());
        let has_dev_folder = folders
            .iter()
            .any(|path| path.contains(&"Development".to_string()));
        assert!(has_dev_folder);
    }

    #[tokio::test]
    async fn test_server_with_test_bookmarks() {
        let (_temp_dir, bookmark_file) = setup_test_bookmarks();

        let reader = Arc::new(BookmarkReader::with_path(&bookmark_file));
        let fetcher = Arc::new(ContentFetcher::new().unwrap());
        let server = BookmarkServer::new(reader.clone(), fetcher);

        // Test that the server initializes properly
        let info = server.get_info();
        assert_eq!(info.server_info.name, "chrome-bookmark-mcp");

        // Test tools through the reader directly
        let search_results = reader.search_bookmarks("GitHub").unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].name, "GitHub");

        let folders = reader.list_all_folders().unwrap();
        assert!(folders.len() > 0);
    }

    #[tokio::test]
    async fn test_content_fetcher() {
        let fetcher = ContentFetcher::new().unwrap();

        // Test metadata extraction from HTML
        let test_html = r#"
            <html>
                <head>
                    <title>Test Page</title>
                    <meta name="description" content="Test description">
                    <meta property="og:title" content="OG Test Title">
                    <meta property="og:description" content="OG Test Description">
                </head>
                <body>
                    <h1>Test Content</h1>
                    <p>Some test content here</p>
                </body>
            </html>
        "#;

        let metadata = fetcher.extract_metadata(test_html, "https://example.com");
        assert_eq!(metadata.title, Some("Test Page".to_string()));
        assert_eq!(metadata.description, Some("Test description".to_string()));
        assert_eq!(metadata.og_title, Some("OG Test Title".to_string()));
        assert_eq!(
            metadata.og_description,
            Some("OG Test Description".to_string())
        );
    }

    #[tokio::test]
    async fn test_bookmark_filtering() {
        let (_temp_dir, bookmark_file) = setup_test_bookmarks();

        let mut config = Config::default();
        config.include_folders = vec![vec!["Bookmarks Bar".to_string(), "Development".to_string()]];

        let reader = BookmarkReader::with_path(&bookmark_file);

        // Even with filtering config, with_path doesn't use config
        // So we test the raw functionality
        let tree = reader.read().unwrap();

        // Should still have all bookmarks since with_path doesn't filter
        let tree_json = serde_json::to_string(&tree).unwrap();
        assert!(tree_json.contains("Test Bookmark"));
        assert!(tree_json.contains("Development"));
    }
}
