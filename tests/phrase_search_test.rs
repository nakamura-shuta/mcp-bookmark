use anyhow::Result;
use mcp_bookmark::bookmark::FlatBookmark;
use mcp_bookmark::search::search_manager::SearchManager;
use tempfile::TempDir;

/// Test basic phrase search functionality
#[test]
fn test_basic_phrase_search() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("test_index");

    // Create test bookmarks
    let bookmarks = vec![
        FlatBookmark {
            id: "1".to_string(),
            name: "React hooks documentation".to_string(),
            url: "https://example.com/react-hooks".to_string(),
            folder_path: vec!["Development".to_string(), "React".to_string()],
            date_added: None,
            date_modified: None,
        },
        FlatBookmark {
            id: "2".to_string(),
            name: "Vue composition API".to_string(),
            url: "https://example.com/vue".to_string(),
            folder_path: vec!["Development".to_string(), "Vue".to_string()],
            date_added: None,
            date_modified: None,
        },
        FlatBookmark {
            id: "3".to_string(),
            name: "Custom React and hooks tutorial".to_string(),
            url: "https://example.com/tutorial".to_string(),
            folder_path: vec!["Development".to_string(), "Tutorials".to_string()],
            date_added: None,
            date_modified: None,
        },
    ];

    // Index content
    let content_map = [
        (
            "https://example.com/react-hooks",
            "Learn about React hooks and how to use them in functional components. React hooks provide state management.",
        ),
        (
            "https://example.com/vue",
            "React is mentioned here. Also, hooks are discussed separately.",
        ),
        (
            "https://example.com/tutorial",
            "This tutorial covers React. It also covers hooks. But not together.",
        ),
    ];

    // Create and populate search manager
    let mut manager = SearchManager::new_for_testing(index_path)?;

    for (bookmark, (url, content)) in bookmarks.iter().zip(content_map.iter()) {
        assert_eq!(&bookmark.url, url);
        manager.index_bookmark_with_content(bookmark, Some(content))?;
    }
    manager.commit()?;

    // Test 1: Exact phrase search
    let results = manager.search("\"React hooks\"", 10)?;
    assert_eq!(
        results.len(),
        1,
        "Phrase search should find only exact matches"
    );
    assert_eq!(results[0].url, "https://example.com/react-hooks");

    // Test 2: Words without quotes (should find all documents with either word)
    let results = manager.search("React hooks", 10)?;
    assert!(
        results.len() >= 2,
        "Word search should find documents with either word"
    );

    // Test 3: Mixed phrase and word search
    let results = manager.search("\"React hooks\" documentation", 10)?;
    assert_eq!(
        results.len(),
        1,
        "Should find document with phrase and word"
    );
    assert_eq!(results[0].url, "https://example.com/react-hooks");

    Ok(())
}

/// Test phrase search with special characters
#[test]
fn test_phrase_search_with_special_chars() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("test_index");

    let bookmarks = vec![
        FlatBookmark {
            id: "1".to_string(),
            name: "Error handling".to_string(),
            url: "https://example.com/errors".to_string(),
            folder_path: vec!["Development".to_string()],
            date_added: None,
            date_modified: None,
        },
        FlatBookmark {
            id: "2".to_string(),
            name: "JavaScript errors".to_string(),
            url: "https://example.com/js-errors".to_string(),
            folder_path: vec!["Development".to_string()],
            date_added: None,
            date_modified: None,
        },
    ];

    let content_map = [
        (
            "https://example.com/errors",
            "Common error: Cannot read property 'undefined' of null. This is a typical JavaScript error.",
        ),
        (
            "https://example.com/js-errors",
            "Cannot read property is common. Also undefined of null happens. But not the exact phrase.",
        ),
    ];

    let mut manager = SearchManager::new_for_testing(index_path)?;

    for (bookmark, (url, content)) in bookmarks.iter().zip(content_map.iter()) {
        assert_eq!(&bookmark.url, url);
        manager.index_bookmark_with_content(bookmark, Some(content))?;
    }
    manager.commit()?;

    // Search for error message as phrase
    let results = manager.search("\"Cannot read property 'undefined' of null\"", 10)?;
    assert_eq!(results.len(), 1, "Should find exact error message");
    assert_eq!(results[0].url, "https://example.com/errors");

    Ok(())
}

