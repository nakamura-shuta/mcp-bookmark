use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{Duration, timeout};
use tracing::{debug, info, warn};

use super::common::IndexingStatus;
use super::{SearchManager, SearchParams, SearchResult, search_manager_trait::SearchManagerTrait};
use crate::bookmark::{BookmarkReader, FlatBookmark};
use crate::content::ContentFetcher;

/// Content index management
/// Progressively index content in background and provide search functionality
#[derive(Debug, Clone)]
pub struct ContentIndexManager {
    /// Tantivy search engine
    tantivy_search: Arc<Mutex<SearchManager>>,

    /// For content fetching
    content_fetcher: Arc<ContentFetcher>,

    /// Index building status
    indexing_status: Arc<IndexingStatus>,
}

impl ContentIndexManager {
    /// Create new
    pub async fn new(reader: Arc<BookmarkReader>, fetcher: Arc<ContentFetcher>) -> Result<Self> {
        // Check if we're using pre-built index (from Chrome extension)
        let using_prebuilt_index = reader.config.index_name.is_some();

        let (bookmarks, total) = if using_prebuilt_index {
            // Don't read bookmarks when using pre-built index
            debug!("Using pre-built index, skipping bookmark file reading");
            (vec![], 0)
        } else {
            // Get bookmarks for normal operation
            let bookmarks = reader.read_bookmarks()?;
            let total = bookmarks.len();
            debug!("Initializing search manager ({} bookmarks)", total);
            (bookmarks, total)
        };

        // Create SearchManager - using config
        let mut search_manager = SearchManager::new_with_config(&reader.config)?;

        // Check if index already exists with content
        let index_exists = search_manager.index_exists();
        let (has_content, doc_count) = if index_exists {
            // Check if we have indexed content already
            match search_manager.get_stats() {
                Ok(stats) => {
                    info!(
                        "Existing index found with {} documents",
                        stats.total_documents
                    );
                    (stats.total_documents > 0, stats.total_documents)
                }
                Err(_) => (false, 0),
            }
        } else {
            (false, 0)
        };

        // Only rebuild if index is empty or doesn't exist (and we have bookmarks)
        if !has_content && !bookmarks.is_empty() {
            debug!("Building new index with metadata...");
            search_manager.build_index(&bookmarks)?;
        } else if has_content {
            debug!("Using existing index, skipping rebuild");
        }

        // Create appropriate IndexingStatus based on whether we're using pre-built index
        let indexing_status = if using_prebuilt_index {
            Arc::new(IndexingStatus::for_prebuilt(doc_count))
        } else {
            Arc::new(IndexingStatus::new(total))
        };

        // Create manager
        let manager = Self {
            tantivy_search: Arc::new(Mutex::new(search_manager)),
            content_fetcher: fetcher,
            indexing_status,
        };

        // Start fetching content in background (only if we have bookmarks)
        if !bookmarks.is_empty() {
            manager.start_background_indexing(bookmarks).await;
        }
        // Note: For pre-built index, is_complete is already set to true in IndexingStatus::for_prebuilt()

        Ok(manager)
    }

