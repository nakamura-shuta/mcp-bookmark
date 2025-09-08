use serde::{Deserialize, Serialize};
use std::cmp::min;

/// Phase 2.2: Scored snippet with relevance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredSnippet {
    /// The snippet text
    pub text: String,

    /// Relevance score (0.0 to 1.0)
    pub relevance_score: f32,

    /// Position in the document (character offset)
    pub position: usize,

    /// Type of content in the snippet
    pub context_type: ContextType,

    /// Query term density in this snippet
    pub match_density: f32,

    /// Section or heading this snippet belongs to
    pub section: Option<String>,
}

/// Type of content context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContextType {
    /// Main content paragraph
    Content,
    /// Code example or snippet
    CodeExample,
    /// Important note or warning
    ImportantNote,
    /// Procedure or step-by-step guide
    Procedure,
    /// Header or title
    Header,
    /// List or bullet points
    ListItem,
    /// Unknown or mixed content
    Mixed,
}

/// Enhanced snippet generator with scoring (Phase 2.2)
#[derive(Debug)]
pub struct ScoredSnippetGenerator {
    max_snippet_length: usize,
    max_snippets: usize,
    context_window: usize,
}

impl ScoredSnippetGenerator {
    pub fn new() -> Self {
        let config = crate::config::Config::default();
        // Use 1.5x the configured snippet length for internal buffer
        let buffer_size = config.max_snippet_length + (config.max_snippet_length / 2);
        Self {
            max_snippet_length: buffer_size,
            max_snippets: 5,
            context_window: config.max_snippet_length / 3, // 1/3 of snippet length
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        max_snippet_length: usize,
        max_snippets: usize,
        context_window: usize,
    ) -> Self {
        Self {
            max_snippet_length,
            max_snippets,
            context_window,
        }
    }

    /// Generate a single best snippet from content
    pub fn generate_snippet(&self, content: &str, query: &str, max_len: usize) -> ScoredSnippet {
        let snippets = self.generate_scored_snippets(content, query);

        if let Some(mut best) = snippets.into_iter().next() {
            // Truncate if needed
            if best.text.len() > max_len {
                // Find safe UTF-8 boundary
                let mut truncate_pos = max_len;
                while truncate_pos > 0 && !best.text.is_char_boundary(truncate_pos) {
                    truncate_pos -= 1;
                }
                best.text.truncate(truncate_pos);
                if !best.text.ends_with("...") {
                    best.text.push_str("...");
                }
            }
            best
        } else {
            // Return fallback snippet if no matches
            self.create_fallback_snippet(content)
        }
    }

    /// Generate scored snippets from content
    pub fn generate_scored_snippets(&self, content: &str, query: &str) -> Vec<ScoredSnippet> {
        if content.is_empty() || query.is_empty() {
            return vec![];
        }

        // Tokenize query
        let query_terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        // Find all match positions with detailed scoring
        let mut match_positions = self.find_detailed_matches(content, &query_terms);

        if match_positions.is_empty() {
            // Return beginning with low score if no matches
            return vec![self.create_fallback_snippet(content)];
        }

        // Sort by relevance score
        match_positions.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());

        // Generate scored snippets
        let mut snippets = Vec::new();
        let mut used_ranges = Vec::new();

        for match_info in match_positions.iter().take(self.max_snippets * 2) {
            if self.overlaps_with_ranges(match_info.position, &used_ranges) {
                continue;
            }

            let snippet = self.create_scored_snippet(content, match_info, &query_terms);

            if let Some(s) = snippet {
                let range = (
                    match_info.position.saturating_sub(self.context_window),
                    min(match_info.position + self.context_window, content.len()),
                );
                used_ranges.push(range);
                snippets.push(s);

                if snippets.len() >= self.max_snippets {
                    break;
                }
            }
        }

        snippets
    }

    /// Find matches with detailed scoring information
    fn find_detailed_matches(&self, content: &str, query_terms: &[String]) -> Vec<MatchInfo> {
        let content_lower = content.to_lowercase();
        let mut matches = Vec::new();

        // Sliding window analysis - use configured snippet length
        let config = crate::config::Config::default();
        let window_size = config.max_snippet_length;
        let step = config.max_snippet_length / 3;

        for start in (0..content.len()).step_by(step) {
            let mut start_byte = start;
            while start_byte < content.len() && !content.is_char_boundary(start_byte) {
                start_byte += 1;
            }
            if start_byte >= content.len() {
                break;
            }

            let end = min(start_byte + window_size, content.len());
            let mut end_byte = end;
            while end_byte < content.len() && !content.is_char_boundary(end_byte) {
                end_byte += 1;
            }

            let window_text = &content_lower[start_byte..end_byte];
            let original_window = &content[start_byte..end_byte];

            // Calculate match score for this window
            let (match_count, unique_terms) = self.count_matches(window_text, query_terms);

            if match_count > 0 {
                let density = match_count as f32 / (window_size as f32 / 100.0);
                let term_coverage = unique_terms as f32 / query_terms.len() as f32;
                let context_type = self.detect_context_type(original_window);
                let context_boost = self.get_context_boost(&context_type);

                // Combined relevance score
                let relevance =
                    (density * 0.4 + term_coverage * 0.4 + context_boost * 0.2).min(1.0);

                matches.push(MatchInfo {
                    position: start_byte,
                    relevance,
                    match_count,
                    context_type,
                    density,
                });
            }
        }

        matches
    }

