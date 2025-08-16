use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

use super::SearchResult;

/// Search result aggregator for AI processing
/// Combines multiple matches and provides comprehensive context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedSearchResult {
    /// Primary search results (top scoring)
    pub primary_results: Vec<SearchResult>,
    
    /// Additional context from lower-scoring matches
    pub supplementary_results: Vec<SearchResult>,
    
    /// Query metadata
    pub query_metadata: QueryMetadata,
    
    /// Content summary for AI processing
    pub content_summary: ContentSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    pub original_query: String,
    pub total_matches: usize,
    pub returned_primary: usize,
    pub returned_supplementary: usize,
    pub search_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSummary {
    /// Combined relevant snippets from all results
    pub combined_snippets: Vec<String>,
    
    /// Key topics found across results
    pub common_topics: Vec<String>,
    
    /// Domains represented in results
    pub domains: Vec<String>,
    
    /// Total content size available
    pub total_content_chars: usize,
}

pub struct SearchAggregator {
    /// Maximum primary results to return
    max_primary: usize,
    
    /// Maximum supplementary results to include
    max_supplementary: usize,
    
    /// Maximum total content size (characters)
    max_content_size: usize,
}

impl SearchAggregator {
    pub fn new() -> Self {
        Self {
            max_primary: 5,        // Top 5 most relevant
            max_supplementary: 10, // Next 10 for context
            max_content_size: 50000, // ~12.5k tokens for AI
        }
    }
    
    /// Aggregate search results for AI processing
    pub fn aggregate(
        &self,
        query: &str,
        mut results: Vec<SearchResult>,
    ) -> Result<AggregatedSearchResult> {
        debug!(
            "Aggregating {} search results for query: '{}'",
            results.len(),
            query
        );
        
        // Sort by score (highest first)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        // Split into primary and supplementary
        let primary_results: Vec<SearchResult> = results
            .iter()
            .take(self.max_primary)
            .cloned()
            .collect();
            
        let supplementary_results: Vec<SearchResult> = results
            .iter()
            .skip(self.max_primary)
            .take(self.max_supplementary)
            .cloned()
            .collect();
        
        // Build content summary
        let content_summary = self.build_content_summary(&results);
        
        // Create metadata
        let query_metadata = QueryMetadata {
            original_query: query.to_string(),
            total_matches: results.len(),
            returned_primary: primary_results.len(),
            returned_supplementary: supplementary_results.len(),
            search_strategy: "field_boosting_v1.2".to_string(),
        };
        
        Ok(AggregatedSearchResult {
            primary_results,
            supplementary_results,
            query_metadata,
            content_summary,
        })
    }
    
    /// Build content summary for AI processing
    fn build_content_summary(&self, results: &[SearchResult]) -> ContentSummary {
        let mut combined_snippets = Vec::new();
        let mut domains = Vec::new();
        let mut total_chars = 0;
        
        // Collect unique snippets and domains
        let mut seen_snippets = HashMap::new();
        
        for result in results.iter().take(self.max_primary + self.max_supplementary) {
            // Add domain if not seen
            if !domains.contains(&result.domain) {
                domains.push(result.domain.clone());
            }
            
            // Add unique snippets
            for snippet in &result.content_snippets {
                let snippet_key = snippet.chars().take(50).collect::<String>();
                if !seen_snippets.contains_key(&snippet_key) {
                    seen_snippets.insert(snippet_key, true);
                    combined_snippets.push(snippet.clone());
                    total_chars += snippet.len();
                    
                    // Stop if we exceed max content size
                    if total_chars > self.max_content_size {
                        break;
                    }
                }
            }
            
            if total_chars > self.max_content_size {
                break;
            }
        }
        
        // Extract common topics (simple keyword extraction)
        let common_topics = self.extract_common_topics(results);
        
        ContentSummary {
            combined_snippets,
            common_topics,
            domains,
            total_content_chars: total_chars,
        }
    }
    
