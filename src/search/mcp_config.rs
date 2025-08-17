use crate::search::scored_snippet::ScoredSnippetGenerator;

/// MCP-optimized configuration for search responses
pub struct McpSearchConfig {
    /// Maximum snippets per result for MCP
    pub max_snippets_per_result: usize,
    /// Maximum snippet length for MCP
    pub max_snippet_length: usize,
    /// Maximum total results
    pub max_results: usize,
    /// Include backward compatibility fields
    pub include_legacy_fields: bool,
}

impl Default for McpSearchConfig {
    fn default() -> Self {
        Self {
            max_snippets_per_result: 2,   // Reduced from 5
            max_snippet_length: 300,      // Reduced from 400
            max_results: 10,              // Limit total results
            include_legacy_fields: false, // Don't include content_snippet and content_snippets
        }
    }
}

impl McpSearchConfig {
    /// Create MCP-optimized snippet generator
    pub fn create_snippet_generator(&self) -> ScoredSnippetGenerator {
        ScoredSnippetGenerator::with_config(
            self.max_snippet_length,
            self.max_snippets_per_result,
            100, // context_window
        )
    }
}
