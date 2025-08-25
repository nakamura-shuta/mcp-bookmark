use std::fmt;

/// Represents a parsed query term
#[derive(Debug, Clone, PartialEq)]
pub enum QueryTerm {
    /// A phrase that must appear exactly as specified (e.g., "React hooks")
    Phrase(String),
    /// A single word or token
    Word(String),
}

impl fmt::Display for QueryTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryTerm::Phrase(phrase) => write!(f, "\"{}\"", phrase),
            QueryTerm::Word(word) => write!(f, "{}", word),
        }
    }
}

/// Parser for search queries with phrase support
pub struct QueryParser;

impl QueryParser {
    /// Parse a query string into query terms
    /// Supports phrases in double quotes and regular words
    ///
    /// # Examples
    /// ```
    /// use mcp_bookmark::search::query_parser::{QueryParser, QueryTerm};
    ///
    /// let terms = QueryParser::parse("\"React hooks\" useState documentation");
    /// assert_eq!(terms[0], QueryTerm::Phrase("React hooks".to_string()));
    /// assert_eq!(terms[1], QueryTerm::Word("useState".to_string()));
    /// assert_eq!(terms[2], QueryTerm::Word("documentation".to_string()));
    /// ```
    pub fn parse(query: &str) -> Vec<QueryTerm> {
        let mut terms = Vec::new();
        let mut chars = query.chars().peekable();
        let mut current = String::new();
        let mut in_phrase = false;
        let mut escape_next = false;

        while let Some(ch) = chars.next() {
            if escape_next {
                current.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' => {
                    escape_next = true;
                }
                '"' => {
                    if in_phrase {
                        // End of phrase
                        if !current.trim().is_empty() {
                            terms.push(QueryTerm::Phrase(current.trim().to_string()));
                        }
                        current.clear();
                        in_phrase = false;
                    } else {
                        // Start of phrase - save any accumulated word first
                        if !current.trim().is_empty() {
                            for word in current.split_whitespace() {
                                if !word.is_empty() {
                                    terms.push(QueryTerm::Word(word.to_string()));
                                }
                            }
                        }
                        current.clear();
                        in_phrase = true;
                    }
                }
                ' ' | '\t' | '\n' | '\r' => {
                    if in_phrase {
                        // Keep whitespace in phrases
                        current.push(ch);
                    } else {
                        // End of word
                        if !current.trim().is_empty() {
                            terms.push(QueryTerm::Word(current.trim().to_string()));
                        }
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        // Handle any remaining content
        if in_phrase && !current.trim().is_empty() {
            // Unclosed phrase - treat as phrase anyway
            terms.push(QueryTerm::Phrase(current.trim().to_string()));
        } else if !current.trim().is_empty() {
            // Remaining words
            for word in current.split_whitespace() {
                if !word.is_empty() {
                    terms.push(QueryTerm::Word(word.to_string()));
                }
            }
        }

        terms
    }

    /// Check if the query contains any phrase terms
    pub fn has_phrases(terms: &[QueryTerm]) -> bool {
        terms
            .iter()
            .any(|term| matches!(term, QueryTerm::Phrase(_)))
    }

    /// Extract all phrases from query terms
    pub fn extract_phrases(terms: &[QueryTerm]) -> Vec<String> {
        terms
            .iter()
            .filter_map(|term| match term {
                QueryTerm::Phrase(phrase) => Some(phrase.clone()),
                _ => None,
            })
            .collect()
    }

    /// Extract all words from query terms
    pub fn extract_words(terms: &[QueryTerm]) -> Vec<String> {
        terms
            .iter()
            .filter_map(|term| match term {
                QueryTerm::Word(word) => Some(word.clone()),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_words() {
        let query = "React hooks documentation";
        let terms = QueryParser::parse(query);

        assert_eq!(terms.len(), 3);
        assert_eq!(terms[0], QueryTerm::Word("React".to_string()));
        assert_eq!(terms[1], QueryTerm::Word("hooks".to_string()));
        assert_eq!(terms[2], QueryTerm::Word("documentation".to_string()));
    }

    #[test]
    fn test_parse_single_phrase() {
        let query = "\"React hooks\"";
        let terms = QueryParser::parse(query);

        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0], QueryTerm::Phrase("React hooks".to_string()));
    }

    #[test]
    fn test_parse_mixed_phrase_and_words() {
        let query = "\"React hooks\" useState \"custom hook\" documentation";
        let terms = QueryParser::parse(query);

        assert_eq!(terms.len(), 4);
        assert_eq!(terms[0], QueryTerm::Phrase("React hooks".to_string()));
        assert_eq!(terms[1], QueryTerm::Word("useState".to_string()));
        assert_eq!(terms[2], QueryTerm::Phrase("custom hook".to_string()));
        assert_eq!(terms[3], QueryTerm::Word("documentation".to_string()));
    }

    #[test]
    fn test_parse_with_extra_whitespace() {
        let query = "  \"React  hooks\"   useState   ";
        let terms = QueryParser::parse(query);

        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0], QueryTerm::Phrase("React  hooks".to_string()));
        assert_eq!(terms[1], QueryTerm::Word("useState".to_string()));
    }

    #[test]
    fn test_parse_unclosed_phrase() {
        let query = "\"React hooks useState";
        let terms = QueryParser::parse(query);

        assert_eq!(terms.len(), 1);
        assert_eq!(
            terms[0],
            QueryTerm::Phrase("React hooks useState".to_string())
        );
    }

    #[test]
    fn test_parse_empty_phrase() {
        let query = "\"\" word \"  \"";
        let terms = QueryParser::parse(query);

        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0], QueryTerm::Word("word".to_string()));
    }