    /// Extract common topics from results
    fn extract_common_topics(&self, results: &[SearchResult]) -> Vec<String> {
        // Simple implementation - can be enhanced with better NLP
        let mut topic_counts: HashMap<String, usize> = HashMap::new();
        
        for result in results.iter().take(10) {
            // Extract words from title
            for word in result.title.split_whitespace() {
                let word_lower = word.to_lowercase();
                if word_lower.len() > 3 { // Skip short words
                    *topic_counts.entry(word_lower).or_insert(0) += 1;
                }
            }
        }
        
        // Sort by frequency and return top topics
        let mut topics: Vec<_> = topic_counts.into_iter().collect();
        topics.sort_by(|a, b| b.1.cmp(&a.1));
        
        topics
            .into_iter()
            .take(5)
            .map(|(topic, _)| topic)
            .collect()
    }
}

impl Default for SearchAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Extended search result for AI with full content access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISearchResult {
    /// Aggregated search results
    pub aggregated: AggregatedSearchResult,
    
    /// Full content for top results (if requested)
    pub full_contents: Vec<FullContent>,
    
    /// Processing hints for AI
    pub ai_hints: AIProcessingHints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullContent {
    pub url: String,
    pub title: String,
    pub content: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIProcessingHints {
    /// Suggested focus areas based on query
    pub focus_areas: Vec<String>,
    
    /// Whether results seem to answer the query
    pub high_confidence_match: bool,
    
    /// Suggested response strategy
    pub response_strategy: String,
}

impl AISearchResult {
    /// Create AI-optimized search result
    pub fn new(
        aggregated: AggregatedSearchResult,
        full_contents: Vec<FullContent>,
    ) -> Self {
        let ai_hints = Self::generate_hints(&aggregated);
        
        Self {
            aggregated,
            full_contents,
            ai_hints,
        }
    }
    
    /// Generate AI processing hints
    fn generate_hints(aggregated: &AggregatedSearchResult) -> AIProcessingHints {
        let high_confidence_match = aggregated
            .primary_results
            .first()
            .map(|r| r.score > 2.0)
            .unwrap_or(false);
        
        let response_strategy = if high_confidence_match {
            "direct_answer".to_string()
        } else if aggregated.primary_results.len() > 3 {
            "synthesize_multiple".to_string()
        } else {
            "exploratory".to_string()
        };
        
        let focus_areas = aggregated
            .content_summary
            .common_topics
            .iter()
            .take(3)
            .cloned()
            .collect();
        
        AIProcessingHints {
            focus_areas,
            high_confidence_match,
            response_strategy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_results() -> Vec<SearchResult> {
        vec![
            SearchResult {
                id: "1".to_string(),
                url: "https://example.com/rds-guide".to_string(),
                title: "RDS Configuration Guide".to_string(),
                folder_path: "Tech".to_string(),
                domain: "example.com".to_string(),
                score: 4.5,
                date_added: 0,
                date_modified: 0,
                content_snippet: Some("RDS setup...".to_string()),
                content_snippets: vec!["RDS setup guide...".to_string()],
                has_full_content: true,
            },
            SearchResult {
                id: "2".to_string(),
                url: "https://aws.amazon.com/rds".to_string(),
                title: "Amazon RDS".to_string(),
                folder_path: "AWS".to_string(),
                domain: "aws.amazon.com".to_string(),
                score: 3.2,
                date_added: 0,
                date_modified: 0,
                content_snippet: Some("Managed database...".to_string()),
                content_snippets: vec!["Managed database service...".to_string()],
                has_full_content: true,
            },
        ]
    }
    
    #[test]
    fn test_aggregation() {
        let aggregator = SearchAggregator::new();
        let results = create_test_results();
        
        let aggregated = aggregator.aggregate("RDS", results).unwrap();
        
        assert_eq!(aggregated.primary_results.len(), 2);
        assert_eq!(aggregated.query_metadata.original_query, "RDS");
        assert_eq!(aggregated.content_summary.domains.len(), 2);
    }
}