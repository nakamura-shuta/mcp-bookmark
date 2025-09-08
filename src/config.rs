use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Index name to use (direct index selection)
    #[serde(default)]
    pub index_name: Option<String>,

    /// Maximum number of bookmarks to fetch (0 is unlimited)
    #[serde(default)]
    pub max_bookmarks: usize,

    /// Maximum snippet length for search results
    #[serde(default = "default_max_snippet_length")]
    pub max_snippet_length: usize,
}

/// Default maximum snippet length for search results
pub const DEFAULT_MAX_SNIPPET_LENGTH: usize = 600;

fn default_max_snippet_length() -> usize {
    DEFAULT_MAX_SNIPPET_LENGTH
}

impl Default for Config {
    fn default() -> Self {
        Self {
            index_name: None,
            max_bookmarks: 0,
            max_snippet_length: default_max_snippet_length(),
        }
    }
}

impl Config {
    /// Parse index names from comma-separated string
    pub fn parse_index_names(&self) -> Vec<String> {
        self.index_name
            .as_ref()
            .map(|s| {
                s.split(',')
                    .map(|name| name.trim())
                    .filter(|name| !name.is_empty())
                    .map(|name| name.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if multiple indices are configured
    pub fn is_multi_index(&self) -> bool {
        self.parse_index_names().len() > 1
    }
}
