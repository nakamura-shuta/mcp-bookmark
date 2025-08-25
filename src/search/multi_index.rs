use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use tracing::{info, warn};

use crate::config::Config;
use crate::search::{SearchManager, SearchParams, SearchResult};
use crate::search::search_manager_trait::SearchManagerTrait;

/// Multi-index search manager for searching across multiple indices
#[derive(Debug)]
pub struct MultiIndexSearchManager {
    managers: Vec<SearchManager>,
    index_names: Vec<String>,
}

impl MultiIndexSearchManager {
    /// Create a new multi-index search manager
    pub fn new(config: &Config) -> Result<Self> {
        let index_names = config.parse_index_names();
        
        if index_names.is_empty() {
            anyhow::bail!("No index names provided");
        }
        
        info!("Initializing multi-index search with {} indices", index_names.len());
        
        let mut managers = Vec::new();
        let mut failed_indices = Vec::new();
        
        for name in &index_names {
            info!("Loading index: {}", name);
            match SearchManager::open_readonly(name) {
                Ok(manager) => {
                    info!("Successfully loaded index: {}", name);
                    managers.push(manager);
                }
                Err(e) => {
                    warn!("Failed to load index '{}': {}", name, e);
                    failed_indices.push(name.clone());
                }
            }
        }
        
        if managers.is_empty() {
            anyhow::bail!(
                "Failed to load any indices. Failed indices: {:?}", 
                failed_indices
            );
        }
        
        if !failed_indices.is_empty() {
            warn!(
                "Some indices could not be loaded: {:?}. Continuing with {} available indices.",
                failed_indices,
                managers.len()
            );
        }
        
        Ok(Self { 
            managers,
            index_names: index_names.iter()
                .filter(|n| !failed_indices.contains(n))
                .cloned()
                .collect(),
        })
    }
    
    /// Search across all indices and merge results
    pub fn search_multi(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let mut all_results = Vec::new();
        
        // Collect results from all indices
        for (idx, manager) in self.managers.iter().enumerate() {
            let index_name = &self.index_names[idx];
            info!("Searching in index: {}", index_name);
            
            match manager.search(query, limit * 2) {
                Ok(results) => {
                    info!("Found {} results in {}", results.len(), index_name);
                    all_results.extend(results);
                }
                Err(e) => {
                    warn!("Search failed for index '{}': {}", index_name, e);
                }
            }
        }
        
        // Merge and deduplicate results
        let merged = self.merge_results(all_results, limit);
        
        info!("Multi-index search completed: {} results", merged.len());
        Ok(merged)
    }
    
    /// Merge results from multiple indices
    fn merge_results(&self, results: Vec<SearchResult>, limit: usize) -> Vec<SearchResult> {
        // Use HashMap to deduplicate by URL, keeping highest score
        let mut url_map: HashMap<String, SearchResult> = HashMap::new();
        
        for result in results {
            url_map
                .entry(result.url.clone())
                .and_modify(|existing| {
                    if result.score > existing.score {
                        *existing = result.clone();
                    }
                })
                .or_insert(result);
        }
        
        // Convert to vector and sort by score
        let mut merged: Vec<SearchResult> = url_map.into_values().collect();
        merged.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Limit results
        merged.truncate(limit);
        merged
    }
    
    /// Get indexing status from all indices
    pub fn get_indexing_status_string(&self) -> String {
        let mut messages = Vec::new();
        
        for (idx, manager) in self.managers.iter().enumerate() {
            let index_name = &self.index_names[idx];
            // Get stats from each manager
            if let Ok(stats) = manager.get_stats() {
                messages.push(format!("{}: {} docs", index_name, stats.total_documents));
            }
        }
        
        format!(
            "Multi-index mode: {} indices loaded ({})",
            self.managers.len(),
            messages.join(", ")
        )
    }
}

