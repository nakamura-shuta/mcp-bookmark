use std::cmp::min;

/// Improved snippet generator that considers sentence boundaries
#[derive(Debug)]
pub struct SnippetGenerator {
    max_snippet_length: usize,
    max_snippets: usize,
    context_window: usize,
}

impl SnippetGenerator {
    pub fn new() -> Self {
        Self {
            max_snippet_length: 300, // Maximum characters per snippet
            max_snippets: 3,         // Maximum number of snippets to return
            context_window: 50,      // Characters before/after match
        }
    }

    /// Generate snippets from content that preserve sentence boundaries
    pub fn generate_snippets(&self, content: &str, query: &str) -> Vec<String> {
        if content.is_empty() || query.is_empty() {
            return vec![];
        }

        // Tokenize query into words
        let query_terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        // Find all positions where query terms appear
        let mut match_positions = self.find_match_positions(content, &query_terms);

        if match_positions.is_empty() {
            // If no matches, return the beginning of content
            return vec![self.extract_sentence_aware_snippet(content, 0, self.max_snippet_length)];
        }

        // Sort positions by relevance score
        match_positions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Generate snippets for top matches
        let mut snippets = Vec::new();
        let mut used_ranges = Vec::new();

        for (pos, _score) in match_positions.iter().take(self.max_snippets) {
            // Skip if this position overlaps with an already used range
            if self.overlaps_with_ranges(*pos, &used_ranges) {
                continue;
            }

            let snippet = self.extract_snippet_around_position(content, *pos, &query_terms);
            if !snippet.is_empty() {
                let range = (
                    pos.saturating_sub(self.context_window),
                    min(pos + self.context_window, content.len()),
                );
                used_ranges.push(range);
                snippets.push(snippet);
            }

            if snippets.len() >= self.max_snippets {
                break;
            }
        }

        // If no snippets generated, fall back to beginning of content
        if snippets.is_empty() {
            snippets.push(self.extract_sentence_aware_snippet(content, 0, self.max_snippet_length));
        }

        snippets
    }

    /// Find positions where query terms appear with relevance scores
    fn find_match_positions(&self, content: &str, query_terms: &[String]) -> Vec<(usize, f32)> {
        let content_lower = content.to_lowercase();
        let mut positions = Vec::new();

        // Sliding window to find dense areas of matches
        let window_size = 200;
        let step = 50;

        for start in (0..content.len()).step_by(step) {
            // Ensure we're at a valid UTF-8 boundary for start
            let mut start_byte = start;
            while start_byte < content.len() && !content.is_char_boundary(start_byte) {
                start_byte += 1;
            }
            if start_byte >= content.len() {
                break;
            }

            let end = min(start_byte + window_size, content.len());
            // Ensure we're at a valid UTF-8 boundary for end
            let mut end_byte = end;
            while end_byte < content.len() && !content.is_char_boundary(end_byte) {
                end_byte += 1;
            }

            let window_text = &content_lower[start_byte..end_byte];

            let mut score = 0.0;
            let mut match_found = false;

            for term in query_terms {
                let matches = window_text.matches(term).count();
                if matches > 0 {
                    match_found = true;
                    score += matches as f32;
                }
            }

            if match_found {
                // Calculate term density for scoring
                let density = score / (window_size as f32 / 100.0);
                positions.push((start_byte, density));
            }
        }

        positions
    }

    /// Extract a snippet around a specific position
    fn extract_snippet_around_position(
        &self,
        content: &str,
        position: usize,
        query_terms: &[String],
    ) -> String {
        // Find sentence boundaries around the position
        let start = self.find_sentence_start(content, position.saturating_sub(self.context_window));
        let end =
            self.find_sentence_end(content, min(position + self.context_window, content.len()));

        // Ensure start and end are at valid UTF-8 boundaries
        let mut start_byte = start;
        while start_byte < content.len() && !content.is_char_boundary(start_byte) {
            start_byte += 1;
        }
        let mut end_byte = end;
        while end_byte > start_byte && !content.is_char_boundary(end_byte) {
            end_byte -= 1;
        }

        // Extract the snippet
        let mut snippet = content[start_byte..end_byte].trim().to_string();

        // Add ellipsis if needed
        if start_byte > 0 {
            snippet = format!("...{}", snippet);
        }
        if end_byte < content.len() {
            snippet = format!("{}...", snippet);
        }

        // Highlight query terms (optional, for better visibility)
        snippet = self.highlight_terms(&snippet, query_terms);

        snippet
    }

