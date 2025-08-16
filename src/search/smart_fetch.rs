use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::SearchResult;

/// Smart content fetching strategy for AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFetchStrategy {
    /// Results that should have full content fetched
    pub fetch_full_content: Vec<String>, // URLs
    
    /// Reason for fetching
    pub fetch_reasons: Vec<FetchReason>,
    
    /// Estimated token usage
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FetchReason {
    HighRelevanceScore(f32),      // Score > threshold
    MultipleFieldMatch,            // Matched title + URL + content
    InsufficientSnippetContext,    // Snippets don't provide enough info
    UserQueryComplexity,           // Complex query needs more context
    TopNResults(usize),           // Always fetch top N
}

/// AI-driven content fetch decision maker
pub struct SmartFetcher {
    /// Always fetch top N results
    auto_fetch_top_n: usize,
    
    /// Score threshold for auto-fetch
    score_threshold: f32,
    
    /// Max tokens to use for content
    max_tokens: usize,
}

impl SmartFetcher {
    pub fn new() -> Self {
        Self {
            auto_fetch_top_n: 2,      // Always fetch top 2
            score_threshold: 3.0,      // Auto-fetch if score > 3.0
            max_tokens: 10000,         // ~40k characters
        }
    }
    
    /// Determine which results need full content
    pub fn determine_fetch_strategy(
        &self,
        query: &str,
        results: &[SearchResult],
    ) -> SmartFetchStrategy {
        let mut fetch_urls = Vec::new();
        let mut fetch_reasons = Vec::new();
        let mut estimated_chars = 0;
        
        // Analyze query complexity
        let query_complexity = self.analyze_query_complexity(query);
        
        for (idx, result) in results.iter().enumerate() {
            let should_fetch = self.should_fetch_content(
                result,
                idx,
                query_complexity,
                &mut fetch_reasons,
            );
            
            if should_fetch {
                fetch_urls.push(result.url.clone());
                // Estimate 5000 chars per document
                estimated_chars += 5000;
                
                // Stop if we exceed token limit
                if estimated_chars > self.max_tokens * 4 {
                    info!("Stopping fetch due to token limit");
                    break;
                }
            }
        }
        
        SmartFetchStrategy {
            fetch_full_content: fetch_urls,
            fetch_reasons,
            estimated_tokens: estimated_chars / 4, // Rough estimate
        }
    }
    
    /// Determine if content should be fetched for a result
    fn should_fetch_content(
        &self,
        result: &SearchResult,
        index: usize,
        query_complexity: QueryComplexity,
        reasons: &mut Vec<FetchReason>,
    ) -> bool {
        // 1. Always fetch top N
        if index < self.auto_fetch_top_n {
            reasons.push(FetchReason::TopNResults(index + 1));
            return true;
        }
        
        // 2. High relevance score
        if result.score > self.score_threshold {
            reasons.push(FetchReason::HighRelevanceScore(result.score));
            return true;
        }
        
        // 3. Complex query needs more context
        if matches!(query_complexity, QueryComplexity::High) {
            reasons.push(FetchReason::UserQueryComplexity);
            return true;
        }
        
        // 4. Check if snippets are insufficient
        let total_snippet_length: usize = result
            .content_snippets
            .iter()
            .map(|s| s.len())
            .sum();
            
        if total_snippet_length < 200 {
            reasons.push(FetchReason::InsufficientSnippetContext);
            return true;
        }
        
        false
    }
    
    /// Analyze query complexity
    fn analyze_query_complexity(&self, query: &str) -> QueryComplexity {
        let word_count = query.split_whitespace().count();
        
        // Check for complex patterns
        let has_operators = query.contains(" AND ") 
            || query.contains(" OR ") 
            || query.contains(" NOT ");
            
        let has_questions = query.contains("how")
            || query.contains("why")
            || query.contains("what")
            || query.contains("比較")
            || query.contains("違い");
        
        if word_count > 5 || has_operators || has_questions {
            QueryComplexity::High
        } else if word_count > 2 {
            QueryComplexity::Medium
        } else {
            QueryComplexity::Low
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum QueryComplexity {
    Low,
    Medium,
    High,
}

impl Default for SmartFetcher {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP tool response with fetch recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchWithRecommendations {
    /// Standard search results
    pub results: Vec<SearchResult>,
    
    /// Fetch recommendations for AI
    pub fetch_recommendations: FetchRecommendations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRecommendations {
    /// URLs that should be fetched for better answer
    pub recommended_urls: Vec<String>,
    
    /// Priority order (highest priority first)
    pub priority: Vec<usize>,
    
    /// Confidence that current snippets are sufficient
    pub snippet_sufficiency: f32, // 0.0 to 1.0
    
    /// Suggested processing approach
    pub processing_hint: String,
}

impl FetchRecommendations {
    pub fn new(results: &[SearchResult], query: &str) -> Self {
        let fetcher = SmartFetcher::new();
        let strategy = fetcher.determine_fetch_strategy(query, results);
        
        // Calculate snippet sufficiency
        let snippet_sufficiency = Self::calculate_sufficiency(results);
        
        // Determine processing hint
        let processing_hint = if strategy.fetch_full_content.is_empty() {
            "Snippets are sufficient for response".to_string()
        } else if strategy.fetch_full_content.len() <= 2 {
            "Fetch top results for comprehensive answer".to_string()
        } else {
            "Multiple sources needed for complete answer".to_string()
        };
        
        Self {
            recommended_urls: strategy.fetch_full_content,
            priority: (0..results.len()).collect(),
            snippet_sufficiency,
            processing_hint,
        }
    }
    
    fn calculate_sufficiency(results: &[SearchResult]) -> f32 {
        if results.is_empty() {
            return 0.0;
        }
        
        let top_score = results[0].score;
        let total_snippet_chars: usize = results
            .iter()
            .take(3)
            .flat_map(|r| &r.content_snippets)
            .map(|s| s.len())
            .sum();
        
        // High score + good snippet coverage = high sufficiency
        let score_factor = (top_score / 5.0).min(1.0);
        let snippet_factor = (total_snippet_chars as f32 / 1500.0).min(1.0);
        
        (score_factor * 0.6 + snippet_factor * 0.4).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_smart_fetch_strategy() {
        let fetcher = SmartFetcher::new();
        
        let results = vec![
            SearchResult {
                id: "1".to_string(),
                url: "https://example.com/1".to_string(),
                title: "High Score Result".to_string(),
                score: 4.5,
                content_snippets: vec!["Short snippet".to_string()],
                // ... other fields
                folder_path: "".to_string(),
                domain: "".to_string(),
                date_added: 0,
                date_modified: 0,
                content_snippet: None,
                has_full_content: true,
            },
        ];
        
        let strategy = fetcher.determine_fetch_strategy("complex query", &results);
        
        // Should fetch due to high score and top-N
        assert!(!strategy.fetch_full_content.is_empty());
        assert!(strategy.fetch_reasons.len() > 0);
    }
}