use mcp_bookmark::batch_manager::BatchIndexManager;
use mcp_bookmark::bookmark::FlatBookmark;
use mcp_bookmark::search::SearchManager;
use tempfile::TempDir;

#[tokio::test]
async fn test_empty_batch_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
    let manager = BatchIndexManager::new(search_manager);

    // Empty batch should be rejected
    let result = manager.start_batch("empty_batch".to_string(), 0).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty batch"));
}

#[tokio::test]
async fn test_single_bookmark_batch() {
    let temp_dir = TempDir::new().unwrap();
    let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
    let manager = BatchIndexManager::new(search_manager);

    // Start batch with 1 bookmark
    manager.start_batch("single".to_string(), 1).await.unwrap();

    // Add bookmark
    let bookmark = FlatBookmark {
        id: "1".to_string(),
        name: "Test Site".to_string(),
        url: "https://example.com".to_string(),
        date_added: Some("1234567890".to_string()),
        date_modified: None,
        folder_path: vec!["Test".to_string()],
    };

    manager
        .add_to_batch(
            "single".to_string(),
            0,
            bookmark,
            "Test content".to_string(),
        )
        .await
        .unwrap();

    // End batch
    let result = manager.end_batch("single".to_string()).await.unwrap();
    assert_eq!(result.success_count, 1);
    assert_eq!(result.failed_count, 0);
}

#[tokio::test]
async fn test_two_bookmarks_immediate_commit() {
    let temp_dir = TempDir::new().unwrap();
    let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
    let manager = BatchIndexManager::new(search_manager);

    // Batch with 2 bookmarks should have immediate_commit = true
    manager.start_batch("two".to_string(), 2).await.unwrap();

    for i in 0..2 {
        let bookmark = FlatBookmark {
            id: format!("id_{i}"),
            name: format!("Site {i}"),
            url: format!("https://example.com/{i}"),
            date_added: Some("1234567890".to_string()),
            date_modified: None,
            folder_path: vec!["Test".to_string()],
        };

        manager
            .add_to_batch("two".to_string(), i, bookmark, format!("Content {i}"))
            .await
            .unwrap();
    }

    let result = manager.end_batch("two".to_string()).await.unwrap();
    assert_eq!(result.success_count, 2);
    assert_eq!(result.failed_count, 0);
}

#[tokio::test]
async fn test_parallel_batch_processing() {
    let temp_dir = TempDir::new().unwrap();
    let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
    let manager = BatchIndexManager::new(search_manager);

    // Start batch with 10 bookmarks
    manager
        .start_batch("parallel".to_string(), 10)
        .await
        .unwrap();

    // Add bookmarks sequentially (can't share manager reference across threads)
    for i in 0..10 {
        let bookmark = FlatBookmark {
            id: format!("id_{i}"),
            name: format!("Site {i}"),
            url: format!("https://example.com/{i}"),
            date_added: Some("1234567890".to_string()),
            date_modified: None,
            folder_path: vec!["Parallel".to_string()],
        };

        manager
            .add_to_batch("parallel".to_string(), i, bookmark, format!("Content {i}"))
            .await
            .unwrap();
    }

    // End batch
    let result = manager.end_batch("parallel".to_string()).await.unwrap();
    assert_eq!(result.success_count, 10);
    assert_eq!(result.failed_count, 0);
}

#[tokio::test]
async fn test_duplicate_index_prevention() {
    let temp_dir = TempDir::new().unwrap();
    let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
    let manager = BatchIndexManager::new(search_manager);

    manager
        .start_batch("dup_test".to_string(), 3)
        .await
        .unwrap();

    let bookmark = FlatBookmark {
        id: "1".to_string(),
        name: "Test".to_string(),
        url: "https://test.com".to_string(),
        date_added: None,
        date_modified: None,
        folder_path: vec![],
    };

    // Add index 0 twice
    manager
        .add_to_batch(
            "dup_test".to_string(),
            0,
            bookmark.clone(),
            "Content1".to_string(),
        )
        .await
        .unwrap();

    // This should succeed but not add duplicate
    manager
        .add_to_batch(
            "dup_test".to_string(),
            0,
            bookmark.clone(),
            "Content2".to_string(),
        )
        .await
        .unwrap();

    // Add different index
    manager
        .add_to_batch("dup_test".to_string(), 1, bookmark, "Content3".to_string())
        .await
        .unwrap();

    let status = manager.get_batch_status("dup_test").await.unwrap();
    assert_eq!(status.0, 2); // Only 2 unique indices
    assert_eq!(status.1, 3); // Total of 3
}

