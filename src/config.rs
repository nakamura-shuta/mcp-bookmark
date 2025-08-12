use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// List of folder paths to load (load all if empty)
    #[serde(default)]
    pub include_folders: Vec<Vec<String>>,

    /// List of folder paths to exclude
    #[serde(default)]
    pub exclude_folders: Vec<Vec<String>>,

    /// Maximum number of bookmarks to fetch (0 is unlimited)
    #[serde(default)]
    pub max_bookmarks: usize,

    /// Chrome profile name (display name)
    #[serde(default)]
    pub profile_name: Option<String>,

    /// Search by specific folder name (language independent)
    #[serde(default)]
    pub target_folder: Option<String>,
}

impl Config {
    /// Check if a folder path should be included based on filter configuration
    ///
    /// Returns false if the path matches any exclude_folders.
    /// Returns true if include_folders is empty or the path matches any include_folders.
    pub fn should_include_folder(&self, folder_path: &[String]) -> bool {
        // First check exclusions (exclude takes priority)
        for exclude in &self.exclude_folders {
            if folder_path.starts_with(exclude) {
                return false;
            }
        }

        // If no includes specified, include everything (that wasn't excluded)
        if self.include_folders.is_empty() {
            return true;
        }

        // Check if path matches any include pattern
        for include in &self.include_folders {
            if folder_path.starts_with(include) {
                return true;
            }
        }

        false
    }
}
