use anyhow::{Context, Result};
use std::path::Path;
use tantivy::{Index, directory::MmapDirectory};

use super::{BookmarkSchema, SearchResult, boosting::SearchBooster, searcher::BookmarkSearcher};

/// Read-only searcher for Chrome extension indexes
/// This doesn't use any locks and allows multiple processes to access the same index
pub struct ReadOnlySearcher {
    searcher: BookmarkSearcher,
    booster: Option<SearchBooster>,
}

impl std::fmt::Debug for ReadOnlySearcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadOnlySearcher")
            .field("searcher", &"BookmarkSearcher")
            .finish()
    }
}

impl ReadOnlySearcher {
    /// Open an existing index in read-only mode
    pub fn open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let index_path = index_path.as_ref();

        // Check if index exists
        if !index_path.join("meta.json").exists() {
            return Err(anyhow::anyhow!("Index not found at {:?}", index_path));
        }

        // Open index in read-only mode
        let mmap_directory =
            MmapDirectory::open(index_path).context("Failed to open index directory")?;

        let index = Index::open(mmap_directory).context("Failed to open index")?;

        let schema = BookmarkSchema::new();

        // Create a custom BookmarkSearcher with read-only settings
        let searcher = BookmarkSearcher::new(index.clone(), schema.clone())?;

        // Create booster with the same index and reader
        let booster = SearchBooster::new(index, schema, searcher.reader.clone());

        Ok(Self {
            searcher,
            booster: Some(booster),
        })
    }

    /// Search the index (no locks, thread-safe)
    /// Uses field boosting for better relevance (Phase 1.2)
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Use booster if available for improved relevance
        if let Some(booster) = &self.booster {
            booster.search_with_boosting(query, limit)
        } else {
            self.searcher.search(query, limit)
        }
    }

    /// Get content by URL
    pub fn get_content_by_url(&self, url: &str) -> Result<Option<String>> {
        self.searcher.get_full_content_by_url(url)
    }

    /// Get index statistics
    pub fn get_stats(&self) -> Result<super::searcher::IndexStats> {
        self.searcher.get_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_readonly_fails_on_missing_index() {
        let temp_dir = TempDir::new().unwrap();
        let result = ReadOnlySearcher::open(temp_dir.path());
        assert!(result.is_err());
    }
}
