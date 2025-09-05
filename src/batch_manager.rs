use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::bookmark::FlatBookmark;
use crate::search::SearchManager;

/// Batch processing state
#[derive(Debug)]
pub struct BatchState {
    pub batch_id: String,
    pub total: usize,
    pub received: HashSet<usize>,
    pub bookmarks: Vec<(FlatBookmark, String)>,
    pub start_time: Instant,
    pub last_activity: Instant,
    pub immediate_commit: bool,
}

impl BatchState {
    pub fn new(batch_id: String, total: usize) -> Self {
        let now = Instant::now();
        // For small batches (<=2), commit immediately
        let immediate_commit = total <= 2;

        Self {
            batch_id,
            total,
            received: HashSet::new(),
            bookmarks: Vec::with_capacity(total),
            start_time: now,
            last_activity: now,
            immediate_commit,
        }
    }

    pub fn add_bookmark(&mut self, index: usize, bookmark: FlatBookmark, content: String) {
        self.received.insert(index);
        self.bookmarks.push((bookmark, content));
        self.last_activity = Instant::now();
    }

    pub fn is_complete(&self) -> bool {
        self.received.len() == self.total
    }

    pub fn is_stale(&self) -> bool {
        self.last_activity.elapsed() > Duration::from_secs(120)
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Batch processing result
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchResult {
    pub batch_id: String,
    pub success_count: usize,
    pub failed_count: usize,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

/// Batch index manager for parallel processing
pub struct BatchIndexManager {
    batches: Arc<Mutex<HashMap<String, BatchState>>>,
    search_manager: Arc<Mutex<SearchManager>>,
    max_buffer_size: usize,
}

impl BatchIndexManager {
    pub fn new(search_manager: SearchManager) -> Self {
        Self {
            batches: Arc::new(Mutex::new(HashMap::new())),
            search_manager: Arc::new(Mutex::new(search_manager)),
            max_buffer_size: 50, // Buffer size before auto-commit
        }
    }

    /// Start a new batch
    pub async fn start_batch(&self, batch_id: String, total: usize) -> Result<()> {
        if total == 0 {
            return Err(anyhow::anyhow!("Cannot start empty batch"));
        }

        let mut batches = self.batches.lock().await;

        if batches.contains_key(&batch_id) {
            warn!("Batch {} already exists, replacing", batch_id);
        }

        let batch = BatchState::new(batch_id.clone(), total);
        let immediate = batch.immediate_commit;

        info!(
            "Started batch {} with {} bookmarks (immediate_commit: {})",
            batch_id, total, immediate
        );

        batches.insert(batch_id, batch);
        Ok(())
    }

    /// Add bookmark to batch
    pub async fn add_to_batch(
        &self,
        batch_id: String,
        index: usize,
        bookmark: FlatBookmark,
        content: String,
    ) -> Result<()> {
        let should_commit;
        let mut bookmarks_to_commit = Vec::new();

        // Update batch state
        {
            let mut batches = self.batches.lock().await;

            if let Some(batch) = batches.get_mut(&batch_id) {
                // Check for duplicate index
                if batch.received.contains(&index) {
                    warn!("Duplicate index {} in batch {}", index, batch_id);
                    return Ok(());
                }

                batch.add_bookmark(index, bookmark, content);
                debug!(
                    "Added bookmark {}/{} to batch {}",
                    batch.received.len(),
                    batch.total,
                    batch_id
                );

                // Determine if we should commit
                should_commit = batch.immediate_commit
                    || batch.bookmarks.len() >= self.max_buffer_size
                    || batch.is_complete();

                if should_commit {
                    bookmarks_to_commit = batch.bookmarks.drain(..).collect();
                    if batch.immediate_commit {
                        debug!("Immediate commit for small batch {}", batch_id);
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Batch {} not found", batch_id));
            }
        }

        // Commit if needed (outside lock)
        if should_commit && !bookmarks_to_commit.is_empty() {
            self.commit_bookmarks(bookmarks_to_commit).await?;
        }

        Ok(())
    }

    /// End batch and return results
    pub async fn end_batch(&self, batch_id: String) -> Result<BatchResult> {
        let batch = {
            let mut batches = self.batches.lock().await;
            batches.remove(&batch_id)
        };

        if let Some(batch) = batch {
            let duration = batch.elapsed();
            let total = batch.total;
            let received = batch.received.len();

            // Commit remaining bookmarks
            if !batch.bookmarks.is_empty() {
                let count = batch.bookmarks.len();
                self.commit_bookmarks(batch.bookmarks).await?;
                debug!("Final commit of {} bookmarks for batch {}", count, batch_id);
            }

            info!(
                "Batch {} completed: {}/{} bookmarks in {:?}",
                batch_id, received, total, duration
            );

            Ok(BatchResult {
                batch_id,
                success_count: received,
                failed_count: total.saturating_sub(received),
                duration_ms: duration.as_millis() as u64,
                errors: Vec::new(),
            })
        } else {
            Err(anyhow::anyhow!("Batch {} not found", batch_id))
        }
    }

    /// Commit bookmarks to index
    async fn commit_bookmarks(&self, bookmarks: Vec<(FlatBookmark, String)>) -> Result<()> {
        let mut manager = self.search_manager.lock().await;
        let count = bookmarks.len();

        debug!("Committing {} bookmarks to index", count);

        for (bookmark, content) in bookmarks {
            manager
                .index_bookmark_with_content(&bookmark, Some(&content))
                .context("Failed to index bookmark")?;
        }

        manager.commit().context("Failed to commit index")?;
        debug!("Successfully committed {} bookmarks", count);

        Ok(())
    }

    /// Clean up stale batches
    pub async fn cleanup_stale_batches(&self) {
        let mut batches = self.batches.lock().await;
        let stale_ids: Vec<String> = batches
            .iter()
            .filter(|(_, batch)| batch.is_stale())
            .map(|(id, _)| id.clone())
            .collect();

        for id in stale_ids {
            warn!("Removing stale batch: {}", id);
            if let Some(batch) = batches.remove(&id) {
                // Try to save what we have
                if !batch.bookmarks.is_empty() {
                    let bookmarks = batch.bookmarks;
                    drop(batches);

                    if let Err(e) = self.commit_bookmarks(bookmarks).await {
                        warn!("Failed to commit stale batch {}: {}", id, e);
                    }

                    return; // Re-acquire lock next time
                }
            }
        }
    }

    /// Get batch status
    pub async fn get_batch_status(&self, batch_id: &str) -> Option<(usize, usize)> {
        let batches = self.batches.lock().await;
        batches
            .get(batch_id)
            .map(|batch| (batch.received.len(), batch.total))
    }

    /// Get all active batches
    pub async fn get_active_batches(&self) -> Vec<String> {
        let batches = self.batches.lock().await;
        batches.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_batch_state() {
        let batch = BatchState::new("test_batch".to_string(), 5);
        assert!(!batch.is_complete());
        assert!(!batch.is_stale());
        assert_eq!(batch.total, 5);
    }

    #[tokio::test]
    async fn test_small_batch_immediate_commit() {
        let batch = BatchState::new("small_batch".to_string(), 2);
        assert!(batch.immediate_commit);

        let batch = BatchState::new("large_batch".to_string(), 10);
        assert!(!batch.immediate_commit);
    }

    #[tokio::test]
    async fn test_batch_manager_empty_batch() {
        let temp_dir = TempDir::new().unwrap();
        let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
        let manager = BatchIndexManager::new(search_manager);

        let result = manager.start_batch("empty".to_string(), 0).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty batch"));
    }

    #[tokio::test]
    async fn test_batch_manager_basic_flow() {
        let temp_dir = TempDir::new().unwrap();
        let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
        let manager = BatchIndexManager::new(search_manager);

        // Start batch
        manager
            .start_batch("test_batch".to_string(), 3)
            .await
            .unwrap();

        // Add bookmarks
        for i in 0..3 {
            let bookmark = FlatBookmark {
                id: format!("id_{i}"),
                name: format!("Bookmark {i}"),
                url: format!("https://example.com/{i}"),
                date_added: Some("1234567890".to_string()),
                date_modified: None,
                folder_path: vec!["Test".to_string()],
            };

            manager
                .add_to_batch(
                    "test_batch".to_string(),
                    i,
                    bookmark,
                    format!("Content {i}"),
                )
                .await
                .unwrap();
        }

        // End batch
        let result = manager.end_batch("test_batch".to_string()).await.unwrap();
        assert_eq!(result.success_count, 3);
        assert_eq!(result.failed_count, 0);
    }

    #[tokio::test]
    async fn test_duplicate_index_handling() {
        let temp_dir = TempDir::new().unwrap();
        let search_manager = SearchManager::new_for_testing(temp_dir.path()).unwrap();
        let manager = BatchIndexManager::new(search_manager);

        manager.start_batch("test".to_string(), 2).await.unwrap();

        let bookmark = FlatBookmark {
            id: "1".to_string(),
            name: "Test".to_string(),
            url: "https://test.com".to_string(),
            date_added: None,
            date_modified: None,
            folder_path: vec![],
        };

        // Add same index twice
        manager
            .add_to_batch(
                "test".to_string(),
                0,
                bookmark.clone(),
                "Content".to_string(),
            )
            .await
            .unwrap();

        // Second add should be ignored
        manager
            .add_to_batch("test".to_string(), 0, bookmark, "Content2".to_string())
            .await
            .unwrap();

        let status = manager.get_batch_status("test").await.unwrap();
        assert_eq!(status.0, 1); // Only 1 bookmark added
    }

    #[tokio::test]
    async fn test_batch_not_found() {
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
}
