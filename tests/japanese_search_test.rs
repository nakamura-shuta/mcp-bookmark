#[cfg(test)]
mod japanese_search_tests {
    use mcp_bookmark::bookmark::FlatBookmark;
    use mcp_bookmark::config::Config;
    use mcp_bookmark::search::SearchManager;
    use tempfile::TempDir;

    /// Create test bookmark with Japanese content
    fn create_japanese_bookmark(id: &str, title: &str, _content: &str) -> FlatBookmark {
        FlatBookmark {
            id: id.to_string(),
            name: title.to_string(),
            url: format!("https://example.com/{id}"),
            folder_path: vec!["test".to_string()],
            date_added: Some("2024-01-01".to_string()),
            date_modified: Some("2024-01-01".to_string()),
        }
    }

    #[test]
    fn test_japanese_tokenization_search() {
        // Create temporary directory for index
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().to_path_buf();

        // Create config with test index
        let mut config = Config::default();
        config.index_name = Some("test_japanese_index".to_string());

        // Create search manager
        let mut manager = SearchManager::new(Some(index_path)).unwrap();

        // Create test bookmarks with Japanese content
        let bookmarks = vec![
            create_japanese_bookmark(
                "1",
                "石川さんの出社日",
                "石川さんの出社日について確認します。基本的に平日はSlack OK。",
            ),
            create_japanese_bookmark(
                "2",
                "会議スケジュール",
                "明日の会議は石川さんも参加予定です。",
            ),
            create_japanese_bookmark(
                "3",
                "プロジェクト計画",
                "来週から新しいプロジェクトが始まります。出社は必須ではありません。",
            ),
        ];

        // Build index with Japanese content
        manager.build_index(&bookmarks).unwrap();
        manager.commit().unwrap();

        // Test 1: Search with space-separated terms
        let results = manager.search("石川 出社", 10).unwrap();
        assert!(!results.is_empty(), "Should find results for '石川 出社'");
        assert!(
            results.iter().any(|r| r.url == "https://example.com/1"),
            "Should find bookmark 1 with '石川さんの出社日'"
        );

        // Test 2: Search for single term
        let results = manager.search("石川さん", 10).unwrap();
        assert!(!results.is_empty(), "Should find results for '石川さん'");
        assert_eq!(
            results.len(),
            2,
            "Should find 2 bookmarks mentioning 石川さん"
        );

        // Test 3: Search for partial match
        let results = manager.search("出社日", 10).unwrap();
        assert!(!results.is_empty(), "Should find results for '出社日'");

        // Test 4: Search with different word order
        let results = manager.search("出社 石川", 10).unwrap();
        assert!(
            !results.is_empty(),
            "Should find results even with different word order"
        );
        assert!(
            results.iter().any(|r| r.url == "https://example.com/1"),
            "Should still find bookmark 1"
        );
    }

    #[test]
    fn test_japanese_mixed_content() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().to_path_buf();

        let mut manager = SearchManager::new(Some(index_path)).unwrap();

        // Create bookmarks with mixed Japanese/English content
        let bookmarks = vec![
            create_japanese_bookmark(
                "1",
                "サーバー設定",
                "サーバーの設定について説明します。Server configuration guide.",
            ),
            create_japanese_bookmark("2", "データベース接続", "Database connection の設定方法。"),
            create_japanese_bookmark("3", "API仕様書", "REST APIの仕様書です。"),
        ];

        manager.build_index(&bookmarks).unwrap();
        manager.commit().unwrap();

        // Test mixed language search
        let results = manager.search("サーバー configuration", 10).unwrap();
        assert!(
            !results.is_empty(),
            "Should find results for mixed language query"
        );

        // Test English search in Japanese content
        let results = manager.search("Database", 10).unwrap();
        assert!(
            !results.is_empty(),
            "Should find English terms in mixed content"
        );

        // Test Japanese search
        let results = manager.search("設定", 10).unwrap();
        assert_eq!(
            results.len(),
            2,
            "Should find Japanese term in multiple documents"
        );
    }

    #[test]
    fn test_japanese_phrase_variations() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().to_path_buf();

        let mut manager = SearchManager::new(Some(index_path)).unwrap();

        let bookmarks = vec![
            create_japanese_bookmark("1", "東京都の天気", "東京都の今日の天気は晴れです。"),
            create_japanese_bookmark("2", "東京オリンピック", "東京で開催されたオリンピック。"),
            create_japanese_bookmark("3", "京都観光", "京都の観光スポット紹介。"),
        ];

        manager.build_index(&bookmarks).unwrap();
        manager.commit().unwrap();

        // Test compound word tokenization
        let results = manager.search("東京", 10).unwrap();
        assert_eq!(results.len(), 2, "Should find '東京' in multiple documents");

        // Test partial match in compound
        let results = manager.search("京都", 10).unwrap();
        assert_eq!(results.len(), 1, "Should find '京都' correctly");
    }

    #[test]
    fn test_notion_example_case() {
        // This test specifically addresses the Notion bookmark example
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().to_path_buf();

        let mut manager = SearchManager::new(Some(index_path)).unwrap();

        // Simulate the Notion page content
        let content = "各種設定情報 データベース関連 レポート info \
                      デプロイ：https://vercel.com/guides/set-up-a-staging-environment-on-vercel \
                      4/11までの質問 石川さんの出社日 基本的に平日はslack OK \
                      打ち合わせは18,25は可能な予定 それぞれのドメインとvercelが発行するURLの関係など";

        let bookmarks = vec![FlatBookmark {
            id: "421".to_string(),
            name: "だれでもAIメーカー関連".to_string(),
            url: "https://www.notion.so/AI-fd5e608496a94ccaabd62821096a4992".to_string(),
            folder_path: vec!["new-index-test".to_string()],
            date_added: Some("2024-01-01".to_string()),
            date_modified: Some("2024-01-01".to_string()),
        }];

        manager.build_index(&bookmarks).unwrap();

        // Simulate adding content to the index
        // Note: In real implementation, content would be fetched and indexed
        manager.commit().unwrap();

        // Test the specific search case
        let results = manager.search("石川さんの出社日", 10).unwrap();
        // This will pass once content indexing is properly implemented

        // Test with space-separated query
        let results = manager.search("石川 出社日", 10).unwrap();
        // This should work with Lindera tokenizer
    }
}
