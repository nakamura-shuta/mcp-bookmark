use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tantivy::{
    Index, IndexReader, TantivyDocument, Term,
    collector::TopDocs,
    directory::MmapDirectory,
    query::{
        BooleanQuery, BoostQuery, EmptyQuery, Occur, PhraseQuery, Query, QueryParser, TermQuery,
    },
    schema::Value,
};
use tracing::debug;

use super::common::{INDEX_METADATA_FILE, doc_to_result};
use super::query_parser::{QueryParser as CustomQueryParser, QueryTerm};
use super::schema::BookmarkSchema;
use super::scored_snippet::ScoredSnippetGenerator;
use super::tokenizer::register_lindera_tokenizer;

/// Unified searcher that combines all search functionality
pub struct UnifiedSearcher {
    index: Index,
    schema: BookmarkSchema,
    reader: IndexReader,
    scored_snippet_generator: ScoredSnippetGenerator,
    enable_boosting: bool,
}

impl std::fmt::Debug for UnifiedSearcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnifiedSearcher")
            .field("enable_boosting", &self.enable_boosting)
            .finish()
    }
}

impl UnifiedSearcher {
    /// Create a new searcher with read-write access
    pub fn new(index: Index, schema: BookmarkSchema) -> Result<Self> {
        // Note: Lindera tokenizer is already registered in SearchManager

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("Failed to create index reader")?;

        Ok(Self {
            index,
            schema,
            reader,
            scored_snippet_generator: ScoredSnippetGenerator::new(),
            enable_boosting: true,
        })
    }

    /// Open an existing index in read-only mode
    pub fn open_readonly<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let index_path = index_path.as_ref();

        if !index_path.join(INDEX_METADATA_FILE).exists() {
            return Err(anyhow::anyhow!("Index not found at {:?}", index_path));
        }

        let mmap_directory =
            MmapDirectory::open(index_path).context("Failed to open index directory")?;
        let index = Index::open(mmap_directory).context("Failed to open index")?;
        let schema = BookmarkSchema::new();

        // Register Lindera tokenizer for read-only index
        register_lindera_tokenizer(&index)?;

