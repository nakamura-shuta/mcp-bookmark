// Integration test for search functionality
#[cfg(test)]
mod tests {
    use mcp_bookmark::bookmark::BookmarkReader;
    use mcp_bookmark::config::Config;
    use mcp_bookmark::search::SearchManager;

    #[test]
    fn test_search_integration() {
        // Create reader with index_name set
        let mut config = Config::default();
        config.index_name = Some("test_integration_index".to_string());
        let reader = BookmarkReader::with_config(config).unwrap();

        // Get all bookmarks
        let bookmarks = reader.read_bookmarks().unwrap();
        println!("Found {} bookmarks", bookmarks.len());

        // Create search manager and index bookmarks
        let mut search_manager = SearchManager::new(None).unwrap();
        search_manager.build_index(&bookmarks).unwrap();

        // Test search
        let results = search_manager.search("", 10).unwrap();
        println!("Search returned {} results", results.len());

        assert!(results.len() <= 10);
    }
}