    #[test]
    fn test_parse_escaped_quote() {
        let query = r#"word \"escaped quote\" phrase"#;
        let terms = QueryParser::parse(query);

        assert_eq!(terms.len(), 4);
        assert_eq!(terms[0], QueryTerm::Word("word".to_string()));
        assert_eq!(terms[1], QueryTerm::Word("\"escaped".to_string()));
        assert_eq!(terms[2], QueryTerm::Word("quote\"".to_string()));
        assert_eq!(terms[3], QueryTerm::Word("phrase".to_string()));
    }

    #[test]
    fn test_has_phrases() {
        let terms = vec![
            QueryTerm::Phrase("React hooks".to_string()),
            QueryTerm::Word("useState".to_string()),
        ];
        assert!(QueryParser::has_phrases(&terms));

        let terms = vec![
            QueryTerm::Word("React".to_string()),
            QueryTerm::Word("hooks".to_string()),
        ];
        assert!(!QueryParser::has_phrases(&terms));
    }

    #[test]
    fn test_extract_phrases() {
        let terms = vec![
            QueryTerm::Phrase("React hooks".to_string()),
            QueryTerm::Word("useState".to_string()),
            QueryTerm::Phrase("custom hook".to_string()),
        ];

        let phrases = QueryParser::extract_phrases(&terms);
        assert_eq!(phrases.len(), 2);
        assert_eq!(phrases[0], "React hooks");
        assert_eq!(phrases[1], "custom hook");
    }

    #[test]
    fn test_extract_words() {
        let terms = vec![
            QueryTerm::Phrase("React hooks".to_string()),
            QueryTerm::Word("useState".to_string()),
            QueryTerm::Word("useEffect".to_string()),
        ];

        let words = QueryParser::extract_words(&terms);
        assert_eq!(words.len(), 2);
        assert_eq!(words[0], "useState");
        assert_eq!(words[1], "useEffect");
    }

    #[test]
    fn test_japanese_phrase() {
        let query = "\"React フック\" 状態管理";
        let terms = QueryParser::parse(query);

        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0], QueryTerm::Phrase("React フック".to_string()));
        assert_eq!(terms[1], QueryTerm::Word("状態管理".to_string()));
    }

    #[test]
    fn test_error_message_phrase() {
        let query = r#""Cannot read property 'undefined' of null" JavaScript"#;
        let terms = QueryParser::parse(query);

        assert_eq!(terms.len(), 2);
        assert_eq!(
            terms[0],
            QueryTerm::Phrase("Cannot read property 'undefined' of null".to_string())
        );
        assert_eq!(terms[1], QueryTerm::Word("JavaScript".to_string()));
    }
}
