#[cfg(test)]
mod lindera_integration_tests {
    use lindera::dictionary::{DictionaryKind, load_dictionary_from_kind};
    use lindera::mode::{Mode, Penalty};
    use lindera::segmenter::Segmenter;
    use lindera_tantivy::tokenizer::LinderaTokenizer;
    use mcp_bookmark::bookmark::FlatBookmark;
    use mcp_bookmark::search::SearchManager;
    use tantivy::collector::TopDocs;
    use tantivy::query::QueryParser;
    use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value};
    use tantivy::{Index, TantivyDocument, doc};
    use tempfile::TempDir;

    #[test]
    fn test_direct_lindera_indexing() {
        // Create schema with Lindera tokenizer
        let mut schema_builder = Schema::builder();

        let text_field_indexing = TextFieldIndexing::default()
            .set_tokenizer("lang_ja") // Use lang_ja as tokenizer name
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);

        let text_options = TextOptions::default()
            .set_indexing_options(text_field_indexing)
            .set_stored();

        let title = schema_builder.add_text_field("title", text_options.clone());
        let body = schema_builder.add_text_field("body", text_options);

        let schema = schema_builder.build();

        // Create index in RAM
        let index = Index::create_in_ram(schema.clone());

        // Configure and register Lindera tokenizer
        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();
        let mode = Mode::Decompose(Penalty::default());
        let segmenter = Segmenter::new(mode, dictionary, None);
        let tokenizer = LinderaTokenizer::from_segmenter(segmenter);

        // Register tokenizer with name "lang_ja"
        index.tokenizers().register("lang_ja", tokenizer);

        // Create index writer and add documents
        let mut writer = index.writer(50_000_000).unwrap();

        writer
            .add_document(doc!(
                title => "石川さんの出社日",
                body => "石川さんの出社日について確認します。基本的に平日はSlack OK。"
            ))
            .unwrap();

        writer
            .add_document(doc!(
                title => "会議スケジュール",
                body => "明日の会議は石川さんも参加予定です。"
            ))
            .unwrap();

        writer
            .add_document(doc!(
                title => "プロジェクト計画",
                body => "来週から新しいプロジェクトが始まります。出社は必須ではありません。"
            ))
            .unwrap();

        writer.commit().unwrap();

        // Create searcher
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();

        // Create query parser for both fields
        let query_parser = QueryParser::for_index(&index, vec![title, body]);

        // Test 1: Search with space-separated terms
        println!("\nTest 1: Searching for '石川 出社'");
        let query = query_parser.parse_query("石川 出社").unwrap();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

        println!("  Found {} results", top_docs.len());
        for (_score, doc_address) in &top_docs {
            let doc: TantivyDocument = searcher.doc(*doc_address).unwrap();
            if let Some(title_value) = doc.get_first(title) {
                println!("  - {}", title_value.as_str().unwrap_or(""));
            }
        }
        assert!(!top_docs.is_empty(), "Should find results for '石川 出社'");

        // Test 2: Search for "石川さん"
        println!("\nTest 2: Searching for '石川さん'");
        let query = query_parser.parse_query("石川さん").unwrap();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

        println!("  Found {} results", top_docs.len());
        assert_eq!(top_docs.len(), 2, "Should find 2 results for '石川さん'");

        // Test 3: Search for "出社日"
        println!("\nTest 3: Searching for '出社日'");
        let query = query_parser.parse_query("出社日").unwrap();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

        println!("  Found {} results", top_docs.len());
        assert!(!top_docs.is_empty(), "Should find results for '出社日'");
    }

    #[test]
    fn test_search_manager_with_content() {
        // Create temporary directory for index
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().to_path_buf();

        // Create search manager
        let mut manager = SearchManager::new(Some(index_path)).unwrap();

        // Create test bookmarks - note that content is in the title field
        let bookmarks = vec![
            FlatBookmark {
                id: "1".to_string(),
                name: "石川さんの出社日について確認します。基本的に平日はSlack OK。".to_string(),
                url: "https://example.com/1".to_string(),
                folder_path: vec!["test".to_string()],
                date_added: Some("2024-01-01".to_string()),
                date_modified: Some("2024-01-01".to_string()),
            },
            FlatBookmark {
                id: "2".to_string(),
                name: "明日の会議は石川さんも参加予定です。".to_string(),
                url: "https://example.com/2".to_string(),
                folder_path: vec!["test".to_string()],
                date_added: Some("2024-01-01".to_string()),
                date_modified: Some("2024-01-01".to_string()),
            },
            FlatBookmark {
                id: "3".to_string(),
                name: "来週から新しいプロジェクトが始まります。出社は必須ではありません。"
                    .to_string(),
                url: "https://example.com/3".to_string(),
                folder_path: vec!["test".to_string()],
                date_added: Some("2024-01-01".to_string()),
                date_modified: Some("2024-01-01".to_string()),
            },
        ];

        // Build index
        manager.build_index(&bookmarks).unwrap();
        manager.commit().unwrap();

        // Test search
        let results = manager.search("石川 出社", 10).unwrap();
        println!("Search for '石川 出社': found {} results", results.len());
        for result in &results {
            println!("  - {}: {}", result.url, result.title);
        }
        assert!(!results.is_empty(), "Should find results for '石川 出社'");

        let results = manager.search("石川さん", 10).unwrap();
        println!("Search for '石川さん': found {} results", results.len());
        assert_eq!(results.len(), 2, "Should find 2 results for '石川さん'");
    }
}
