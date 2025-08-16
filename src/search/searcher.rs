use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tantivy::{
    Index, IndexReader, TantivyDocument, Term,
    collector::TopDocs,
    query::{BooleanQuery, Occur, Query, QueryParser, TermQuery},
    schema::Value,
};

use super::schema::BookmarkSchema;
use super::snippet::SnippetGenerator;

/// Handles search operations on the bookmark index
pub struct BookmarkSearcher {
    index: Index,
    schema: BookmarkSchema,
    pub reader: IndexReader,
    snippet_generator: SnippetGenerator,
}

impl std::fmt::Debug for BookmarkSearcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BookmarkSearcher")
            .field("index", &"Index")
            .field("schema", &self.schema)
            .finish()
    }
}

impl BookmarkSearcher {
    /// Create a new searcher
    pub fn new(index: Index, schema: BookmarkSchema) -> Result<Self> {
        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("Failed to create index reader")?;

        Ok(Self {
            index,
            schema,
            reader,
            snippet_generator: SnippetGenerator::new(),
        })
    }

    /// Reload the index reader to see new changes
    pub fn reload(&mut self) -> Result<()> {
        self.reader.reload()?;
        Ok(())
    }

    /// Simple text search across all text fields
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        tracing::debug!(
            "BookmarkSearcher::search called with query: '{}', limit: {}",
            query,
            limit
        );
        let searcher = self.reader.searcher();
        tracing::debug!("Got searcher");

        // Create query parser for text fields
        let text_fields = self.schema.text_fields();
        tracing::debug!("Got text fields: {} fields", text_fields.len());
        let query_parser = QueryParser::for_index(&self.index, text_fields);
        tracing::debug!("Created query parser");

        let parsed_query = query_parser
            .parse_query(query)
            .context("Failed to parse search query")?;
        tracing::debug!("Parsed query successfully");

