use super::{SearchParams, SearchResult};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Metadata about a bookmark's page structure (for PDFs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkMetadata {
    pub url: String,
    pub title: String,
    pub page_count: usize,
    pub total_chars: usize,
    pub content_type: String,
    pub has_pages: bool,
}

/// Common trait for search managers
#[async_trait]
pub trait SearchManagerTrait: Send + Sync + Debug {
    /// Execute search
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;

    /// Advanced search with filters
    async fn search_advanced(&self, params: &SearchParams) -> Result<Vec<SearchResult>>;

    /// Get content by URL
    async fn get_content_by_url(&self, url: &str) -> Result<Option<String>>;

    /// Get page range content from a PDF bookmark (for single page, use start_page = end_page)
    async fn get_page_range_content(
        &self,
        url: &str,
        start_page: usize,
        end_page: usize,
    ) -> Result<Option<String>>;

    /// Get indexing status
    fn get_indexing_status(&self) -> String;

    /// Check if indexing is complete
    fn is_indexing_complete(&self) -> bool;
}
