use super::{SearchParams, SearchResult};
use anyhow::Result;
use async_trait::async_trait;
use std::fmt::Debug;

/// Common trait for search managers
#[async_trait]
pub trait SearchManagerTrait: Send + Sync + Debug {
    /// Execute search
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;

    /// Advanced search with filters
    async fn search_advanced(&self, params: &SearchParams) -> Result<Vec<SearchResult>>;

    /// Get content by URL
    async fn get_content_by_url(&self, url: &str) -> Result<Option<String>>;

    /// Get indexing status
    fn get_indexing_status(&self) -> String;

    /// Check if indexing is complete
    fn is_indexing_complete(&self) -> bool;
}