    /// Count matches and unique terms in text
    fn count_matches(&self, text: &str, query_terms: &[String]) -> (usize, usize) {
        let mut total_matches = 0;
        let mut unique_terms = 0;

        for term in query_terms {
            let matches = text.matches(term).count();
            if matches > 0 {
                total_matches += matches;
                unique_terms += 1;
            }
        }

        (total_matches, unique_terms)
    }

    /// Detect the type of content based on patterns
    fn detect_context_type(&self, text: &str) -> ContextType {
        // Check for important notes first (more specific patterns)
        if text.contains("重要")
            || text.contains("注意")
            || text.contains("WARNING")
            || text.contains("NOTE:")
            || text.contains("Note:")
            || text.contains("！")
            || text.contains("!")
        {
            return ContextType::ImportantNote;
        }

        // Check for code patterns
        if text.contains("```")
            || text.contains("function")
            || text.contains("class")
            || (text.contains("import") && !text.contains("important"))
            || text.contains("export")
            || text.contains("{")
        {
            return ContextType::CodeExample;
        }

        // Check for procedure markers
        if text.contains("Step")
            || text.contains("手順")
            || text.contains("1.")
            || text.contains("2.")
        {
            return ContextType::Procedure;
        }

        // Check for list items
        if text.contains("- ") || text.contains("* ") || text.contains("• ") {
            return ContextType::ListItem;
        }

        // Check for headers (simple heuristic)
        if text.len() < 100
            && (text.contains("#")
                || text.chars().filter(|c| c.is_uppercase()).count() > text.len() / 3)
        {
            return ContextType::Header;
        }

        ContextType::Content
    }

    /// Get relevance boost based on context type
    fn get_context_boost(&self, context_type: &ContextType) -> f32 {
        match context_type {
            ContextType::CodeExample => 0.8,
            ContextType::Procedure => 0.9,
            ContextType::ImportantNote => 0.85,
            ContextType::Header => 0.7,
            ContextType::ListItem => 0.75,
            ContextType::Content => 0.6,
            ContextType::Mixed => 0.5,
        }
    }

    /// Create a scored snippet from match information
    fn create_scored_snippet(
        &self,
        content: &str,
        match_info: &MatchInfo,
        query_terms: &[String],
    ) -> Option<ScoredSnippet> {
        // Find sentence boundaries
        let start = self.find_sentence_start(
            content,
            match_info.position.saturating_sub(self.context_window),
        );
        let end = self.find_sentence_end(
            content,
            min(match_info.position + self.context_window, content.len()),
        );

        // Ensure valid UTF-8 boundaries
        let mut start_byte = start;
        while start_byte < content.len() && !content.is_char_boundary(start_byte) {
            start_byte += 1;
        }
        let mut end_byte = end;
        while end_byte > start_byte && !content.is_char_boundary(end_byte) {
            end_byte -= 1;
        }

        if start_byte >= end_byte {
            return None;
        }

        // Extract text
        let mut text = content[start_byte..end_byte].trim().to_string();

        // Add ellipsis if needed
        if start_byte > 0 {
            text = format!("...{text}");
        }
        if end_byte < content.len() {
            text = format!("{text}...");
        }

        // Detect section heading if possible
        let section = self.find_section_heading(content, match_info.position);

        Some(ScoredSnippet {
            text: self.highlight_terms(&text, query_terms),
            relevance_score: match_info.relevance,
            position: match_info.position,
            context_type: match_info.context_type.clone(),
            match_density: match_info.density,
            section,
        })
    }

    /// Create a fallback snippet when no matches found
    fn create_fallback_snippet(&self, content: &str) -> ScoredSnippet {
        let max_len = min(self.max_snippet_length, content.len());
        let mut end_byte = max_len;
        while end_byte > 0 && !content.is_char_boundary(end_byte) {
            end_byte -= 1;
        }

        let text = if content.len() > max_len {
            format!("{}...", &content[..end_byte])
        } else {
            content.to_string()
        };

        ScoredSnippet {
            text,
            relevance_score: 0.1,
            position: 0,
            context_type: ContextType::Content,
            match_density: 0.0,
            section: None,
        }
    }

