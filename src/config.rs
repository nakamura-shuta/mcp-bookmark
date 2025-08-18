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