#[async_trait]
impl SearchManagerTrait for MultiIndexSearchManager {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search_multi(query, limit)
    }
    
    async fn search_advanced(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        // For multi-index, we use the simple search for now
        // Advanced filtering could be implemented later
        let query = params.query.as_deref().unwrap_or("");
        self.search_multi(query, params.limit)
    }
    
    async fn get_content_by_url(&self, url: &str) -> Result<Option<String>> {
        // Try to get content from any index that has it
        for manager in &self.managers {
            if let Ok(Some(content)) = manager.get_content_by_url(url).await {
                return Ok(Some(content));
            }
        }
        Ok(None)
    }
    
    fn get_indexing_status(&self) -> String {
        self.get_indexing_status_string()
    }
    
    fn is_indexing_complete(&self) -> bool {
        true // Multi-index always uses pre-built indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_index_names() {
        let config = Config {
            index_name: Some("work,personal,research".to_string()),
            max_bookmarks: 0,
        };
        
        let names = config.parse_index_names();
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "work");
        assert_eq!(names[1], "personal");
        assert_eq!(names[2], "research");
    }
    
    #[test]
    fn test_parse_index_names_with_spaces() {
        let config = Config {
            index_name: Some("work , personal , research".to_string()),
            max_bookmarks: 0,
        };
        
        let names = config.parse_index_names();
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "work");
        assert_eq!(names[1], "personal");
        assert_eq!(names[2], "research");
    }
    
    #[test]
    fn test_parse_single_index() {
        let config = Config {
            index_name: Some("work".to_string()),
            max_bookmarks: 0,
        };
        
        let names = config.parse_index_names();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "work");
        assert!(!config.is_multi_index());
    }
    
    #[test]
    fn test_parse_empty_index() {
        let config = Config {
            index_name: None,
            max_bookmarks: 0,
        };
        
        let names = config.parse_index_names();
        assert_eq!(names.len(), 0);
    }
    
    #[test]
    fn test_merge_results_deduplication() {
        let manager = MultiIndexSearchManager {
            managers: vec![],
            index_names: vec![],
        };
        
        let results = vec![
            SearchResult {
                id: "1".to_string(),
                url: "http://example.com".to_string(),
                title: "Example 1".to_string(),
                snippet: "Snippet 1".to_string(),
                content: "Content 1".to_string(),
                score: 0.8,
                context_type: Some("ImportantNote".to_string()),
                full_content: None,
                folder_path: "folder1".to_string(),
                last_indexed: None,
            },
            SearchResult {
                id: "2".to_string(),
                url: "http://example.com".to_string(),
                title: "Example 2".to_string(),
                snippet: "Snippet 2".to_string(),
                content: "Content 2".to_string(),
                score: 0.9,  // Higher score
                context_type: Some("ImportantNote".to_string()),
                full_content: None,
                folder_path: "folder2".to_string(),
                last_indexed: None,
            },
            SearchResult {
                id: "3".to_string(),
                url: "http://other.com".to_string(),
                title: "Other".to_string(),
                snippet: "Other snippet".to_string(),
                content: "Other content".to_string(),
                score: 0.7,
                context_type: Some("RegularText".to_string()),
                full_content: None,
                folder_path: "folder3".to_string(),
                last_indexed: None,
            },
        ];
        
        let merged = manager.merge_results(results, 10);
        
        // Should have 2 unique URLs
        assert_eq!(merged.len(), 2);
        
        // Higher score should be kept for duplicate URL
        assert_eq!(merged[0].url, "http://example.com");
        assert_eq!(merged[0].score, 0.9);
        assert_eq!(merged[0].title, "Example 2");
        
        // Other URL should be present
        assert_eq!(merged[1].url, "http://other.com");
    }
    
    #[test]
    fn test_merge_results_limit() {
        let manager = MultiIndexSearchManager {
            managers: vec![],
            index_names: vec![],
        };
        
        let mut results = Vec::new();
        for i in 0..10 {
            results.push(SearchResult {
                id: format!("{}", i),
                url: format!("http://example{}.com", i),
                title: format!("Example {}", i),
                snippet: format!("Snippet {}", i),
                content: format!("Content {}", i),
                score: (10 - i) as f32 / 10.0,
                context_type: Some("RegularText".to_string()),
                full_content: None,
                folder_path: format!("folder{}", i),
                last_indexed: None,
            });
        }
        
        let merged = manager.merge_results(results, 5);
        
        // Should be limited to 5 results
        assert_eq!(merged.len(), 5);
        
        // Should be sorted by score (descending)
        assert!(merged[0].score >= merged[1].score);
        assert!(merged[1].score >= merged[2].score);
    }
}