    /// Find the section heading for a position
    fn find_section_heading(&self, content: &str, position: usize) -> Option<String> {
        // Look backwards for heading patterns
        let mut search_start = position.saturating_sub(1000);
        let mut search_end = position;

        // Ensure valid UTF-8 boundaries
        while search_start < content.len() && !content.is_char_boundary(search_start) {
            search_start += 1;
        }
        while search_end > search_start && !content.is_char_boundary(search_end) {
            search_end -= 1;
        }

        if search_start >= search_end {
            return None;
        }

        let search_text = &content[search_start..search_end];

        // Look for markdown headers
        if let Some(header_pos) = search_text.rfind("\n#") {
            let mut header_start = search_start + header_pos + 1;

            // Ensure header_start is at valid UTF-8 boundary
            while header_start < content.len() && !content.is_char_boundary(header_start) {
                header_start += 1;
            }

            if header_start < content.len() {
                if let Some(header_end_offset) = content[header_start..].find('\n') {
                    let mut header_end = header_start + header_end_offset;

                    // Ensure header_end is at valid UTF-8 boundary
                    while header_end > header_start && !content.is_char_boundary(header_end) {
                        header_end -= 1;
                    }

                    if header_start < header_end {
                        let header = content[header_start..header_end]
                            .trim_start_matches('#')
                            .trim();
                        return Some(header.to_string());
                    }
                }
            }
        }

        None
    }

    /// Check if position overlaps with existing ranges
    fn overlaps_with_ranges(&self, position: usize, ranges: &[(usize, usize)]) -> bool {
        for &(start, end) in ranges {
            if position >= start && position <= end {
                return true;
            }
        }
        false
    }

    /// Find sentence start
    fn find_sentence_start(&self, content: &str, position: usize) -> usize {
        if position == 0 {
            return 0;
        }

        let bytes = content.as_bytes();
        let mut pos = position;

        while pos > 0 {
            if pos >= 2 {
                let prev_char = bytes[pos - 1];
                let prev_prev_char = bytes[pos - 2];

                if (prev_prev_char == b'.' || prev_prev_char == b'!' || prev_prev_char == b'?')
                    && prev_char == b' '
                {
                    return pos;
                }
            }

            if pos >= 1 && bytes[pos - 1] == b'\n' {
                return pos;
            }

            pos = pos.saturating_sub(1);
        }

        0
    }

    /// Find sentence end
    fn find_sentence_end(&self, content: &str, position: usize) -> usize {
        let bytes = content.as_bytes();
        let mut pos = position;

        while pos < content.len() {
            if bytes[pos] == b'.' || bytes[pos] == b'!' || bytes[pos] == b'?' {
                if pos + 1 >= content.len() {
                    return content.len();
                } else if bytes[pos + 1] == b' ' || bytes[pos + 1] == b'\n' {
                    return pos + 1;
                }
            }

            if bytes[pos] == b'\n' {
                return pos;
            }

            pos += 1;
        }

        content.len()
    }

    /// Highlight query terms (returns marked text)
    fn highlight_terms(&self, text: &str, _query_terms: &[String]) -> String {
        // For now, return as-is
        // Future: Add **term** or <mark>term</mark> highlighting
        text.to_string()
    }
}

impl Default for ScoredSnippetGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal match information
#[derive(Debug)]
struct MatchInfo {
    position: usize,
    relevance: f32,
    #[allow(dead_code)]
    match_count: usize,
    context_type: ContextType,
    density: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scored_snippet_generation() {
        let generator = ScoredSnippetGenerator::new();
        let content = "This is important information about database connections. \
                      Step 1: Configure the connection string. \
                      Step 2: Test the connection. \
                      ```python\ndb.connect()\n```\n\
                      Note: Always close connections properly.";

        let snippets = generator.generate_scored_snippets(content, "database connection");

        assert!(!snippets.is_empty());

        // Check relevance scores are reasonable
        for snippet in &snippets {
            assert!(snippet.relevance_score >= 0.0 && snippet.relevance_score <= 1.0);
        }

        // Check that we have various context types
        let has_code = snippets
            .iter()
            .any(|s| s.context_type == ContextType::CodeExample);
        let has_note = snippets
            .iter()
            .any(|s| s.context_type == ContextType::ImportantNote);
        assert!(
            has_code || has_note,
            "Should have at least one special context type"
        );
    }

    #[test]
    fn test_context_type_detection() {
        let generator = ScoredSnippetGenerator::new();

        assert_eq!(
            generator.detect_context_type("```python\nprint('hello')\n```"),
            ContextType::CodeExample
        );

        assert_eq!(
            generator.detect_context_type("Step 1: First do this"),
            ContextType::Procedure
        );

        assert_eq!(
            generator.detect_context_type("NOTE: This is important!"),
            ContextType::ImportantNote
        );
    }
}
