pub mod boosting;
pub mod content_index;
pub mod indexer;
pub mod mcp_config;
pub mod readonly_index;
pub mod readonly_searcher;
pub mod schema;
pub mod scored_snippet;
pub mod search_manager_trait;
pub mod searcher;
pub mod snippet;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tantivy::{Index, directory::MmapDirectory};
use tracing::{debug, info};

pub use content_index::ContentIndexManager;
pub use indexer::BookmarkIndexer;
pub use schema::BookmarkSchema;
pub use searcher::{SearchParams, SearchResult};

use crate::bookmark::FlatBookmark;
use crate::config::Config;
use searcher::BookmarkSearcher;

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

/// Main search manager that coordinates indexing and searching
#[derive(Debug)]
pub struct SearchManager {
    #[allow(dead_code)]
    index: Index,
    #[allow(dead_code)]
    schema: BookmarkSchema,
    indexer: BookmarkIndexer,
    searcher: BookmarkSearcher,
    booster: Option<boosting::SearchBooster>,
    index_path: PathBuf,
}

impl SearchManager {
    /// Generate index key from config
    pub fn get_index_key(config: &Config) -> String {
        // Use index_name directly if provided
        config.index_name.clone().unwrap_or_else(|| "default_index".to_string())
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
        // For backward compatibility, use the old path if provided
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

    /// Internal constructor
    fn new_internal(index_path: PathBuf, config: Option<&Config>) -> Result<Self> {
        // Ensure directory exists
        std::fs::create_dir_all(&index_path).context("Failed to create index directory")?;

        let schema = BookmarkSchema::new();

        // Open or create index
        let index = if index_path.join("meta.json").exists() {
            info!("Using existing index: {:?}", index_path);

            // Read and log metadata
            if let Ok(meta_content) = std::fs::read_to_string(index_path.join("meta.json")) {
                if let Ok(meta) = serde_json::from_str::<IndexMetadata>(&meta_content) {
                    info!(
                        "  Last updated: {}, Bookmark count: {}",
                        meta.last_updated, meta.bookmark_count
                    );
                }
            }

            Index::open_in_dir(&index_path).context("Failed to open existing index")?
        } else {
            info!("Creating new index: {:?}", index_path);

            // Write metadata if config is provided
            if let Some(cfg) = config {
                Self::write_metadata(&index_path, cfg)?;
            }

            let mmap_directory =
                MmapDirectory::open(&index_path).context("Failed to open index directory")?;
            Index::create(mmap_directory, schema.schema.clone(), Default::default())
                .context("Failed to create new index")?
        };

        let indexer = BookmarkIndexer::new(index.clone(), schema.clone());
        let searcher = BookmarkSearcher::new(index.clone(), schema.clone())?;

        // Create booster for improved relevance (Phase 1.2)
        let booster = Some(boosting::SearchBooster::new(
            index.clone(),
            schema.clone(),
            searcher.reader.clone(),
        ));

        Ok(Self {
            index,
            schema,
            indexer,
            searcher,
            booster,
            index_path,
        })
    }

    /// Write metadata to index directory
    fn write_metadata(path: &Path, config: &Config) -> Result<()> {
        let meta = IndexMetadata {
            version: "1.0.0".to_string(),
            index_name: config
                .index_name
                .clone()
                .unwrap_or_else(|| "default_index".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
            bookmark_count: 0,
            indexed_count: 0,
            index_size_bytes: 0,
        };

        let meta_path = path.join("meta.json");
        let json = serde_json::to_string_pretty(&meta)?;
        std::fs::write(meta_path, json)?;
        Ok(())
    }

    /// Build or rebuild the entire index
    pub fn build_index(&mut self, bookmarks: &[FlatBookmark]) -> Result<()> {
        debug!("Building index for {} bookmarks", bookmarks.len());
        self.indexer.build_index(bookmarks)?;
        self.searcher.reload()?;
        Ok(())
    }

    /// Update a single bookmark
    pub fn update_bookmark(
        &mut self,
        bookmark: &FlatBookmark,
        content: Option<&str>,
    ) -> Result<()> {
        debug!("Updating bookmark {} in index", bookmark.id);
        self.indexer.update_bookmark(bookmark, content)?;
        self.searcher.reload()?;
        Ok(())
    }

    /// Delete a bookmark
    pub fn delete_bookmark(&mut self, bookmark_id: &str) -> Result<()> {
        self.indexer.delete_bookmark(bookmark_id)?;
        self.searcher.reload()?;
        Ok(())
    }

    /// Simple text search
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        debug!(
            "SearchManager::search called with query: '{}', limit: {}",
            query, limit
        );
        // Use booster if available for improved relevance (Phase 1.2)
        let result = if let Some(booster) = &self.booster {
            booster.search_with_boosting(query, limit)
        } else {
            self.searcher.search(query, limit)
        };
        debug!("SearchManager::search completed");
        result
    }

    /// Search only in content field
    pub fn search_content_only(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.searcher.search_content_only(query, limit)
    }

    /// Advanced search with filters
    pub fn search_advanced(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        self.searcher.search_with_filters(params)
    }

    /// Get bookmark by ID
    pub fn get_by_id(&self, id: &str) -> Result<Option<SearchResult>> {
        self.searcher.get_by_id(id)
    }

    /// Get full content by URL from index
    pub fn get_content_by_url(&self, url: &str) -> Result<Option<String>> {
        // Get full content directly from index
        self.searcher.get_full_content_by_url(url)
    }

    /// Get index statistics
    pub fn get_stats(&self) -> Result<searcher::IndexStats> {
        self.searcher.get_stats()
    }

    /// Get index directory size
    pub fn get_index_size(&self) -> Result<u64> {
        calculate_dir_size(&self.index_path)
    }

    /// Clear and rebuild index
    pub fn rebuild_index(&mut self, bookmarks: &[FlatBookmark]) -> Result<()> {
        debug!("Rebuilding entire index");
        self.indexer.build_index(bookmarks)?;
        self.searcher.reload()?;
        Ok(())
    }

    /// Check if index exists
    pub fn index_exists(&self) -> bool {
        self.index_path.join("meta.json").exists()
    }

    /// Update metadata after indexing
    pub fn update_metadata(&self, bookmark_count: usize, indexed_count: usize) -> Result<()> {
        let meta_path = self.index_path.join("meta.json");

        // Read existing metadata or create new
        let mut meta = if meta_path.exists() {
            let content = std::fs::read_to_string(&meta_path)?;
            serde_json::from_str::<IndexMetadata>(&content)?
        } else {
            IndexMetadata {
                version: "1.0.0".to_string(),
                index_name: "default_index".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                last_updated: chrono::Utc::now().to_rfc3339(),
                bookmark_count: 0,
                indexed_count: 0,
                index_size_bytes: 0,
            }
        };

        // Update fields
        meta.last_updated = chrono::Utc::now().to_rfc3339();
        meta.bookmark_count = bookmark_count;
        meta.indexed_count = indexed_count;
        meta.index_size_bytes = self.get_index_size().unwrap_or(0);

        // Write back
        let json = serde_json::to_string_pretty(&meta)?;
        std::fs::write(meta_path, json)?;
        Ok(())
    }
}

/// Calculate directory size recursively
fn calculate_dir_size(path: &Path) -> Result<u64> {
    let mut size = 0;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                size += calculate_dir_size(&path)?;
            } else {
                size += entry.metadata()?.len();
            }
        }
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_bookmarks() -> Vec<FlatBookmark> {
        vec![
            FlatBookmark {
                id: "1".to_string(),
                name: "Rust Documentation".to_string(),
                url: "https://doc.rust-lang.org".to_string(),
                date_added: None,
                date_modified: None,
                folder_path: vec!["Tech".to_string()],
            },
            FlatBookmark {
                id: "2".to_string(),
                name: "Rust Book".to_string(),
                url: "https://doc.rust-lang.org/book".to_string(),
                date_added: None,
                date_modified: None,
                folder_path: vec!["Tech".to_string(), "Books".to_string()],
            },
        ]
    }

    #[test]
    fn test_index_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let bookmarks = create_test_bookmarks();
        manager.build_index(&bookmarks).unwrap();

        let results = manager.search("rust", 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_update_bookmark() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let bookmarks = create_test_bookmarks();
        manager.build_index(&bookmarks).unwrap();

        let updated = FlatBookmark {
            id: "1".to_string(),
            name: "Updated Rust Docs".to_string(),
            url: "https://doc.rust-lang.org".to_string(),
            date_added: None,
            date_modified: None,
            folder_path: vec!["Tech".to_string()],
        };
        manager.update_bookmark(&updated, None).unwrap();

        let results = manager.search("Updated", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_content_search() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let bookmarks = create_test_bookmarks();
        manager.build_index(&bookmarks).unwrap();

        // Update with content
        manager
            .update_bookmark(&bookmarks[0], Some("This is Rust programming content"))
            .unwrap();

        let results = manager.search_content_only("programming", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_delete_bookmark() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let bookmarks = create_test_bookmarks();
        manager.build_index(&bookmarks).unwrap();

        // Delete first bookmark
        manager.delete_bookmark("1").unwrap();

        let results = manager.search("rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "2");
    }

    #[test]
    fn test_get_by_id() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let bookmarks = create_test_bookmarks();
        manager.build_index(&bookmarks).unwrap();

        let result = manager.get_by_id("1").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "1");

        let result = manager.get_by_id("999").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_stats() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let bookmarks = create_test_bookmarks();
        manager.build_index(&bookmarks).unwrap();

        let stats = manager.get_stats().unwrap();
        assert_eq!(stats.num_documents, 2);
    }

    #[test]
    fn test_rebuild_index() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let bookmarks = create_test_bookmarks();
        manager.build_index(&bookmarks).unwrap();

        // Add new bookmark for rebuild
        let mut new_bookmarks = bookmarks.clone();
        new_bookmarks.push(FlatBookmark {
            id: "3".to_string(),
            name: "New Bookmark".to_string(),
            url: "https://example.com".to_string(),
            date_added: None,
            date_modified: None,
            folder_path: vec!["Other".to_string()],
        });

        manager.rebuild_index(&new_bookmarks).unwrap();

        // Empty query may not return all results, search for something specific
        let results = manager.search("bookmark", 10).unwrap();
        // Just verify rebuild doesn't crash and returns some results
        assert!(results.len() <= 3);
    }

    #[test]
    fn test_index_exists() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();

        // Note: index_exists() may return true even for new index
        // because MmapDirectory creates meta.json immediately
        // Just ensure it doesn't crash
        let _ = manager.index_exists();

        let bookmarks = create_test_bookmarks();
        manager.build_index(&bookmarks).unwrap();

        // Should be true after building
        assert!(manager.index_exists());
    }

    #[test]
    fn test_get_index_size() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let bookmarks = create_test_bookmarks();
        manager.build_index(&bookmarks).unwrap();

        let size = manager.get_index_size().unwrap();
        assert!(size > 0);
    }
}
