use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tantivy::{Index, IndexWriter, directory::MmapDirectory};
use tracing::{debug, info};

use super::common::{
    DEFAULT_INDEX_NAME, DEFAULT_WRITER_HEAP_SIZE, INDEX_METADATA_FILE, IndexingStatus,
};
use super::indexer::BookmarkIndexer;
use super::schema::BookmarkSchema;
use super::search_manager_trait::SearchManagerTrait;
use super::tokenizer::register_lindera_tokenizer;
use super::unified_searcher::{SearchParams, SearchResult, UnifiedSearcher};

use crate::bookmark::FlatBookmark;
use crate::config::Config;

/// Index metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub version: String,
    pub index_name: String,
    pub created_at: String,
    pub last_updated: String,
    pub bookmark_count: usize,
    pub indexed_count: usize,
    pub index_size_bytes: u64,
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_documents: usize,
    pub index_size_bytes: u64,
}

/// Main search manager that coordinates indexing and searching
pub struct SearchManager {
    #[allow(dead_code)]
    index: Option<Index>, // None for read-only mode
    #[allow(dead_code)]
    schema: Option<BookmarkSchema>, // None for read-only mode
    #[allow(dead_code)]
    indexer: Option<BookmarkIndexer>, // None for read-only mode
    searcher: UnifiedSearcher,
    index_path: PathBuf,
    writer: Option<IndexWriter>,
    indexing_status: Arc<IndexingStatus>,
    read_only: bool,
}

impl std::fmt::Debug for SearchManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchManager")
            .field("index_path", &self.index_path)
            .field("has_writer", &self.writer.is_some())
            .finish()
    }
}

impl SearchManager {
    /// Generate index key from config
    pub fn get_index_key(config: &Config) -> String {
        config
            .index_name
            .clone()
            .unwrap_or_else(|| DEFAULT_INDEX_NAME.to_string())
    }

