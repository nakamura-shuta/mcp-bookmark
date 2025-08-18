use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use super::SearchResult;
use super::readonly_searcher::ReadOnlySearcher;
use super::search_manager_trait::SearchManagerTrait;
use crate::bookmark::BookmarkReader;

/// Read-only content index manager for Chrome extension indexes
/// This doesn't use any locks and allows multiple processes to access the same index
#[derive(Clone, Debug)]
pub struct ReadOnlyIndexManager {
    /// Read-only searcher (no locks)
    searcher: Arc<ReadOnlySearcher>,

    /// Index status
    indexing_status: Arc<IndexingStatus>,
}

/// Index status for read-only access
#[derive(Debug)]
pub struct IndexingStatus {
    /// Number of documents in index
    pub doc_count: usize,
    /// Whether using pre-built index
    pub using_prebuilt: bool,
}

impl ReadOnlyIndexManager {
    /// Create new read-only manager for Chrome extension index
    pub async fn new(reader: Arc<BookmarkReader>) -> Result<Self> {
        // Get index name from config
        let index_name = reader
            .config
            .index_name
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("INDEX_NAME is required"))?;

        Self::new_with_index_name(index_name).await
    }

    /// Create new read-only manager with explicit index name
    pub async fn new_with_index_name(index_name: &str) -> Result<Self> {
        let index_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mcp-bookmark")
            .join(index_name);

        info!("Opening read-only index at: {:?}", index_dir);

        // Open index in read-only mode (no locks)
        let searcher =
            ReadOnlySearcher::open(&index_dir).context("Failed to open read-only index")?;

        // Get document count
        let stats = searcher.get_stats()?;
        let doc_count = stats.num_documents;

        info!("Read-only index opened with {} documents", doc_count);

        let indexing_status = Arc::new(IndexingStatus {
            doc_count,
            using_prebuilt: true,
        });

        Ok(Self {
            searcher: Arc::new(searcher),
            indexing_status,
        })
    }

    /// Execute search (lock-free, thread-safe)
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        info!(
            "ReadOnlyIndexManager::search called with query: '{}', limit: {}",
            query, limit
        );

        // Direct search without any locks
        let results = self.searcher.search(query, limit)?;

        info!("Search returned {} results", results.len());
        Ok(results)
    }

    /// Get full content by URL (lock-free)
    pub async fn get_content_by_url(&self, url: &str) -> Result<Option<String>> {
        info!("Getting content for URL: {}", url);
        self.searcher.get_content_by_url(url)
    }

    /// Get indexing status
    pub fn get_indexing_status(&self) -> String {
        format!(
            "✅ Chrome Extension index loaded: {} documents ready (read-only)",
            self.indexing_status.doc_count
        )
    }

    /// Check if indexing is complete (always true for read-only)
    pub fn is_indexing_complete(&self) -> bool {
        true
    }
}

// Implement the SearchManagerTrait for ReadOnlyIndexManager
#[async_trait]
impl SearchManagerTrait for ReadOnlyIndexManager {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search(query, limit).await
    }

    async fn search_advanced(&self, params: &super::SearchParams) -> Result<Vec<SearchResult>> {
        // For read-only index, we can only do simple text search
        // Use query if provided, otherwise return empty results
        if let Some(query) = &params.query {
            self.search(query, params.limit).await
        } else {
            Ok(Vec::new())
        }
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