        let top_docs = searcher
            .search(&parsed_query, &TopDocs::with_limit(limit))
            .context("Search failed")?;
        tracing::debug!("Search executed, got {} results", top_docs.len());

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            // Generate results with snippets
            results.push(self.doc_to_result_with_snippet(&doc, score, query)?);
        }

        Ok(results)
    }

    /// Search only in content field
    pub fn search_content_only(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        // Create query parser for content field only
        let query_parser = QueryParser::for_index(&self.index, vec![self.schema.content]);

        let parsed_query = query_parser
            .parse_query(query)
            .context("Failed to parse content search query")?;

        let top_docs = searcher
            .search(&parsed_query, &TopDocs::with_limit(limit))
            .context("Content search failed")?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            let mut result = self.doc_to_result_with_snippet(&doc, score, query)?;
            // Boost score for content-only search to differentiate it
            result.score = score * 1.5;
            results.push(result);
        }

        Ok(results)
    }

    /// Advanced search with filters
    pub fn search_with_filters(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        // Text query
        if let Some(query_text) = &params.query {
            if !query_text.is_empty() {
                let query_parser = QueryParser::for_index(&self.index, self.schema.text_fields());
                let text_query = query_parser.parse_query(query_text)?;
                subqueries.push((Occur::Must, text_query));
            }
        }

        // Folder filter
        if let Some(folder) = &params.folder_filter {
            let term = Term::from_field_text(self.schema.folder_path, folder);
            let folder_query: Box<dyn Query> = Box::new(TermQuery::new(
                term,
                tantivy::schema::IndexRecordOption::Basic,
            ));
            subqueries.push((Occur::Must, folder_query));
        }

        // Domain filter
        if let Some(domain) = &params.domain_filter {
            let term = Term::from_field_text(self.schema.domain, domain);
            let domain_query: Box<dyn Query> = Box::new(TermQuery::new(
                term,
                tantivy::schema::IndexRecordOption::Basic,
            ));
            subqueries.push((Occur::Must, domain_query));
        }

        // Build final query
        let query: Box<dyn Query> = if subqueries.is_empty() {
            Box::new(tantivy::query::AllQuery)
        } else if subqueries.len() == 1 {
            subqueries.into_iter().next().unwrap().1
        } else {
            Box::new(BooleanQuery::new(subqueries))
        };

        let top_docs = searcher.search(&query, &TopDocs::with_limit(params.limit))?;

        let mut results = Vec::new();
        let query_str = params.query.as_deref().unwrap_or("");
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            results.push(self.doc_to_result_with_snippet(&doc, score, query_str)?);
        }

        Ok(results)
    }

    /// Get bookmark by ID
    pub fn get_by_id(&self, id: &str) -> Result<Option<SearchResult>> {
        let searcher = self.reader.searcher();

        let term = Term::from_field_text(self.schema.id, id);
        let query = TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

        if let Some((score, doc_address)) = top_docs.into_iter().next() {
            let doc = searcher.doc(doc_address)?;
            // No snippet needed when getting by ID (for full text retrieval)
            Ok(Some(self.doc_to_result(&doc, score)?))
        } else {
            Ok(None)
        }
    }

    /// Get full content by URL from index
    pub fn get_full_content_by_url(&self, url: &str) -> Result<Option<String>> {
        let searcher = self.reader.searcher();

        // Exact match search on URL field (now STRING field)
        let term = Term::from_field_text(self.schema.url, url);
        let query = TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

        if let Some((_, doc_address)) = top_docs.into_iter().next() {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            // Get full text from content field
            let content = doc
                .get_first(self.schema.content)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            Ok(content)
        } else {
            Ok(None)
        }
    }

    /// Get index statistics
    pub fn get_stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        let num_docs = searcher.num_docs();

        Ok(IndexStats {
            num_documents: num_docs as usize,
        })
    }

    /// Convert document to search result
    fn doc_to_result(&self, doc: &TantivyDocument, score: f32) -> Result<SearchResult> {
        let id = self.get_text_field(doc, self.schema.id)?;
        let url = self.get_text_field(doc, self.schema.url)?;
        let title = self.get_text_field(doc, self.schema.title)?;
        let folder_path = self.get_text_field(doc, self.schema.folder_path)?;
        let domain = self.get_text_field(doc, self.schema.domain)?;

        let date_added = doc
            .get_first(self.schema.date_added)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let date_modified = doc
            .get_first(self.schema.date_modified)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // Get content field (check if exists)
        let content = doc
            .get_first(self.schema.content)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let has_full_content = content.is_some();
        // Snippet will be generated later based on search query
        let content_snippet = None;
        let content_snippets = Vec::new();

        Ok(SearchResult {
            id,
            url,
            title,
            folder_path,
            domain,
            score,
            date_added,
            date_modified,
            content_snippet,
            content_snippets,
            has_full_content,
        })
    }

    /// Convert document to search result with content snippet
    fn doc_to_result_with_snippet(
        &self,
        doc: &TantivyDocument,
        score: f32,
        query: &str,
    ) -> Result<SearchResult> {
        let mut result = self.doc_to_result(doc, score)?;

        // Generate snippets from content using the improved snippet generator
        if let Some(content_value) = doc.get_first(self.schema.content) {
            if let Some(content_text) = content_value.as_str() {
                // Generate multiple snippets with sentence boundary awareness
                let snippets = self
                    .snippet_generator
                    .generate_snippets(content_text, query);

                // Store multiple snippets (Phase 1.1 improvement)
                result.content_snippets = snippets.clone();

                // Keep backward compatibility - store first snippet in old field
                result.content_snippet = snippets.first().cloned();
            }
        }

        Ok(result)
    }

    /// Generate content snippet around search terms
    fn generate_snippet(&self, content: &str, query: &str) -> Option<String> {
        if content.is_empty() {
            return None;
        }

        // Split query into words (simple implementation)
        let query_terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if query_terms.is_empty() {
            // Return first 200 characters if no query
            let snippet = if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.to_string()
            };
            return Some(snippet);
        }

        let content_lower = content.to_lowercase();

        // Find first matching position
        for term in &query_terms {
            if let Some(pos) = content_lower.find(term) {
                // Get 100 characters before and after match
                let start = pos.saturating_sub(100);
                let end = std::cmp::min(pos + term.len() + 100, content.len());

                // Safe extraction considering UTF-8 boundaries
                let mut start_byte = start;
                while start_byte > 0 && !content.is_char_boundary(start_byte) {
                    start_byte -= 1;
                }

                let mut end_byte = end;
                while end_byte < content.len() && !content.is_char_boundary(end_byte) {
                    end_byte += 1;
                }

                let snippet = &content[start_byte..end_byte];

                // Add ellipsis before and after
                let formatted = if start_byte > 0 && end_byte < content.len() {
                    format!("...{snippet}...")
                } else if start_byte > 0 {
                    format!("...{snippet}")
                } else if end_byte < content.len() {
                    format!("{snippet}...")
                } else {
                    snippet.to_string()
                };

                return Some(formatted);
            }
        }

        // First 200 characters if no match
        let snippet = if content.len() > 200 {
            format!("{}...", &content[..200])
        } else {
            content.to_string()
        };
        Some(snippet)
    }

    /// Helper to extract text field
    fn get_text_field(
        &self,
        doc: &TantivyDocument,
        field: tantivy::schema::Field,
    ) -> Result<String> {
        doc.get_first(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Field not found"))
    }
}

/// Search parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchParams {
    pub query: Option<String>,
    pub folder_filter: Option<String>,
    pub domain_filter: Option<String>,
    pub date_range: Option<DateRange>,
    pub limit: usize,
}

