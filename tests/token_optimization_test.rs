use mcp_bookmark::search::schema::BookmarkSchema;
use mcp_bookmark::search::unified_searcher::{SearchResult, UnifiedSearcher};
use tantivy::{Index, doc};
use tempfile::TempDir;

/// Test that SearchResult no longer has duplicate content field
#[test]
fn test_search_result_no_duplicate_content() {
    let result = SearchResult {
        id: "test-id".to_string(),
        title: "Test Title".to_string(),
        url: "https://example.com".to_string(),
        snippet: "This is a test snippet".to_string(),
        full_content: None,
        score: 0.95,
        folder_path: "/test/path".to_string(),
        last_indexed: None,
        context_type: Some("Content".to_string()),
    };

    // Serialize to JSON to verify structure
    let json = serde_json::to_value(&result).unwrap();

    // Verify that snippet field exists
    assert!(json.get("snippet").is_some());
    assert_eq!(json.get("snippet").unwrap(), "This is a test snippet");

    // Verify that content field does NOT exist
    assert!(
        json.get("content").is_none(),
        "content field should not exist in SearchResult"
    );

    // Verify other fields are still present
    assert!(json.get("id").is_some());
    assert!(json.get("title").is_some());
    assert!(json.get("url").is_some());
    assert!(json.get("score").is_some());
    assert!(json.get("folder_path").is_some());
}

/// Test that search results only contain snippet, not duplicate content
#[test]
fn test_search_returns_only_snippet() {
    let temp_dir = TempDir::new().unwrap();
    let schema = BookmarkSchema::new();
    let index = Index::create_in_dir(temp_dir.path(), schema.schema.clone()).unwrap();

    // Register tokenizer
    mcp_bookmark::search::tokenizer::register_lindera_tokenizer(&index).unwrap();

    // Index a test document
    let mut index_writer = index.writer(50_000_000).unwrap();
    let long_content = "This is a very long content that would normally be duplicated in both content and snippet fields. \
                       It contains important information about React hooks and how to use them effectively in modern web development. \
                       The content continues with more details about useState, useEffect, and custom hooks. \
                       This extensive text would consume unnecessary tokens if duplicated.";

    index_writer
        .add_document(doc!(
            schema.id => "1",
            schema.title => "React Hooks Guide",
            schema.url => "https://example.com/react-hooks",
            schema.content => long_content,
            schema.folder_path => "development/react"
        ))
        .unwrap();

    index_writer.commit().unwrap();

    // Create searcher and perform search
    let searcher = UnifiedSearcher::new(index, schema).unwrap();
    let results = searcher.search("React hooks", 10).unwrap();

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Verify snippet is present and not too long
    assert!(!result.snippet.is_empty());
    assert!(result.snippet.len() <= 400); // Should be truncated to around 300 chars + ellipsis

    // Serialize to JSON to absolutely verify no content field
    let json = serde_json::to_value(result).unwrap();
    assert!(
        json.get("content").is_none(),
        "SearchResult should not have a content field"
    );
    assert!(
        json.get("snippet").is_some(),
        "SearchResult should have a snippet field"
    );
}

/// Test memory efficiency - verify that we're not storing duplicate data
#[test]
fn test_memory_efficiency() {
    let snippet_text = "This is a snippet that represents the content preview";

    let result = SearchResult {
        id: "mem-test".to_string(),
        title: "Memory Test".to_string(),
        url: "https://example.com/memory".to_string(),
        snippet: snippet_text.to_string(),
        full_content: None,
        score: 0.85,
        folder_path: "/test".to_string(),
        last_indexed: None,
        context_type: None,
    };

    // Calculate approximate memory usage
    let json_str = serde_json::to_string(&result).unwrap();
    let json_size = json_str.len();

    // With the old structure, we would have both 'content' and 'snippet' with the same value
    // The size should now be smaller by approximately the length of the snippet text

    // Verify the JSON doesn't contain duplicate text
    let occurrences = json_str.matches(snippet_text).count();
    assert_eq!(
        occurrences, 1,
        "Snippet text should appear only once in serialized result"
    );

    println!("Serialized SearchResult size: {json_size} bytes");
    println!("Snippet length: {} chars", snippet_text.len());
}