    /// Index content in background
    async fn start_background_indexing(&self, bookmarks: Vec<FlatBookmark>) {
        let search_manager = self.tantivy_search.clone();
        let fetcher = self.content_fetcher.clone();
        let status = self.indexing_status.clone();

        tokio::spawn(async move {
            info!("Starting background index building");

            // Sort by priority (important domains first)
            let mut bookmarks = bookmarks;
            bookmarks.sort_by_key(|b| {
                // Extract domain from URL
                let domain = url::Url::parse(&b.url)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h.to_string()))
                    .unwrap_or_default();

                match domain.as_str() {
                    // Documentation sites have highest priority
                    "docs.rs" | "doc.rust-lang.org" => 0,
                    "react.dev" | "reactjs.org" => 1,
                    "developer.mozilla.org" => 2,
                    "docs.github.com" => 3,
                    "docs.aws.amazon.com" => 4,
                    // Tech blogs
                    "medium.com" | "dev.to" => 10,
                    "stackoverflow.com" => 11,
                    // Others
                    _ => 100,
                }
            });

            // Concurrency limit (10 parallel)
            let semaphore = Arc::new(Semaphore::new(10));
            let mut handles = vec![];

            for bookmark in bookmarks {
                let sem = semaphore.clone();
                let search = search_manager.clone();
                let fetcher = fetcher.clone();
                let status = status.clone();
                let search_for_meta = search_manager.clone();

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();

                    info!("📄 Starting content fetch: {}", bookmark.url);

                    // Fetch content (5 second timeout)
                    let fetch_result =
                        timeout(Duration::from_secs(5), fetcher.fetch_page(&bookmark.url)).await;

                    match fetch_result {
                        Ok(Ok(html)) => {
                            // Extract content
                            let content = fetcher.extract_content(&html, &bookmark.url);

                            // Update tantivy index
                            let mut search = search.lock().await;
                            let _content_text = content.text_content.as_deref();
                            if let Err(e) = search.index_bookmark(&bookmark) {
                                warn!("Index update failed {}: {}", bookmark.url, e);
                                status.increment_errors();
                            } else {
                                debug!("✅ Index update succeeded: {}", bookmark.url);
                            }
                        }
                        Ok(Err(e)) => {
                            warn!("Content fetch failed {}: {}", bookmark.url, e);
                            status.increment_errors();
                        }
                        Err(_) => {
                            warn!("Timeout (5s): {}", bookmark.url);
                            status.increment_errors();
                        }
                    }

                    // Update progress
                    status.increment_completed();
                    let completed = status.completed.load(Ordering::Relaxed);
                    let total = status.total.load(Ordering::Relaxed);

                    // Show progress (10% increments, or first/last)
                    let percentage = (completed as f64 / total as f64 * 100.0) as u32;
                    let prev_percentage = ((completed - 1) as f64 / total as f64 * 100.0) as u32;

                    if completed == 1
                        || completed == total
                        || (percentage / 10 != prev_percentage / 10) // 10% increments
                        || (completed == 10 || completed == 50 || completed == 100)
                    // Milestone
                    {
                        info!("{}", status.summary());
                    }

                    if completed == total {
                        // Final metadata update
                        let mut search = search_for_meta.lock().await;
                        if let Err(e) = search.commit() {
                            warn!("Failed final commit: {}", e);
                        }
                    }
                });

                handles.push(handle);
            }

            // Wait for all to complete
            for handle in handles {
                let _ = handle.await;
            }

            // Mark as complete
            status.mark_complete();
            info!("{}", status.summary());
        });
    }

    /// Execute search
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let search = self.tantivy_search.lock().await;
        search.search(query, limit)
    }

    /// Search with filters
    pub async fn search_with_filters(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        let search = self.tantivy_search.lock().await;
        search.search_with_filters(params)
    }

    /// Get full content by URL
    pub async fn get_full_content_by_url(&self, url: &str) -> Result<Option<String>> {
        let search = self.tantivy_search.lock().await;
        search.get_full_content_by_url(url)
    }

    /// Get indexing status string
    pub fn get_indexing_status(&self) -> String {
        self.indexing_status.summary()
    }

    /// Check if indexing is complete
    pub fn is_indexing_complete(&self) -> bool {
        self.indexing_status.is_complete.load(Ordering::Relaxed)
    }
}

// Implement SearchManagerTrait for ContentIndexManager
#[async_trait]
impl SearchManagerTrait for ContentIndexManager {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        ContentIndexManager::search(self, query, limit).await
    }

    async fn search_advanced(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        self.search_with_filters(params).await
    }

    async fn get_content_by_url(&self, url: &str) -> Result<Option<String>> {
        self.get_full_content_by_url(url).await
    }

    fn get_indexing_status(&self) -> String {
        self.get_indexing_status()
    }

    fn is_indexing_complete(&self) -> bool {
        self.is_indexing_complete()
    }
}
