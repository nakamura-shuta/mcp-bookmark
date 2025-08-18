use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{Duration, timeout};
use tracing::{debug, info, warn};

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

/// Index building status
#[derive(Debug)]
pub struct IndexingStatus {
    /// Total bookmark count
    pub total: AtomicUsize,

    /// Completed count
    pub completed: AtomicUsize,

    /// Error count
    pub errors: AtomicUsize,

    /// Completion flag
    pub is_complete: AtomicBool,

    /// Start time
    pub started_at: std::time::Instant,

    /// Using pre-built index from Chrome extension
    pub using_prebuilt: bool,

    /// Number of documents in pre-built index
    pub prebuilt_doc_count: usize,
}

impl IndexingStatus {
    pub fn new(total: usize) -> Self {
        Self {
            total: AtomicUsize::new(total),
            completed: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            is_complete: AtomicBool::new(false),
            started_at: std::time::Instant::now(),
            using_prebuilt: false,
            prebuilt_doc_count: 0,
        }
    }

    /// Create for pre-built index
    pub fn for_prebuilt(doc_count: usize) -> Self {
        Self {
            total: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            is_complete: AtomicBool::new(true),
            started_at: std::time::Instant::now(),
            using_prebuilt: true,
            prebuilt_doc_count: doc_count,
        }
    }

    /// Get progress percentage (0.0 - 100.0)
    pub fn progress_percentage(&self) -> f64 {
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            return 100.0;
        }
        let completed = self.completed.load(Ordering::Relaxed);
        (completed as f64 / total as f64) * 100.0
    }

    /// Generate status string
    pub fn status_string(&self) -> String {
        // Special message for pre-built index
        if self.using_prebuilt {
            return format!(
                "✅ Chrome Extension index loaded: {} documents ready",
                self.prebuilt_doc_count
            );
        }

        let total = self.total.load(Ordering::Relaxed);
        let completed = self.completed.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        let elapsed = self.started_at.elapsed();

        if self.is_complete.load(Ordering::Relaxed) {
            format!(
                "✅ Index build complete: {}/{} success, {} errors (duration: {:.1}s)",
                completed - errors,
                total,
                errors,
                elapsed.as_secs_f64()
            )
        } else {
            let eta = if completed > 0 {
                let per_item = elapsed.as_secs_f64() / completed as f64;
                let remaining = total - completed;
                Duration::from_secs_f64(per_item * remaining as f64)
            } else {
                Duration::from_secs(0)
            };

            format!(
                "📥 Building index: {}/{} ({:.1}%, {} errors, estimated remaining: {}s)",
                completed,
                total,
                self.progress_percentage(),
                errors,
                eta.as_secs()
            )
        }
    }
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
                        stats.num_documents
                    );
                    (stats.num_documents > 0, stats.num_documents)
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
                            let content_text = content.text_content.as_deref();
                            if let Err(e) = search.update_bookmark(&bookmark, content_text) {
                                warn!("Index update failed {}: {}", bookmark.url, e);
                                status.errors.fetch_add(1, Ordering::Relaxed);
                            } else {
                                debug!("✅ Index update succeeded: {}", bookmark.url);
                            }
                        }
                        Ok(Err(e)) => {
                            warn!("Content fetch failed {}: {}", bookmark.url, e);
                            status.errors.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(_) => {
                            warn!("Timeout (5s): {}", bookmark.url);
                            status.errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    // Update progress
                    let completed = status.completed.fetch_add(1, Ordering::Relaxed) + 1;
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
                        info!("{}", status.status_string());
                    }

                    if completed == total {
                        // Final metadata update
                        let total_val = status.total.load(Ordering::Relaxed);
                        let errors = status.errors.load(Ordering::Relaxed);
                        let search = search_for_meta.lock().await;
                        let _ = search.update_metadata(total_val, completed - errors);
                        drop(search);

                        status.is_complete.store(true, Ordering::Relaxed);
                        info!("🎉 Content index build complete!");
                    }
                });

                handles.push(handle);
            }

            // Wait for all tasks to complete
            for handle in handles {
                let _ = handle.await;
            }
        });
    }

    /// Execute search (using tantivy only)
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        info!(
            "ContentIndexManager::search called with query: '{}', limit: {}",
            query, limit
        );

        // Search with tantivy
        let search = self.tantivy_search.lock().await;
        info!("Calling tantivy search...");
        let results = search.search(query, limit)?;
        info!("Tantivy search returned {} results", results.len());

        // Provide information if few results during indexing
        if results.is_empty() && !self.indexing_status.is_complete.load(Ordering::Relaxed) {
            debug!(
                "No search results. {} (Results may be incomplete as content index is still building)",
                self.indexing_status.status_string()
            );
        } else if !results.is_empty() {
            debug!("Search hits: {} items", results.len());
        }

        Ok(results)
    }

    /// Advanced search (with filters)
    pub async fn search_advanced(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        // Use tantivy only (filter search is tantivy feature)
        let search = self.tantivy_search.lock().await;
        search.search_advanced(params)
    }

    /// Get Index building status
    pub fn get_indexing_status(&self) -> String {
        self.indexing_status.status_string()
    }

    /// Check if index building is complete
    pub fn is_indexing_complete(&self) -> bool {
        self.indexing_status.is_complete.load(Ordering::Relaxed)
    }

    /// Get full content from URL (from index only)
    pub async fn get_content_by_url(&self, url: &str) -> Result<Option<String>> {
        // Get content directly from index only
        let search = self.tantivy_search.lock().await;

        // Get full content from index
        match search.get_content_by_url(url) {
            Ok(Some(content)) => {
                info!("Content fetched from index successfully: {}", url);
                Ok(Some(content))
            }
            Ok(None) => {
                info!("Content not found in index for URL: {}", url);
                Ok(None)
            }
            Err(e) => {
                warn!("Error fetching content from index for {}: {}", url, e);
                Ok(None)
            }
        }
    }
}

// Implement the SearchManagerTrait for ContentIndexManager
#[async_trait]
impl SearchManagerTrait for ContentIndexManager {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search(query, limit).await
    }

    async fn search_advanced(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        self.search_advanced(params).await
    }

    async fn get_content_by_url(&self, url: &str) -> Result<Option<String>> {
        self.get_content_by_url(url).await
    }

    fn get_indexing_status(&self) -> String {
        self.get_indexing_status()
    }

    fn is_indexing_complete(&self) -> bool {
        self.is_indexing_complete()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_manager_creation() {
        // Test configuration
        let config = crate::config::Config::default();
        let reader = Arc::new(BookmarkReader::with_config(config).unwrap());
        let fetcher = Arc::new(ContentFetcher::new().unwrap());

        // Create search manager
        let manager = ContentIndexManager::new(reader, fetcher).await.unwrap();

        // Check index building status
        assert!(!manager.is_indexing_complete());
        let status = manager.get_indexing_status();
        assert!(status.contains("Building index") || status.contains("Index build"));
    }

    #[tokio::test]
    #[ignore] // This test requires exclusive access to index directory
    async fn test_simple_search() {
        let config = crate::config::Config::default();
        let reader = Arc::new(BookmarkReader::with_config(config).unwrap());
        let fetcher = Arc::new(ContentFetcher::new().unwrap());

        let manager = ContentIndexManager::new(reader, fetcher).await.unwrap();

        // Metadata search (without content)
        let results = manager.search("test", 10).await.unwrap();
        // Results are environment-dependent, just check for no errors
        let _ = results;
    }
}
