#[cfg(test)]
mod simple_tests {
    use lindera::dictionary::{DictionaryKind, load_dictionary_from_kind};
    use lindera::mode::{Mode, Penalty};
    use lindera::segmenter::Segmenter;
    use lindera_tantivy::tokenizer::LinderaTokenizer;
    use tantivy::collector::TopDocs;
    use tantivy::query::QueryParser;
    use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
    use tantivy::{Index, doc};

    #[test]
    fn test_lindera_basic() {
        // Create schema with Lindera tokenizer
        let mut schema_builder = Schema::builder();

        let text_field_indexing = TextFieldIndexing::default()
            .set_tokenizer("lindera")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);

        let text_options = TextOptions::default()
            .set_indexing_options(text_field_indexing)
            .set_stored();

        let title = schema_builder.add_text_field("title", text_options.clone());
        let content = schema_builder.add_text_field("content", text_options);

        let schema = schema_builder.build();

        // Create index in RAM
        let index = Index::create_in_ram(schema);

        // Register Lindera tokenizer BEFORE indexing
        println!("Registering Lindera tokenizer...");
        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();
        let mode = Mode::Decompose(Penalty::default());
        let segmenter = Segmenter::new(mode, dictionary, None);
        let tokenizer = LinderaTokenizer::from_segmenter(segmenter);
        index.tokenizers().register("lindera", tokenizer);
        println!("Lindera tokenizer registered!");

        // Create writer and add documents
        let mut writer = index.writer(50_000_000).unwrap();

        writer
            .add_document(doc!(
                title => "田中さんの出社日",
                content => "田中さんの出社日について確認します。基本的に平日はSlack OK。"
            ))
            .unwrap();

        writer
            .add_document(doc!(
                title => "会議スケジュール",
                content => "明日の会議は田中さんも参加予定です。"
            ))
            .unwrap();

        writer.commit().unwrap();

        // Search
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&index, vec![title, content]);

        // Test: Should find documents with "田中 出社"
        let query = query_parser.parse_query("田中 出社").unwrap();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

        println!("Search for '田中 出社': found {} results", top_docs.len());
        assert!(!top_docs.is_empty(), "Should find results for '田中 出社'");

        // Test: Should find documents with "田中さん"
        let query = query_parser.parse_query("田中さん").unwrap();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

        println!("Search for '田中さん': found {} results", top_docs.len());
        assert_eq!(top_docs.len(), 2, "Should find 2 results for '田中さん'");
    }
}
