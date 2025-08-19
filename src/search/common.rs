use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

/// MCP-optimized search configuration constants
pub mod mcp {
    /// Maximum snippets per result for MCP
    pub const MAX_SNIPPETS_PER_RESULT: usize = 2;
    /// Maximum snippet length for MCP
    pub const MAX_SNIPPET_LENGTH: usize = 300;
    /// Maximum total results
    pub const MAX_RESULTS: usize = 10;
}

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

    /// Start time
    pub started_at: std::time::Instant,

    /// Type of indexing
    pub index_type: IndexingType,

    /// Number of documents (for pre-built index)
    pub doc_count: usize,
}

/// Type of indexing operation
#[derive(Debug, Clone, PartialEq)]
pub enum IndexingType {
    /// Full indexing from bookmarks
    FullIndex,
    /// Using pre-built index from Chrome extension
    PreBuilt,
    /// Read-only access to existing index
    ReadOnly,
}

impl IndexingStatus {
    /// Create new status for full indexing
    pub fn new(total: usize) -> Self {
        Self {
            total: AtomicUsize::new(total),
            completed: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            is_complete: AtomicBool::new(false),
            started_at: std::time::Instant::now(),
            index_type: IndexingType::FullIndex,
            doc_count: 0,
        }
    }

    /// Create status for pre-built index
    pub fn for_prebuilt(doc_count: usize) -> Self {
        Self {
            total: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            is_complete: AtomicBool::new(true),
            started_at: std::time::Instant::now(),
            index_type: IndexingType::PreBuilt,
            doc_count,
        }
    }

    /// Create status for read-only index
    pub fn for_readonly(doc_count: usize) -> Self {
        Self {
            total: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            is_complete: AtomicBool::new(true),
            started_at: std::time::Instant::now(),
            index_type: IndexingType::ReadOnly,
            doc_count,
        }
    }

    /// Get progress percentage (0.0 - 100.0)
    pub fn progress(&self) -> f32 {
        match self.index_type {
            IndexingType::PreBuilt | IndexingType::ReadOnly => 100.0,
            IndexingType::FullIndex => {
                let total = self.total.load(Ordering::Relaxed);
                if total == 0 {
                    0.0
                } else {
                    let completed = self.completed.load(Ordering::Relaxed);
                    (completed as f32 / total as f32) * 100.0
                }
            }
        }
    }

    /// Mark as complete
    pub fn mark_complete(&self) {
        self.is_complete.store(true, Ordering::Relaxed);
    }

    /// Increment completed count
    pub fn increment_completed(&self) {
        self.completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment error count
    pub fn increment_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get status summary
    pub fn summary(&self) -> String {
        match self.index_type {
            IndexingType::PreBuilt => {
                format!("Using pre-built index: {} documents", self.doc_count)
            }
            IndexingType::ReadOnly => {
                format!("Read-only index: {} documents", self.doc_count)
            }
            IndexingType::FullIndex => {
                let total = self.total.load(Ordering::Relaxed);
                let completed = self.completed.load(Ordering::Relaxed);
                let errors = self.errors.load(Ordering::Relaxed);
                format!(
                    "Indexing: {}/{} completed, {} errors ({:.1}%)",
                    completed,
                    total,
                    errors,
                    self.progress()
                )
            }
        }
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
    let scored_snippet = snippet_generator.generate_snippet(&content, query, 300);

    Ok(SearchResult {
        id,
        title,
        url,
        snippet: scored_snippet.text.clone(),
        content: scored_snippet.text, // Use snippet for preview
        full_content: None,           // Don't include full content in search results
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
        Self {
            max_results: 100,
            max_snippet_length: 300,
            enable_boosting: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexing_status_progress() {
        let status = IndexingStatus::new(100);
        assert_eq!(status.progress(), 0.0);

        status.completed.store(50, Ordering::Relaxed);
        assert_eq!(status.progress(), 50.0);

        status.completed.store(100, Ordering::Relaxed);
        assert_eq!(status.progress(), 100.0);
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