    /// Get index path from config
    fn get_index_path_from_config(config: &Config) -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mcp-bookmark")
            .join(Self::get_index_key(config))
    }

    /// Create a new search manager
    pub fn new(index_path: Option<PathBuf>) -> Result<Self> {
        let index_path = index_path.unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("mcp-bookmark")
                .join("index")
        });

        Self::new_internal(index_path, None)
    }

    /// Create a new search manager with config
    pub fn new_with_config(config: &Config) -> Result<Self> {
        let index_path = Self::get_index_path_from_config(config);
        let index_key = Self::get_index_key(config);

        info!("=================================================");
        info!("Index configuration:");
        info!("  Index name: {}", index_key);
        info!("  Index path: ~/...mcp-bookmark/{}/", index_key);
        info!("=================================================");

        Self::new_internal(index_path, Some(config))
    }

    /// Open as read-only index (compatible with Chrome extension index)
    pub fn open_readonly(index_name: &str) -> Result<Self> {
        let index_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mcp-bookmark")
            .join(index_name);

        info!("Opening read-only index at: {:?}", index_dir);

        // Open index in read-only mode (no locks)
        let searcher =
            UnifiedSearcher::open_readonly(&index_dir).context("Failed to open read-only index")?;

        // Get document count
        let stats = searcher.get_stats()?;
        let doc_count = stats.total_documents;

        info!("Read-only index opened with {} documents", doc_count);

        let indexing_status = Arc::new(IndexingStatus::for_readonly(doc_count));

        Ok(Self {
            index: None,
            schema: None,
            indexer: None,
            searcher,
            index_path: index_dir,
            writer: None,
            indexing_status,
            read_only: true,
        })
    }

    /// Internal constructor
    fn new_internal(index_path: PathBuf, config: Option<&Config>) -> Result<Self> {
        std::fs::create_dir_all(&index_path).context("Failed to create index directory")?;

        let schema = BookmarkSchema::new();

        let index = if index_path.join(INDEX_METADATA_FILE).exists() {
            info!("Using existing index: {:?}", index_path);

            if let Ok(meta_content) = std::fs::read_to_string(index_path.join(INDEX_METADATA_FILE))
            {
                if let Ok(meta) = serde_json::from_str::<IndexMetadata>(&meta_content) {
                    info!(
                        "  Last updated: {}, Bookmark count: {}",
                        meta.last_updated, meta.bookmark_count
                    );
                }
            }

            let index = Index::open_in_dir(&index_path).context("Failed to open existing index")?;
            // Register Lindera tokenizer for existing index
            register_lindera_tokenizer(&index)?;
            index
        } else {
            info!("Creating new index: {:?}", index_path);

            if let Some(cfg) = config {
                Self::write_metadata(&index_path, cfg)?;
            }

            let mmap_directory =
                MmapDirectory::open(&index_path).context("Failed to open index directory")?;

            // Create index with default settings
            let index = Index::create(mmap_directory, schema.schema.clone(), Default::default())
                .context("Failed to create new index")?;

            // Register Lindera tokenizer for new index
            register_lindera_tokenizer(&index)?;
            index
        };

        let indexer = BookmarkIndexer::new(index.clone(), schema.clone());
        let searcher = UnifiedSearcher::new(index.clone(), schema.clone())?;
        let writer = Some(indexer.create_writer(DEFAULT_WRITER_HEAP_SIZE)?);

        // Get document count for indexing status
        let doc_count = searcher.get_stats()?.total_documents;
        let indexing_status = Arc::new(IndexingStatus::new(doc_count));
        if doc_count > 0 {
            indexing_status
                .is_complete
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }

        Ok(Self {
            index: Some(index),
            schema: Some(schema),
            indexer: Some(indexer),
            searcher,
            index_path,
            writer,
            indexing_status,
            read_only: false,
        })
    }

    /// Write index metadata
    fn write_metadata(index_path: &Path, config: &Config) -> Result<()> {
        let metadata = IndexMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            index_name: Self::get_index_key(config),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
            bookmark_count: 0,
            indexed_count: 0,
            index_size_bytes: 0,
        };

        let meta_path = index_path.join(INDEX_METADATA_FILE);
        let meta_content = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(meta_path, meta_content)?;

        Ok(())
    }

    /// Create a new search manager for testing
    pub fn new_for_testing<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let index_path = index_path.as_ref().to_path_buf();

        // Ensure directory exists
        std::fs::create_dir_all(&index_path)?;

        let schema = BookmarkSchema::new();
        let index = Index::create_in_dir(&index_path, schema.schema.clone())?;

        // Register tokenizer
        register_lindera_tokenizer(&index)?;

        let indexer = BookmarkIndexer::new(index.clone(), schema.clone());
        let writer = index.writer(DEFAULT_WRITER_HEAP_SIZE)?;
        let searcher = UnifiedSearcher::new(index.clone(), schema.clone())?;

        Ok(Self {
            index: Some(index),
            schema: Some(schema),
            indexer: Some(indexer),
            searcher,
            index_path,
            writer: Some(writer),
            indexing_status: Arc::new(IndexingStatus::new(0)),
            read_only: false,
        })
    }

    /// Index a single bookmark
    pub fn index_bookmark(&mut self, bookmark: &FlatBookmark) -> Result<()> {
        if self.read_only {
            return Err(anyhow::anyhow!("Cannot index bookmark in read-only mode"));
        }
        if let (Some(writer), Some(indexer)) = (&mut self.writer, &self.indexer) {
            indexer.index_bookmark(writer, bookmark, None)?;
        }
        Ok(())
    }

    /// Index a single bookmark with content
    pub fn index_bookmark_with_content(
        &mut self,
        bookmark: &FlatBookmark,
        content: Option<&str>,
    ) -> Result<()> {
        if self.read_only {
            return Err(anyhow::anyhow!("Cannot index bookmark in read-only mode"));
        }
        if let (Some(writer), Some(indexer)) = (&mut self.writer, &self.indexer) {
            indexer.index_bookmark(writer, bookmark, content)?;
        }
        Ok(())
    }

    /// Index bookmarks with content
    pub fn index_bookmarks_with_content(
        &mut self,
        bookmarks: &[FlatBookmark],
        content_map: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        if self.read_only {
            return Err(anyhow::anyhow!("Cannot index bookmarks in read-only mode"));
        }
        if let (Some(writer), Some(indexer)) = (&mut self.writer, &self.indexer) {
            for bookmark in bookmarks {
                let content = content_map.get(&bookmark.url).map(|s| s.as_str());
                indexer.index_bookmark(writer, bookmark, content)?;
            }
            writer.commit()?;
        }
        Ok(())
    }

    /// Commit pending changes
    pub fn commit(&mut self) -> Result<()> {
        if let Some(ref mut writer) = self.writer {
            writer.commit()?;
            // Reload searcher to see new changes
            self.searcher.reload()?;
        }
        Ok(())
    }

    /// Search the index
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        debug!(
            "SearchManager::search called with query: '{}', limit: {}",
            query, limit
        );
        self.searcher.search(query, limit)
    }

    /// Search with filters
    pub fn search_with_filters(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        self.searcher.search_with_params(params)
    }

    /// Get full content by URL
    pub fn get_full_content_by_url(&self, url: &str) -> Result<Option<String>> {
        self.searcher.get_content_by_url(url)
    }

    /// Get index statistics
    pub fn get_stats(&self) -> Result<IndexStats> {
        let stats = self.searcher.get_stats()?;

        let size_bytes = Self::calculate_index_size(&self.index_path)?;

        Ok(IndexStats {
            total_documents: stats.total_documents,
            index_size_bytes: size_bytes,
        })
    }

    /// Check if index exists
    pub fn index_exists(&self) -> bool {
        self.index_path.join(INDEX_METADATA_FILE).exists()
    }

    /// Build the entire index from bookmarks
    pub fn build_index(&mut self, bookmarks: &[FlatBookmark]) -> Result<()> {
        if self.read_only {
            return Err(anyhow::anyhow!("Cannot build index in read-only mode"));
        }

        debug!("Building index for {} bookmarks", bookmarks.len());

        // Reset indexing status
        self.indexing_status = Arc::new(IndexingStatus::new(bookmarks.len()));

        if let (Some(writer), Some(indexer)) = (&mut self.writer, &self.indexer) {
            // Clear existing documents
            writer.delete_all_documents()?;

            // Index each bookmark
            let mut success_count = 0;
            let mut error_count = 0;

            for bookmark in bookmarks {
                match indexer.index_bookmark(writer, bookmark, None) {
                    Ok(_) => {
                        success_count += 1;
                        self.indexing_status
                            .completed
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to index bookmark {}: {}", bookmark.id, e);
                        error_count += 1;
                        self.indexing_status
                            .errors
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }

            writer.commit().context("Failed to commit index")?;

            // Mark indexing as complete
            self.indexing_status
                .is_complete
                .store(true, std::sync::atomic::Ordering::Relaxed);

            if error_count > 0 {
                tracing::warn!(
                    "Index built with errors: {} successful, {} errors",
                    success_count,
                    error_count
                );
            } else {
                debug!("Index built successfully: {} documents", success_count);
            }
        }

        Ok(())
    }

    /// Calculate index directory size
    fn calculate_index_size(path: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    /// Clear the index
    pub fn clear_index(&mut self) -> Result<()> {
        if let Some(ref mut writer) = self.writer {
            writer.delete_all_documents()?;
            writer.commit()?;
            info!("Index cleared");
        }
        Ok(())
    }

    /// Reload the searcher to see new changes
    pub fn reload(&mut self) -> Result<()> {
        self.searcher.reload()
    }
}

// Implement SearchManagerTrait for SearchManager
#[async_trait]
impl SearchManagerTrait for SearchManager {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        SearchManager::search(self, query, limit)
    }

    async fn search_advanced(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        self.search_with_filters(params)
    }

    async fn get_content_by_url(&self, url: &str) -> Result<Option<String>> {
        self.get_full_content_by_url(url)
    }

    fn get_indexing_status(&self) -> String {
        if self.read_only {
            format!(
                "✅ Chrome Extension index loaded: {} documents ready (read-only)",
                self.indexing_status.doc_count
            )
        } else {
            self.indexing_status.summary()
        }
    }

    fn is_indexing_complete(&self) -> bool {
        self.indexing_status
            .is_complete
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_search_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = crate::config::Config::default();
        config.index_name = Some("test_index".to_string());

        let result = SearchManager::new(Some(temp_dir.path().to_path_buf()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_index_key_generation() {
        let mut config = Config::default();
        config.index_name = Some("custom_index".to_string());
        assert_eq!(SearchManager::get_index_key(&config), "custom_index");

        let config_no_name = Config::default();
        assert_eq!(
            SearchManager::get_index_key(&config_no_name),
            DEFAULT_INDEX_NAME
        );
    }
}
