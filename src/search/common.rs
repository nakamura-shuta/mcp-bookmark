use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use tantivy::{TantivyDocument, schema::Value};

use super::schema::BookmarkSchema;
use super::scored_snippet::ScoredSnippetGenerator;
use super::unified_searcher::SearchResult;

// ============================================================================
// Constants (previously in constants.rs)
// ============================================================================

/// Default heap size for index writer (50MB)
pub const DEFAULT_WRITER_HEAP_SIZE: usize = 50_000_000;

/// Minimum heap size for tantivy 0.24 (15MB)
pub const MIN_WRITER_HEAP_SIZE: usize = 15_000_000;

/// Default index directory name
pub const DEFAULT_INDEX_NAME: &str = "default_index";

/// Index metadata file name
pub const INDEX_METADATA_FILE: &str = "meta.json";

/// Unified indexing status for all managers
#[derive(Debug)]
pub struct IndexingStatus {
    /// Total items to process
    pub total: AtomicUsize,

    /// Completed items
    pub completed: AtomicUsize,

    /// Error count
    pub errors: AtomicUsize,

    /// Completion flag
    pub is_complete: AtomicBool,

    /// Type of indexing
    pub index_type: IndexingType,

    /// Number of documents (for pre-built index)
    pub doc_count: usize,
}

/// Type of indexing operation
#[derive(Debug, Clone, PartialEq)]
pub enum IndexingType {
    /// Read-only access to existing index
    ReadOnly,
}

impl IndexingStatus {
    /// Create new status (for compatibility)
    pub fn new(_total: usize) -> Self {
        // Always returns read-only status as we only support pre-built indexes
        Self::for_readonly(0)
    }

    /// Create status for read-only index
    pub fn for_readonly(doc_count: usize) -> Self {
        Self {
            total: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            is_complete: AtomicBool::new(true),
            index_type: IndexingType::ReadOnly,
            doc_count,
        }
    }

    /// Get progress percentage (0.0 - 100.0)
    pub fn progress(&self) -> f32 {
        100.0 // Always 100% for read-only index
    }

    /// Get status summary
    pub fn summary(&self) -> String {
        format!("Read-only index: {} documents", self.doc_count)
    }
}

/// Common document to search result conversion
pub fn doc_to_result(
    doc: &TantivyDocument,
    schema: &BookmarkSchema,
    score: f32,
    query: &str,
    snippet_generator: &ScoredSnippetGenerator,
) -> Result<SearchResult> {
    let id = doc
        .get_first(schema.id)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let title = doc
        .get_first(schema.title)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let url = doc
        .get_first(schema.url)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let content = doc
        .get_first(schema.content)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let folder_path = doc
        .get_first(schema.folder_path)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Generate snippet with context detection
    // Use config's max_snippet_length (default: 600)
    let config = crate::config::Config::default();
    let scored_snippet =
        snippet_generator.generate_snippet(&content, query, config.max_snippet_length);

    Ok(SearchResult {
        id,
        title,
        url,
        snippet: scored_snippet.text,
        full_content: None, // Don't include full content in search results
        score,
        folder_path,
        last_indexed: None,
        context_type: Some(format!("{:?}", scored_snippet.context_type)),
    })
}

/// Extract domain from URL
pub fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
}

/// Parse date string to timestamp
pub fn parse_date(date: &Option<String>) -> Option<i64> {
    date.as_ref()?.parse::<i64>().ok()
}

/// Common search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonSearchConfig {
    /// Maximum results to return
    pub max_results: usize,
    /// Maximum snippet length
    pub max_snippet_length: usize,
    /// Enable query boosting
    pub enable_boosting: bool,
}

impl Default for CommonSearchConfig {
    fn default() -> Self {
        let config = crate::config::Config::default();
        Self {
            max_results: 100,
            max_snippet_length: config.max_snippet_length,
            enable_boosting: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexing_status_readonly() {
        let status = IndexingStatus::for_readonly(100);
        assert_eq!(status.progress(), 100.0); // Always 100% for read-only
        assert_eq!(status.doc_count, 100);
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("http://sub.example.com"),
            Some("sub.example.com".to_string())
        );
        assert_eq!(extract_domain("invalid-url"), None);
    }

    #[test]
    fn test_parse_date() {
        assert_eq!(
            parse_date(&Some("1234567890".to_string())),
            Some(1234567890)
        );
        assert_eq!(parse_date(&Some("invalid".to_string())), None);
        assert_eq!(parse_date(&None), None);
    }
}