    /// Extract a sentence-aware snippet from a specific position
    fn extract_sentence_aware_snippet(
        &self,
        content: &str,
        start: usize,
        max_length: usize,
    ) -> String {
        // Ensure start is at a valid UTF-8 boundary
        let mut start_byte = start;
        while start_byte < content.len() && !content.is_char_boundary(start_byte) {
            start_byte += 1;
        }

        let end = min(start_byte + max_length, content.len());
        let sentence_end = self.find_sentence_end(content, end);

        // Ensure sentence_end is at a valid UTF-8 boundary
        let mut end_byte = sentence_end;
        while end_byte > start_byte && !content.is_char_boundary(end_byte) {
            end_byte -= 1;
        }

        let mut snippet = content[start_byte..end_byte].trim().to_string();

        if start_byte > 0 {
            snippet = format!("...{}", snippet);
        }
        if end_byte < content.len() {
            snippet = format!("{}...", snippet);
        }

        snippet
    }

    /// Find the start of a sentence
    fn find_sentence_start(&self, content: &str, position: usize) -> usize {
        if position == 0 {
            return 0;
        }

        let bytes = content.as_bytes();
        let mut pos = position;

        // Look backwards for sentence boundaries
        while pos > 0 {
            if pos >= 2 {
                let prev_char = bytes[pos - 1];
                let prev_prev_char = bytes[pos - 2];

                // Check for sentence endings (. ! ?)
                if (prev_prev_char == b'.' || prev_prev_char == b'!' || prev_prev_char == b'?')
                    && prev_char == b' '
                {
                    return pos;
                }
            }

            // Also check for paragraph boundaries
            if pos >= 1 && bytes[pos - 1] == b'\n' {
                return pos;
            }

            pos = pos.saturating_sub(1);
        }

        0
    }

    /// Find the end of a sentence
    fn find_sentence_end(&self, content: &str, position: usize) -> usize {
        let bytes = content.as_bytes();
        let mut pos = position;

        // Look forward for sentence boundaries
        while pos < content.len() {
            if bytes[pos] == b'.' || bytes[pos] == b'!' || bytes[pos] == b'?' {
                // Check if followed by space or end of content
                // Include the punctuation mark in the snippet
                if pos + 1 >= content.len() {
                    return content.len();
                } else if bytes[pos + 1] == b' ' || bytes[pos + 1] == b'\n' {
                    return pos + 1;
                }
            }

            // Also check for paragraph boundaries
            if bytes[pos] == b'\n' {
                return pos;
            }

            pos += 1;
        }

        content.len()
    }

    /// Check if a position overlaps with existing ranges
    fn overlaps_with_ranges(&self, position: usize, ranges: &[(usize, usize)]) -> bool {
        for &(start, end) in ranges {
            if position >= start && position <= end {
                return true;
            }
        }
        false
    }

    /// Highlight query terms in the snippet (optional enhancement)
    fn highlight_terms(&self, text: &str, _query_terms: &[String]) -> String {
        // For now, just return the text as-is
        // In the future, we could add markdown or other highlighting
        text.to_string()
    }
}

impl Default for SnippetGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentence_boundary_detection() {
        let generator = SnippetGenerator::new();
        let content =
            "This is the first sentence. This is the second sentence! And this is the third?";

        // Test finding sentence start
        assert_eq!(generator.find_sentence_start(content, 35), 28);

        // Test finding sentence end
        assert_eq!(generator.find_sentence_end(content, 35), 56);
    }

    #[test]
    fn test_snippet_generation() {
        let generator = SnippetGenerator::new();
        let content = "The database connection is important. You need to configure the connection string properly. The connection pool size matters.";
        let query = "database connection";

        let snippets = generator.generate_snippets(content, query);
        assert!(!snippets.is_empty());

        // Check that snippets preserve sentence boundaries
        for snippet in &snippets {
            // Should not start or end mid-word (unless with ellipsis)
            if !snippet.starts_with("...") {
                assert!(
                    snippet.chars().next().unwrap().is_uppercase()
                        || snippet.chars().next().unwrap().is_alphabetic()
                );
            }
        }
    }

    #[test]
    fn test_multiple_snippets() {
        let generator = SnippetGenerator::new();
        let content = "First paragraph about database connections. Some unrelated content here. Second paragraph with database mentioned. More unrelated text. Third section discussing connection pooling.";
        let query = "database connection";

        let snippets = generator.generate_snippets(content, query);
        assert!(snippets.len() <= 3);
    }

    #[test]
    fn test_empty_query() {
        let generator = SnippetGenerator::new();
        let content = "Some content here.";
        let snippets = generator.generate_snippets(content, "");
        assert_eq!(snippets.len(), 0);
    }

    #[test]
    fn test_no_matches() {
        let generator = SnippetGenerator::new();
        let content = "This content has nothing related to the query.";
        let query = "xyz123";

        let snippets = generator.generate_snippets(content, query);
        assert_eq!(snippets.len(), 1);
        // Should return beginning of content
        assert!(snippets[0].contains("This content"));
    }
}