#[tokio::test]
async fn test_batch_not_found_error() {
    let temp_dir = TempDir::new().unwrap();
    let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
    let manager = BatchIndexManager::new(search_manager);

    let bookmark = FlatBookmark {
        id: "1".to_string(),
        name: "Test".to_string(),
        url: "https://test.com".to_string(),
        date_added: None,
        date_modified: None,
        folder_path: vec![],
    };

    // Try to add to non-existent batch
    let result = manager
        .add_to_batch(
            "nonexistent".to_string(),
            0,
            bookmark,
            "Content".to_string(),
        )
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_auto_commit_at_buffer_size() {
    let temp_dir = TempDir::new().unwrap();
    let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
    let manager = BatchIndexManager::new(search_manager);
    // Note: max_buffer_size is set to 50 by default, which is fine for this test

    manager
        .start_batch("buffer_test".to_string(), 100)
        .await
        .unwrap();

    // Add 50 bookmarks - should trigger auto-commit at buffer size
    for i in 0..50 {
        let bookmark = FlatBookmark {
            id: format!("id_{i}"),
            name: format!("Site {i}"),
            url: format!("https://example.com/{i}"),
            date_added: None,
            date_modified: None,
            folder_path: vec![],
        };

        manager
            .add_to_batch(
                "buffer_test".to_string(),
                i,
                bookmark,
                format!("Content {i}"),
            )
            .await
            .unwrap();
    }

    // The buffer should have been cleared after commit
    let status = manager.get_batch_status("buffer_test").await.unwrap();
    assert_eq!(status.0, 50); // 50 received

    // Add more bookmarks
    for i in 50..100 {
        let bookmark = FlatBookmark {
            id: format!("id_{i}"),
            name: format!("Site {i}"),
            url: format!("https://example.com/{i}"),
            date_added: None,
            date_modified: None,
            folder_path: vec![],
        };

        manager
            .add_to_batch(
                "buffer_test".to_string(),
                i,
                bookmark,
                format!("Content {i}"),
            )
            .await
            .unwrap();
    }

    let result = manager.end_batch("buffer_test".to_string()).await.unwrap();
    assert_eq!(result.success_count, 100);
}

#[tokio::test]
async fn test_concurrent_batches() {
    let temp_dir = TempDir::new().unwrap();
    let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
    let manager = BatchIndexManager::new(search_manager);

    // Start multiple batches
    manager.start_batch("batch1".to_string(), 2).await.unwrap();
    manager.start_batch("batch2".to_string(), 3).await.unwrap();

    // Add to different batches
    let bookmark1 = FlatBookmark {
        id: "1".to_string(),
        name: "Site 1".to_string(),
        url: "https://site1.com".to_string(),
        date_added: None,
        date_modified: None,
        folder_path: vec!["Batch1".to_string()],
    };

    let bookmark2 = FlatBookmark {
        id: "2".to_string(),
        name: "Site 2".to_string(),
        url: "https://site2.com".to_string(),
        date_added: None,
        date_modified: None,
        folder_path: vec!["Batch2".to_string()],
    };

    manager
        .add_to_batch(
            "batch1".to_string(),
            0,
            bookmark1.clone(),
            "Content1".to_string(),
        )
        .await
        .unwrap();

    manager
        .add_to_batch("batch2".to_string(), 0, bookmark2, "Content2".to_string())
        .await
        .unwrap();

    // Check active batches
    let active = manager.get_active_batches().await;
    assert_eq!(active.len(), 2);
    assert!(active.contains(&"batch1".to_string()));
    assert!(active.contains(&"batch2".to_string()));

    // Complete batch1
    manager
        .add_to_batch("batch1".to_string(), 1, bookmark1, "Content1b".to_string())
        .await
        .unwrap();

    let result1 = manager.end_batch("batch1".to_string()).await.unwrap();
    assert_eq!(result1.success_count, 2);

    // batch2 should still be active
    let active = manager.get_active_batches().await;
    assert_eq!(active.len(), 1);
    assert!(active.contains(&"batch2".to_string()));
}
