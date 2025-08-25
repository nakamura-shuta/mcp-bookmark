use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// Index name to use (direct index selection)
    #[serde(default)]
    pub index_name: Option<String>,

    /// Maximum number of bookmarks to fetch (0 is unlimited)
    #[serde(default)]
    pub max_bookmarks: usize,
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
