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

    /// Number of unique bookmarks (for pre-built index)
    pub bookmark_count: usize,
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
        Self::for_readonly(0, 0)
    }

    /// Create status for read-only index
    pub fn for_readonly(doc_count: usize, bookmark_count: usize) -> Self {
        Self {
            total: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            is_complete: AtomicBool::new(true),
            index_type: IndexingType::ReadOnly,
            doc_count,
            bookmark_count,
        }
    }

    /// Get progress percentage (0.0 - 100.0)
    pub fn progress(&self) -> f32 {
        100.0 // Always 100% for read-only index
    }

    /// Get status summary
    pub fn summary(&self) -> String {
        if self.bookmark_count > 0 && self.bookmark_count != self.doc_count {
            format!(
                "Read-only index: {} bookmarks ({} documents)",
                self.bookmark_count, self.doc_count
            )
        } else {
            format!("Read-only index: {} documents", self.doc_count)
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
    // Use config's max_snippet_length (default: 600)
    let config = crate::config::Config::default();
    let scored_snippet =
        snippet_generator.generate_snippet(&content, query, config.max_snippet_length);

    // Extract page number from snippet (for PDF content)
    let page_number = extract_page_number_from_snippet(&scored_snippet.text, &content);

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
        page_number,
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

/// Extract page number from snippet by finding the closest [PAGE:n] marker
/// in the full content before the snippet position
pub fn extract_page_number_from_snippet(snippet: &str, full_content: &str) -> Option<usize> {
    use regex::Regex;

    // If content doesn't have page markers, return None
    if !full_content.contains("[PAGE:") {
        return None;
    }

    // Check if snippet contains a page marker directly
    let page_marker_re = Regex::new(r"\[PAGE:(\d+)\]").ok()?;
    if let Some(cap) = page_marker_re.captures(snippet) {
        if let Some(page_str) = cap.get(1) {
            if let Ok(page_num) = page_str.as_str().parse::<usize>() {
                return Some(page_num);
            }
        }
    }

    // Find the position of the snippet in the full content
    // Remove ellipsis and extract a meaningful search string
    let snippet_search = snippet
        .trim_start_matches("...")
        .trim_end_matches("...")
        .split("[PAGE:")
        .next()
        .unwrap_or(snippet)
        .trim()
        .chars()
        .take(30)
        .collect::<String>();

    if snippet_search.is_empty() || snippet_search.len() < 10 {
        return None;
    }

    let snippet_pos = full_content.find(&snippet_search)?;

    // Find all page markers before this position
    let mut last_page: Option<usize> = None;

    for cap in page_marker_re.captures_iter(&full_content[..snippet_pos]) {
        if let Some(page_str) = cap.get(1) {
            if let Ok(page_num) = page_str.as_str().parse::<usize>() {
                last_page = Some(page_num);
            }
        }
    }

    last_page
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
        let status = IndexingStatus::for_readonly(100, 10);
        assert_eq!(status.progress(), 100.0); // Always 100% for read-only
        assert_eq!(status.doc_count, 100);
        assert_eq!(status.bookmark_count, 10);
        assert_eq!(
            status.summary(),
            "Read-only index: 10 bookmarks (100 documents)"
        );
    }

    #[test]
    fn test_indexing_status_readonly_same_count() {
        // When bookmark_count == doc_count (no page splitting)
        let status = IndexingStatus::for_readonly(50, 50);
        assert_eq!(status.doc_count, 50);
        assert_eq!(status.bookmark_count, 50);
        assert_eq!(status.summary(), "Read-only index: 50 documents");
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

    #[test]
    fn test_extract_page_number_from_snippet() {
        // Test with PDF content with page markers
        let full_content = "[PAGE:1]First page content.[PAGE:2]Second page content with important info.[PAGE:3]Third page.";
        let snippet = "Second page content with important info";

        let page_num = extract_page_number_from_snippet(snippet, full_content);
        assert_eq!(page_num, Some(2));

        // Test with content at the beginning
        let snippet2 = "First page content";
        let page_num2 = extract_page_number_from_snippet(snippet2, full_content);
        assert_eq!(page_num2, Some(1));

        // Test with non-PDF content (no page markers)
        let html_content = "This is regular HTML content without page markers";
        let html_snippet = "regular HTML content";
        let page_num3 = extract_page_number_from_snippet(html_snippet, html_content);
        assert_eq!(page_num3, None);

        // Test with snippet that contains page marker directly
        let snippet4 = "[PAGE:6]Some content on page 6";
        let page_num4 = extract_page_number_from_snippet(snippet4, full_content);
        assert_eq!(page_num4, Some(6));

        // Test with truncated snippet (has ellipsis)
        let snippet5 = "...Second page content with";
        let page_num5 = extract_page_number_from_snippet(snippet5, full_content);
        assert_eq!(page_num5, Some(2));
    }
}
