use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tantivy::{
    collector::TopDocs,
    query::{BooleanQuery, Occur, Query, QueryParser, TermQuery},
    schema::Value,
    TantivyDocument, Index, IndexReader, Term,
};

use super::schema::BookmarkSchema;

/// Handles search operations on the bookmark index
pub struct BookmarkSearcher {
    index: Index,
    schema: BookmarkSchema,
    pub reader: IndexReader,
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
        })
    }
    
    /// Reload the index reader to see new changes
    pub fn reload(&mut self) -> Result<()> {
        self.reader.reload()?;
        Ok(())
    }

    /// Simple text search across all text fields
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        
        // Create query parser for text fields
        let query_parser = QueryParser::for_index(
            &self.index,
            self.schema.text_fields(),
        );
        
        let query = query_parser
            .parse_query(query)
            .context("Failed to parse search query")?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .context("Search failed")?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            results.push(self.doc_to_result(&doc, score)?);
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
                let query_parser = QueryParser::for_index(
                    &self.index,
                    self.schema.text_fields(),
                );
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
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            results.push(self.doc_to_result(&doc, score)?);
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
            Ok(Some(self.doc_to_result(&doc, score)?))
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
        
        Ok(SearchResult {
            id,
            url,
            title,
            folder_path,
            domain,
            score,
            date_added,
            date_modified,
        })
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
}