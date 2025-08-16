use anyhow::{Context, Result};
use tantivy::{
    Index, IndexReader,
    collector::TopDocs,
    query::{BooleanQuery, Occur, Query, QueryParser},
    schema::Value,
};
use tracing::debug;

use super::{
    schema::BookmarkSchema,
    scored_snippet::ScoredSnippetGenerator,
    searcher::{BookmarkSearcher, SearchResult},
    snippet::SnippetGenerator,
};

/// Search booster that applies field-specific boosting
pub struct SearchBooster {
    index: Index,
    schema: BookmarkSchema,
    reader: IndexReader,
    snippet_generator: SnippetGenerator,
    scored_snippet_generator: ScoredSnippetGenerator,
}

impl std::fmt::Debug for SearchBooster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchBooster")
            .field("schema", &self.schema)
            .field("snippet_generator", &self.snippet_generator)
            .finish()
    }
}

impl SearchBooster {
    /// Create a new search booster
    pub fn new(index: Index, schema: BookmarkSchema, reader: IndexReader) -> Self {
        Self {
            index,
            schema,
            reader,
            snippet_generator: SnippetGenerator::new(),
            scored_snippet_generator: ScoredSnippetGenerator::new(),
        }
    }

    /// Search with field boosting (Phase 1.2)
    /// Title matches are weighted 3x more than content matches
    pub fn search_with_boosting(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        debug!(
            "SearchBooster::search_with_boosting called with query: '{}', limit: {}",
            query, limit
        );

        let searcher = self.reader.searcher();

        // Create separate queries for title and content with different boosts
        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        // Title query with 3x boost
        let title_parser = QueryParser::for_index(&self.index, vec![self.schema.title]);
        if let Ok(title_query) = title_parser.parse_query(query) {
            // Boost title matches by 3x
            let boosted_title_query = Box::new(tantivy::query::BoostQuery::new(title_query, 3.0));
            subqueries.push((Occur::Should, boosted_title_query));
        }

        // URL query with 2x boost (URLs are often important)
        let url_parser = QueryParser::for_index(&self.index, vec![self.schema.url]);
        if let Ok(url_query) = url_parser.parse_query(query) {
            let boosted_url_query = Box::new(tantivy::query::BoostQuery::new(url_query, 2.0));
            subqueries.push((Occur::Should, boosted_url_query));
        }

        // Content query with normal weight (1x)
        let content_parser = QueryParser::for_index(&self.index, vec![self.schema.content]);
        if let Ok(content_query) = content_parser.parse_query(query) {
            subqueries.push((Occur::Should, content_query));
        }

        // Combine all queries
        let combined_query: Box<dyn Query> = if subqueries.is_empty() {
            // Fallback to simple search if no specific field queries work
            let all_fields = self.schema.text_fields();
            let parser = QueryParser::for_index(&self.index, all_fields);
            parser.parse_query(query).context("Failed to parse query")?
        } else {
            Box::new(BooleanQuery::new(subqueries))
        };

        // Execute search
        let top_docs = searcher
            .search(&combined_query, &TopDocs::with_limit(limit))
            .context("Boosted search failed")?;

        debug!("Boosted search executed, got {} results", top_docs.len());

        // Convert results
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            let result = BookmarkSearcher::doc_to_result_with_boosting(
                &self.schema,
                &self.snippet_generator,
                &self.scored_snippet_generator,
                &doc,
                score,
                query,
            )?;
            results.push(result);
        }

        // Sort by score (highest first) to ensure title matches appear first
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        Ok(results)
    }
}

impl BookmarkSearcher {
    /// Helper method to convert document to result with boosting info
    pub fn doc_to_result_with_boosting(
        schema: &BookmarkSchema,
        snippet_generator: &SnippetGenerator,
        scored_snippet_generator: &ScoredSnippetGenerator,
        doc: &tantivy::TantivyDocument,
        score: f32,
        query: &str,
    ) -> Result<SearchResult> {
        // Extract fields
        let id = doc
            .get_first(schema.id)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let url = doc
            .get_first(schema.url)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let title = doc
            .get_first(schema.title)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let folder_path = doc
            .get_first(schema.folder_path)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let domain = doc
            .get_first(schema.domain)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let date_added = doc
            .get_first(schema.date_added)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let date_modified = doc
            .get_first(schema.date_modified)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // Get content and generate snippets
        let content = doc
            .get_first(schema.content)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let has_full_content = content.is_some();

        let (content_snippet, content_snippets, scored_snippets) =
            if let Some(content_text) = &content {
                let snippets = snippet_generator.generate_snippets(content_text, query);
                let scored = scored_snippet_generator.generate_scored_snippets(content_text, query);
                (snippets.first().cloned(), snippets, scored)
            } else {
                (None, Vec::new(), Vec::new())
            };

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
            scored_snippets,
            has_full_content,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark::FlatBookmark;
    use crate::search::indexer::BookmarkIndexer;
    use tantivy::directory::MmapDirectory;
    use tempfile::TempDir;

    fn setup_test_index() -> (SearchBooster, BookmarkIndexer, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let schema = BookmarkSchema::new();
        let dir = MmapDirectory::open(temp_dir.path()).unwrap();
        let index = Index::create(dir, schema.schema.clone(), Default::default()).unwrap();

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .unwrap();

        let booster = SearchBooster::new(index.clone(), schema.clone(), reader);
        let indexer = BookmarkIndexer::new(index, schema);

        (booster, indexer, temp_dir)
    }

    #[test]
    fn test_title_boosting() {
        let (booster, indexer, _temp) = setup_test_index();

        // Create test bookmarks
        let bookmarks = vec![
            FlatBookmark {
                id: "1".to_string(),
                name: "Rust Programming Language".to_string(), // Title match
                url: "https://www.rust-lang.org/".to_string(),
                date_added: Some("1000000000000".to_string()),
                date_modified: None,
                folder_path: vec!["Tech".to_string()],
            },
            FlatBookmark {
                id: "2".to_string(),
                name: "Generic Tech Blog".to_string(),
                url: "https://blog.example.com/rust-article".to_string(), // URL match
                date_added: Some("2000000000000".to_string()),
                date_modified: None,
                folder_path: vec!["Tech".to_string()],
            },
        ];

        // Index bookmarks with content
        indexer.build_index(&bookmarks).unwrap();
        indexer
            .update_bookmark(
                &bookmarks[0],
                Some("This page has some general programming content"),
            )
            .unwrap();
        indexer
            .update_bookmark(
                &bookmarks[1],
                Some("Deep dive into Rust programming patterns and best practices"),
            )
            .unwrap();

        // Search for "rust"
        let results = booster.search_with_boosting("rust", 10).unwrap();

        // Title match should score higher than content match
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "1"); // Title match should be first
        assert!(results[0].score > results[1].score); // Title match should have higher score
    }
}