/// Test Japanese phrase search
#[test]
fn test_japanese_phrase_search() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("test_index");

    let bookmarks = vec![
        FlatBookmark {
            id: "1".to_string(),
            name: "React フック入門".to_string(),
            url: "https://example.com/react-hooks-ja".to_string(),
            folder_path: vec!["Development".to_string(), "日本語".to_string()],
            date_added: None,
            date_modified: None,
        },
        FlatBookmark {
            id: "2".to_string(),
            name: "JavaScript基礎".to_string(),
            url: "https://example.com/js-basics-ja".to_string(),
            folder_path: vec!["Development".to_string(), "日本語".to_string()],
            date_added: None,
            date_modified: None,
        },
    ];

    let content_map = [
        (
            "https://example.com/react-hooks-ja",
            "React フックを使用して状態管理を行います。React フックは関数コンポーネントで使用できます。",
        ),
        (
            "https://example.com/js-basics-ja",
            "Reactは素晴らしいライブラリです。フックも便利な機能です。しかしReact フックという連続した言葉はありません。",
        ),
    ];

    let mut manager = SearchManager::new_for_testing(index_path)?;

    for (bookmark, (url, content)) in bookmarks.iter().zip(content_map.iter()) {
        assert_eq!(&bookmark.url, url);
        manager.index_bookmark_with_content(bookmark, Some(content))?;
    }
    manager.commit()?;

    // Search for Japanese phrase
    let results = manager.search("\"React フック\"", 10)?;
    assert!(!results.is_empty(), "Should find Japanese phrase");
    // The first result should be the document with the exact phrase
    assert!(results[0].url.contains("react-hooks-ja"));

    Ok(())
}

/// Test empty phrase handling
#[test]
fn test_empty_phrase_search() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("test_index");

    let bookmarks = [FlatBookmark {
        id: "1".to_string(),
        name: "Test document".to_string(),
        url: "https://example.com/test".to_string(),
        folder_path: vec!["Test".to_string()],
        date_added: None,
        date_modified: None,
    }];

    let mut manager = SearchManager::new_for_testing(index_path)?;
    manager.index_bookmark_with_content(&bookmarks[0], Some("Test content"))?;
    manager.commit()?;

    // Test empty quotes
    let results = manager.search("\"\"", 10)?;
    assert_eq!(results.len(), 0, "Empty phrase should return no results");

    // Test quotes with only whitespace
    let results = manager.search("\"   \"", 10)?;
    assert_eq!(
        results.len(),
        0,
        "Whitespace-only phrase should return no results"
    );

    Ok(())
}

/// Test unclosed phrase handling
#[test]
fn test_unclosed_phrase_search() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("test_index");

    let bookmarks = [FlatBookmark {
        id: "1".to_string(),
        name: "React hooks useState".to_string(),
        url: "https://example.com/test".to_string(),
        folder_path: vec!["Test".to_string()],
        date_added: None,
        date_modified: None,
    }];

    let mut manager = SearchManager::new_for_testing(index_path)?;
    manager.index_bookmark_with_content(
        &bookmarks[0],
        Some("Learn about React hooks useState and useEffect"),
    )?;
    manager.commit()?;

    // Unclosed quote should be treated as a phrase from quote to end
    let results = manager.search("\"React hooks useState", 10)?;
    assert!(!results.is_empty(), "Unclosed phrase should still search");
    assert_eq!(results[0].url, "https://example.com/test");

    Ok(())
}

/// Test multiple phrases in one query
#[test]
fn test_multiple_phrases_search() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("test_index");

    let bookmarks = vec![
        FlatBookmark {
            id: "1".to_string(),
            name: "React Server Components and Client Components".to_string(),
            url: "https://example.com/rsc".to_string(),
            folder_path: vec!["Development".to_string()],
            date_added: None,
            date_modified: None,
        },
        FlatBookmark {
            id: "2".to_string(),
            name: "Next.js documentation".to_string(),
            url: "https://example.com/nextjs".to_string(),
            folder_path: vec!["Development".to_string()],
            date_added: None,
            date_modified: None,
        },
    ];

    let content_map = [
        (
            "https://example.com/rsc",
            "React Server Components allow server-side rendering. Client Components handle interactivity.",
        ),
        (
            "https://example.com/nextjs",
            "Server Components are great. React and Client Components work together. But not the exact phrases.",
        ),
    ];

    let mut manager = SearchManager::new_for_testing(index_path)?;

    for (bookmark, (url, content)) in bookmarks.iter().zip(content_map.iter()) {
        assert_eq!(&bookmark.url, url);
        manager.index_bookmark_with_content(bookmark, Some(content))?;
    }
    manager.commit()?;

    // Search for two phrases
    let results = manager.search("\"React Server Components\" \"Client Components\"", 10)?;
    assert_eq!(results.len(), 1, "Should find document with both phrases");
    assert_eq!(results[0].url, "https://example.com/rsc");

    Ok(())
}