        Self::new(index, schema)
    }

    /// Reload the index reader to see new changes
    pub fn reload(&mut self) -> Result<()> {
        self.reader.reload()?;
        Ok(())
    }

    /// Main search function with optional boosting
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        debug!(
            "UnifiedSearcher::search called with query: '{}', limit: {}",
            query, limit
        );

        let searcher = self.reader.searcher();

        let parsed_query = if self.enable_boosting {
            self.create_boosted_query(query)?
        } else {
            self.create_simple_query(query)?
        };

        let top_docs = searcher
            .search(&parsed_query, &TopDocs::with_limit(limit))
            .context("Search failed")?;

        debug!("Search executed, got {} results", top_docs.len());

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            results.push(self.doc_to_result(&doc, score, query)?);
        }

        Ok(results)
    }

    /// Search with specific parameters and filters
    pub fn search_with_params(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        // Add text query
        if let Some(query_text) = &params.query {
            if !query_text.is_empty() {
                let text_query = if self.enable_boosting {
                    self.create_boosted_query(query_text)?
                } else {
                    self.create_simple_query(query_text)?
                };
                subqueries.push((Occur::Must, text_query));
            }
        }

        // Add folder filter
        if let Some(folder) = &params.folder_filter {
            let term = Term::from_field_text(self.schema.folder_path, folder);
            let folder_query: Box<dyn Query> = Box::new(TermQuery::new(
                term,
                tantivy::schema::IndexRecordOption::Basic,
            ));
            subqueries.push((Occur::Must, folder_query));
        }

        // Add domain filter
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
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            results.push(self.doc_to_result(&doc, score, query_str)?);
        }

        Ok(results)
    }

    /// Get full content by URL from index
    pub fn get_content_by_url(&self, url: &str) -> Result<Option<String>> {
        let searcher = self.reader.searcher();

        let term = Term::from_field_text(self.schema.url, url);
        let query = TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

        if let Some((_score, doc_address)) = top_docs.into_iter().next() {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(content_value) = doc.get_first(self.schema.content) {
                if let Some(content_text) = content_value.as_str() {
                    return Ok(Some(content_text.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Get index statistics
    pub fn get_stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        let segment_readers = searcher.segment_readers();

        let mut total_docs = 0;
        for segment_reader in segment_readers {
            total_docs += segment_reader.num_docs() as usize;
        }

        Ok(IndexStats {
            total_documents: total_docs,
            index_size_bytes: 0, // Can be calculated if needed
        })
    }

    /// Create a simple query without boosting (supports phrases)
    fn create_simple_query(&self, query: &str) -> Result<Box<dyn Query>> {
        // Check for empty query first
        if query.trim().is_empty() {
            // Return a query that matches nothing
            return Ok(Box::new(tantivy::query::EmptyQuery));
        }

        let terms = CustomQueryParser::parse(query);

        if terms.is_empty() {
            // If parser returns no terms, return empty query
            return Ok(Box::new(tantivy::query::EmptyQuery));
        }

        let text_fields = self.schema.text_fields();
        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        for term in terms {
            match term {
                QueryTerm::Phrase(phrase) => {
                    // Skip empty phrases
                    if phrase.trim().is_empty() {
                        continue;
                    }

                    // Create phrase query for each text field
                    let mut phrase_subqueries = Vec::new();

                    for field in &text_fields {
                        if let Ok(phrase_query) = self.create_phrase_query(*field, &phrase) {
                            phrase_subqueries.push((Occur::Should, phrase_query));
                        }
                    }

                    if !phrase_subqueries.is_empty() {
                        let combined_phrase_query = Box::new(BooleanQuery::new(phrase_subqueries));
                        subqueries.push((Occur::Must, combined_phrase_query));
                    }
                }
                QueryTerm::Word(word) => {
                    // Skip empty words
                    if word.trim().is_empty() {
                        continue;
                    }

                    // Use regular query parser for individual words
                    let query_parser = QueryParser::for_index(&self.index, text_fields.clone());
                    if let Ok(word_query) = query_parser.parse_query(&word) {
                        subqueries.push((Occur::Should, word_query));
                    }
                }
            }
        }

        if subqueries.is_empty() {
            // If all terms were empty, return empty query
            Ok(Box::new(tantivy::query::EmptyQuery))
        } else if subqueries.len() == 1 {
            Ok(subqueries.into_iter().next().unwrap().1)
        } else {
            Ok(Box::new(BooleanQuery::new(subqueries)))
        }
    }

    /// Create a phrase query for a specific field
    fn create_phrase_query(
        &self,
        field: tantivy::schema::Field,
        phrase: &str,
    ) -> Result<Box<dyn Query>> {
        // Tokenize the phrase to get individual terms
        let mut tokenizer = self
            .index
            .tokenizers()
            .get("lang_ja")
            .ok_or_else(|| anyhow::anyhow!("Tokenizer not found"))?;

        let mut token_stream = tokenizer.token_stream(phrase);
        let mut terms = Vec::new();

        while let Some(token) = token_stream.next() {
            let term = Term::from_field_text(field, &token.text);
            terms.push(term);
        }

        if terms.is_empty() {
            return Err(anyhow::anyhow!("No terms found in phrase"));
        }

        Ok(Box::new(PhraseQuery::new(terms)))
    }

    /// Create a boosted query with field-specific weights (supports phrases)
    fn create_boosted_query(&self, query: &str) -> Result<Box<dyn Query>> {
        // Check for empty query first
        if query.trim().is_empty() {
            return Ok(Box::new(EmptyQuery));
        }

        let terms = CustomQueryParser::parse(query);

        if terms.is_empty() {
            return Ok(Box::new(EmptyQuery));
        }

        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        for term in terms {
            match term {
                QueryTerm::Phrase(phrase) => {
                    // Skip empty phrases
                    if phrase.trim().is_empty() {
                        continue;
                    }

                    // Create boosted phrase queries for fields that support position indexing
                    // URL field is STRING type and doesn't support phrase queries
                    let mut phrase_field_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

                    if let Ok(title_phrase) = self.create_phrase_query(self.schema.title, &phrase) {
                        let boosted_title: Box<dyn Query> =
                            Box::new(BoostQuery::new(title_phrase, 3.0));
                        phrase_field_queries.push((Occur::Should, boosted_title));
                    }

                    if let Ok(content_phrase) =
                        self.create_phrase_query(self.schema.content, &phrase)
                    {
                        let content_query: Box<dyn Query> = content_phrase;
                        phrase_field_queries.push((Occur::Should, content_query));
                    }

                    // The phrase must be found in at least one field
                    if !phrase_field_queries.is_empty() {
                        let combined_phrase_query =
                            Box::new(BooleanQuery::new(phrase_field_queries));
                        subqueries.push((Occur::Must, combined_phrase_query));
                    }
                }
                QueryTerm::Word(word) => {
                    // Skip empty words
                    if word.trim().is_empty() {
                        continue;
                    }

                    // Title query with 3x boost
                    let title_parser = QueryParser::for_index(&self.index, vec![self.schema.title]);
                    if let Ok(title_query) = title_parser.parse_query(&word) {
                        let boosted_title_query = Box::new(BoostQuery::new(title_query, 3.0));
                        subqueries.push((Occur::Should, boosted_title_query));
                    }

                    // URL query with 2x boost
                    let url_parser = QueryParser::for_index(&self.index, vec![self.schema.url]);
                    if let Ok(url_query) = url_parser.parse_query(&word) {
                        let boosted_url_query = Box::new(BoostQuery::new(url_query, 2.0));
                        subqueries.push((Occur::Should, boosted_url_query));
                    }

                    // Content query with normal weight (1x)
                    let content_parser =
                        QueryParser::for_index(&self.index, vec![self.schema.content]);
                    if let Ok(content_query) = content_parser.parse_query(&word) {
                        subqueries.push((Occur::Should, content_query));
                    }
                }
            }
        }

        // Combine or return empty query
        if subqueries.is_empty() {
            Ok(Box::new(EmptyQuery))
        } else {
            Ok(Box::new(BooleanQuery::new(subqueries)))
        }
    }

    /// Convert document to search result
    fn doc_to_result(
        &self,
        doc: &TantivyDocument,
        score: f32,
        query: &str,
    ) -> Result<SearchResult> {
        doc_to_result(
            doc,
            &self.schema,
            score,
            query,
            &self.scored_snippet_generator,
        )
    }
}

/// Search parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParams {
    pub query: Option<String>,
    pub folder_filter: Option<String>,
    pub domain_filter: Option<String>,
    pub limit: usize,
}

impl SearchParams {
    /// Create new search params with a query
    pub fn new(query: &str) -> Self {
        Self {
            query: Some(query.to_string()),
            folder_filter: None,
            domain_filter: None,
            limit: 20,
        }
    }

    /// Set folder filter
    pub fn with_folder(mut self, folder: String) -> Self {
        self.folder_filter = Some(folder);
        self
    }

    /// Set domain filter
    pub fn with_domain(mut self, domain: String) -> Self {
        self.domain_filter = Some(domain);
        self
    }

    /// Set limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            query: None,
            folder_filter: None,
            domain_filter: None,
            limit: 20,
        }
    }
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub content: String,
    pub full_content: Option<String>,
    pub score: f32,
    pub folder_path: String,
    pub last_indexed: Option<String>,
    pub context_type: Option<String>,
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_documents: usize,
    pub index_size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::schema::BookmarkSchema;
    use crate::search::tokenizer::register_lindera_tokenizer;
    use tantivy::doc;
    use tempfile::TempDir;

    #[test]
    fn test_unified_searcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let schema = BookmarkSchema::new();
        let index = Index::create_in_dir(temp_dir.path(), schema.schema.clone()).unwrap();

        let searcher = UnifiedSearcher::new(index, schema);
        assert!(searcher.is_ok());
    }

    #[test]
    fn test_readonly_open_fails_on_missing_index() {
        let temp_dir = TempDir::new().unwrap();
        let result = UnifiedSearcher::open_readonly(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_phrase_search() {
        let temp_dir = TempDir::new().unwrap();
        let schema = BookmarkSchema::new();
        let index = Index::create_in_dir(temp_dir.path(), schema.schema.clone()).unwrap();

        // Register tokenizer
        register_lindera_tokenizer(&index).unwrap();

        // Index some test documents
        let mut index_writer = index.writer(50_000_000).unwrap();

        // Document with exact phrase "React hooks"
        index_writer.add_document(doc!(
            schema.id => "1",
            schema.title => "React hooks documentation",
            schema.url => "https://example.com/react-hooks",
            schema.content => "Learn about React hooks and how to use them in functional components.",
            schema.folder_path => "docs"
        )).unwrap();

        // Document with words but not as a phrase
        index_writer
            .add_document(doc!(
                schema.id => "2",
                schema.title => "Vue composition API",
                schema.url => "https://example.com/vue",
                schema.content => "React is great. Also, custom hooks are useful in many cases.",
                schema.folder_path => "docs"
            ))
            .unwrap();

        // Document without either word
        index_writer
            .add_document(doc!(
                schema.id => "3",
                schema.title => "Angular guide",
                schema.url => "https://example.com/angular",
                schema.content => "Angular uses components and services.",
                schema.folder_path => "docs"
            ))
            .unwrap();

        index_writer.commit().unwrap();

        // Create searcher
        let searcher = UnifiedSearcher::new(index, schema).unwrap();

        // Test phrase search
        let results = searcher.search("\"React hooks\"", 10).unwrap();
        assert_eq!(
            results.len(),
            1,
            "Should find exactly one document with the phrase 'React hooks'"
        );
        assert_eq!(results[0].id, "1");

        // Test regular word search (should find both documents with either word)
        let results = searcher.search("React hooks", 10).unwrap();
        assert!(
            results.len() >= 2,
            "Should find documents containing 'React' or 'hooks'"
        );
    }

    #[test]
    fn test_mixed_phrase_and_word_search() {
        let temp_dir = TempDir::new().unwrap();
        let schema = BookmarkSchema::new();
        let index = Index::create_in_dir(temp_dir.path(), schema.schema.clone()).unwrap();

        // Register tokenizer
        register_lindera_tokenizer(&index).unwrap();

        // Index test documents
        let mut index_writer = index.writer(50_000_000).unwrap();

        index_writer.add_document(doc!(
            schema.id => "1",
            schema.title => "React Server Components and useState",
            schema.url => "https://example.com/rsc",
            schema.content => "Learn about React Server Components and how they work with useState.",
            schema.folder_path => "docs"
        )).unwrap();

        index_writer.add_document(doc!(
            schema.id => "2",
            schema.title => "React basics",
            schema.url => "https://example.com/react",
            schema.content => "Server side rendering. Components are building blocks. No useState here.",
            schema.folder_path => "docs"
        )).unwrap();

        index_writer.commit().unwrap();

        let searcher = UnifiedSearcher::new(index, schema).unwrap();

        // Search for phrase "React Server Components" and word "useState"
        let results = searcher
            .search("\"React Server Components\" useState", 10)
            .unwrap();
        assert_eq!(
            results.len(),
            1,
            "Should find document with exact phrase and word"
        );
        assert_eq!(results[0].id, "1");
    }

    #[test]
    fn test_japanese_phrase_search() {
        let temp_dir = TempDir::new().unwrap();
        let schema = BookmarkSchema::new();
        let index = Index::create_in_dir(temp_dir.path(), schema.schema.clone()).unwrap();

        // Register tokenizer
        register_lindera_tokenizer(&index).unwrap();

        // Index Japanese documents
        let mut index_writer = index.writer(50_000_000).unwrap();

        index_writer
            .add_document(doc!(
                schema.id => "1",
                schema.title => "React フックの使い方",
                schema.url => "https://example.com/react-hooks-ja",
                schema.content => "React フックを使用して状態管理を行う方法を学びます。",
                schema.folder_path => "docs"
            ))
            .unwrap();

        index_writer
            .add_document(doc!(
                schema.id => "2",
                schema.title => "Reactの基礎",
                schema.url => "https://example.com/react-basic",
                schema.content => "Reactは素晴らしい。フックも便利です。",
                schema.folder_path => "docs"
            ))
            .unwrap();

        index_writer.commit().unwrap();

        let searcher = UnifiedSearcher::new(index, schema).unwrap();

        // Search for Japanese phrase
        let results = searcher.search("\"React フック\"", 10).unwrap();
        assert!(
            results.len() >= 1,
            "Should find documents with Japanese phrase"
        );
    }
}
