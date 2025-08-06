pub mod indexer;
pub mod schema;
pub mod searcher;

// Re-export commonly used types
pub use searcher::{SearchParams, SearchResult};

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tantivy::{directory::MmapDirectory, Index};
use tracing::{debug, info};

use crate::bookmark::FlatBookmark;
use indexer::BookmarkIndexer;
use schema::BookmarkSchema;
use searcher::BookmarkSearcher;

/// Main search manager that coordinates indexing and searching
#[derive(Debug)]
pub struct SearchManager {
    #[allow(dead_code)]
    index: Index,
    #[allow(dead_code)]
    schema: BookmarkSchema,
    indexer: BookmarkIndexer,
    searcher: BookmarkSearcher,
    index_path: PathBuf,
}

impl SearchManager {
    /// Create a new search manager
    pub fn new(index_path: Option<PathBuf>) -> Result<Self> {
        let index_path = index_path.unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("mcp-bookmark")
                .join("index")
        });

        // Ensure directory exists
        std::fs::create_dir_all(&index_path)
            .context("Failed to create index directory")?;

        let schema = BookmarkSchema::new();
        
        // Open or create index
        let index = if index_path.join("meta.json").exists() {
            debug!("Opening existing index at {:?}", index_path);
            Index::open_in_dir(&index_path)
                .context("Failed to open existing index")?
        } else {
            info!("Creating new index at {:?}", index_path);
            let mmap_directory = MmapDirectory::open(&index_path)
                .context("Failed to open index directory")?;
            Index::create(mmap_directory, schema.schema.clone(), Default::default())
                .context("Failed to create new index")?
        };

        let indexer = BookmarkIndexer::new(index.clone(), schema.clone());
        let searcher = BookmarkSearcher::new(index.clone(), schema.clone())?;

        Ok(Self {
            index,
            schema,
            indexer,
            searcher,
            index_path,
        })
    }

    /// Build or rebuild the entire index
    pub fn build_index(&mut self, bookmarks: &[FlatBookmark]) -> Result<()> {
        self.indexer.build_index(bookmarks)?;
        // Reload searcher to see the changes
        self.searcher.reload()?;
        Ok(())
    }

    /// Update a single bookmark
    pub fn update_bookmark(
        &mut self,
        bookmark: &FlatBookmark,
        content: Option<&str>,
    ) -> Result<()> {
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
        self.searcher.search(query, limit)
    }

    /// Advanced search with filters
    pub fn search_advanced(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        self.searcher.search_with_filters(params)
    }

    /// Get bookmark by ID
    pub fn get_by_id(&self, id: &str) -> Result<Option<SearchResult>> {
        self.searcher.get_by_id(id)
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
        info!("Rebuilding entire index");
        self.indexer.build_index(bookmarks)?;
        self.searcher.reload()?;
        Ok(())
    }

    /// Check if index exists
    pub fn index_exists(&self) -> bool {
        self.index_path.join("meta.json").exists()
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
                name: "Test Site 1".to_string(),
                url: "https://example.com/1".to_string(),
                date_added: Some("1000000000000".to_string()),
                date_modified: None,
                folder_path: vec!["Bookmarks Bar".to_string()],
            },
            FlatBookmark {
                id: "2".to_string(),
                name: "Test Site 2".to_string(),
                url: "https://example.com/2".to_string(),
                date_added: Some("2000000000000".to_string()),
                date_modified: None,
                folder_path: vec!["Bookmarks Bar".to_string()],
            },
        ]
    }

    #[test]
    fn test_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");
        let _manager = SearchManager::new(Some(index_path.clone())).unwrap();
        
        // After creation, meta.json should exist
        assert!(index_path.join("meta.json").exists());
    }

    #[test]
    fn test_index_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let bookmarks = create_test_bookmarks();
        
        // Build index
        manager.build_index(&bookmarks).unwrap();
        
        // Search
        let results = manager.search("Test", 10).unwrap();
        assert_eq!(results.len(), 2);
        
        // Update bookmark
        let mut updated = bookmarks[0].clone();
        updated.name = "Updated Site".to_string();
        manager.update_bookmark(&updated, None).unwrap();
        
        // Search for updated bookmark
        let results = manager.search("Updated", 10).unwrap();
        assert_eq!(results.len(), 1);
        
        // Delete bookmark
        manager.delete_bookmark("1").unwrap();
        
        // Verify deletion
        let result = manager.get_by_id("1").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_advanced_search() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SearchManager::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let bookmarks = create_test_bookmarks();
        
        manager.build_index(&bookmarks).unwrap();
        
        // Search with parameters
        let params = SearchParams::new("Test")
            .with_domain("example.com")
            .with_limit(1);
            
        let results = manager.search_advanced(&params).unwrap();
        assert_eq!(results.len(), 1);
    }
}