impl SearchParams {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: Some(query.into()),
            limit: 20,
            ..Default::default()
        }
    }

    pub fn with_folder(mut self, folder: impl Into<String>) -> Self {
        self.folder_filter = Some(folder.into());
        self
    }

    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain_filter = Some(domain.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// Date range for filtering
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DateRange {
    pub start: i64,
    pub end: i64,
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub url: String,
    pub title: String,
    pub folder_path: String,
    pub domain: String,
    pub score: f32,
    pub date_added: i64,
    pub date_modified: i64,
    pub content_snippet: Option<String>, // Excerpt from search hit (backward compatibility)
    pub content_snippets: Vec<String>,   // Multiple relevant snippets (Phase 1.1 improvement)
    pub has_full_content: bool,          // Whether content exists in index
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub num_documents: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark::FlatBookmark;
    use crate::search::indexer::BookmarkIndexer;
    use tantivy::directory::MmapDirectory;
    use tempfile::TempDir;

    fn setup_test_index() -> (BookmarkSearcher, BookmarkIndexer, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let schema = BookmarkSchema::new();
        let dir = MmapDirectory::open(temp_dir.path()).unwrap();
        let index = Index::create(dir, schema.schema.clone(), Default::default()).unwrap();

        let indexer = BookmarkIndexer::new(index.clone(), schema.clone());
        let searcher = BookmarkSearcher::new(index, schema).unwrap();

        (searcher, indexer, temp_dir)
    }

    fn create_test_bookmarks() -> Vec<FlatBookmark> {
        vec![
            FlatBookmark {
                id: "1".to_string(),
                name: "Rust Programming Language".to_string(),
                url: "https://www.rust-lang.org/".to_string(),
                date_added: Some("1000000000000".to_string()),
                date_modified: None,
                folder_path: vec!["Bookmarks Bar".to_string(), "Tech".to_string()],
            },
            FlatBookmark {
                id: "2".to_string(),
                name: "Tantivy Documentation".to_string(),
                url: "https://docs.rs/tantivy".to_string(),
                date_added: Some("2000000000000".to_string()),
                date_modified: None,
                folder_path: vec!["Bookmarks Bar".to_string(), "Tech".to_string()],
            },
            FlatBookmark {
                id: "3".to_string(),
                name: "Example Blog".to_string(),
                url: "https://example.com/blog".to_string(),
                date_added: Some("3000000000000".to_string()),
                date_modified: None,
                folder_path: vec!["Bookmarks Bar".to_string(), "Personal".to_string()],
            },
        ]
    }

    #[test]
    fn test_simple_search() {
        let (mut searcher, indexer, _temp) = setup_test_index();
        let bookmarks = create_test_bookmarks();

        // Index bookmarks
        indexer.build_index(&bookmarks).unwrap();

        // Reload the searcher to see committed changes
        searcher.reload().unwrap();

        // Search for "rust"
        let results = searcher.search("rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("Rust"));

        // Search for "documentation"
        let results = searcher.search("documentation", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("Tantivy"));
    }

    #[test]
    fn test_search_with_folder_filter() {
        let (mut searcher, indexer, _temp) = setup_test_index();
        let bookmarks = create_test_bookmarks();

        indexer.build_index(&bookmarks).unwrap();
        searcher.reload().unwrap();

        // Search in Tech folder
        let params = SearchParams::new("").with_folder("Bookmarks Bar/Tech");
        let results = searcher.search_with_filters(&params).unwrap();
        assert_eq!(results.len(), 2);

        // Search in Personal folder
        let params = SearchParams::new("").with_folder("Bookmarks Bar/Personal");
        let results = searcher.search_with_filters(&params).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_with_domain_filter() {
        let (mut searcher, indexer, _temp) = setup_test_index();
        let bookmarks = create_test_bookmarks();

        indexer.build_index(&bookmarks).unwrap();
        searcher.reload().unwrap();

        // Search for docs.rs domain
        let params = SearchParams::new("").with_domain("docs.rs");
        let results = searcher.search_with_filters(&params).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].url.contains("docs.rs"));
    }

    #[test]
    fn test_get_by_id() {
        let (mut searcher, indexer, _temp) = setup_test_index();
        let bookmarks = create_test_bookmarks();

        indexer.build_index(&bookmarks).unwrap();
        searcher.reload().unwrap();

        // Get bookmark by ID
        let result = searcher.get_by_id("2").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "2");

        // Try non-existent ID
        let result = searcher.get_by_id("999").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_content_only_search() {
        let (mut searcher, indexer, _temp) = setup_test_index();

        // Create bookmarks with content
        let bookmarks = vec![FlatBookmark {
            id: "1".to_string(),
            name: "Different Title".to_string(),
            url: "https://www.example.com/".to_string(),
            date_added: Some("1000000000000".to_string()),
            date_modified: None,
            folder_path: vec!["Bookmarks Bar".to_string()],
        }];

        // Index bookmarks
        indexer.build_index(&bookmarks).unwrap();

        // Add content for the bookmark
        indexer
            .update_bookmark(&bookmarks[0], Some("This page contains Rust information"))
            .unwrap();

        // Reload searcher
        searcher.reload().unwrap();

        // Search for content that only exists in content field
        let results = searcher
            .search_content_only("Rust information", 10)
            .unwrap();
        assert_eq!(results.len(), 1);

        // Search for title text should not find results in content-only search
        let results = searcher.search_content_only("Different Title", 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
