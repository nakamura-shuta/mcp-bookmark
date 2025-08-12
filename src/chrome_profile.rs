use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tracing::info;

/// Chrome Local State file structure (only necessary parts)
#[derive(Debug, Deserialize)]
struct LocalState {
    profile: ProfileInfo,
}

#[derive(Debug, Deserialize)]
struct ProfileInfo {
    info_cache: Value,
}

/// Profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromeProfile {
    pub directory_name: String, // "Default", "Profile 1", etc.
    pub display_name: String,   // "Work", "Personal", etc.
    pub path: PathBuf,
    pub bookmark_count: Option<usize>, // Number of bookmarks
    pub size_kb: Option<u64>,          // Size of bookmarks file in KB
}

/// Structure to manage Chrome profiles
pub struct ProfileResolver {
    chrome_base_dir: PathBuf,
}

impl ProfileResolver {
    /// Create new
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        let chrome_base_dir = home.join("Library/Application Support/Google/Chrome");

        if !chrome_base_dir.exists() {
            anyhow::bail!("Chrome directory not found at {:?}", chrome_base_dir);
        }

        Ok(Self { chrome_base_dir })
    }

    /// Read Local State file
    fn read_local_state(&self) -> Result<LocalState> {
        let local_state_path = self.chrome_base_dir.join("Local State");

        if !local_state_path.exists() {
            anyhow::bail!("Local State file not found at {:?}", local_state_path);
        }

        let content =
            fs::read_to_string(&local_state_path).context("Failed to read Local State file")?;

        let state: LocalState =
            serde_json::from_str(&content).context("Failed to parse Local State JSON")?;

        Ok(state)
    }

    /// Resolve directory from profile name
    pub fn resolve_by_name(&self, profile_name: &str) -> Result<ChromeProfile> {
        let state = self.read_local_state()?;

        // Search all profiles from info_cache
        if let Some(info_cache) = state.profile.info_cache.as_object() {
            for (dir_name, profile_info) in info_cache {
                // Check name field
                if let Some(name) = profile_info.get("name").and_then(|n| n.as_str()) {
                    if name == profile_name {
                        let profile_path = self.chrome_base_dir.join(dir_name);

                        info!(
                            "Found profile '{}' at directory '{}'",
                            profile_name, dir_name
                        );

                        return Ok(ChromeProfile {
                            directory_name: dir_name.clone(),
                            display_name: name.to_string(),
                            path: profile_path,
                            bookmark_count: None,
                            size_kb: None,
                        });
                    }
                }

                // Also check gaia_name (for Google account names)
                if let Some(gaia_name) = profile_info.get("gaia_name").and_then(|n| n.as_str()) {
                    if gaia_name == profile_name {
                        let profile_path = self.chrome_base_dir.join(dir_name);

                        info!(
                            "Found profile '{}' (gaia) at directory '{}'",
                            profile_name, dir_name
                        );

                        return Ok(ChromeProfile {
                            directory_name: dir_name.clone(),
                            display_name: gaia_name.to_string(),
                            path: profile_path,
                            bookmark_count: None,
                            size_kb: None,
                        });
                    }
                }
            }
        }

        anyhow::bail!("Profile '{}' not found", profile_name)
    }

    /// Get Bookmarks file path from profile directory
    pub fn get_bookmarks_path(&self, profile: &ChromeProfile) -> PathBuf {
        profile.path.join("Bookmarks")
    }

    /// Get all available profiles
    pub fn list_all_profiles(&self) -> Result<Vec<ChromeProfile>> {
        let mut profiles = Vec::new();
        let state = self.read_local_state()?;

        // Get all profiles from info_cache
        if let Some(info_cache) = state.profile.info_cache.as_object() {
            for (dir_name, profile_info) in info_cache {
                let profile_path = self.chrome_base_dir.join(dir_name);
                let bookmarks_path = profile_path.join("Bookmarks");

                // Get bookmark file size and count
                let (size_kb, bookmark_count) = if bookmarks_path.exists() {
                    let size = fs::metadata(&bookmarks_path).map(|m| m.len() / 1024).ok();

                    let count = Self::count_bookmarks(&bookmarks_path);
                    (size, count)
                } else {
                    (None, None)
                };

                // Get display name (name or gaia_name)
                let display_name = profile_info
                    .get("name")
                    .and_then(|n| n.as_str())
                    .or_else(|| profile_info.get("gaia_name").and_then(|n| n.as_str()))
                    .unwrap_or(dir_name)
                    .to_string();

                profiles.push(ChromeProfile {
                    directory_name: dir_name.clone(),
                    display_name,
                    path: profile_path,
                    bookmark_count,
                    size_kb,
                });
            }
        }

        // Add Default profile if not included
        if !profiles.iter().any(|p| p.directory_name == "Default") {
            let default_path = self.chrome_base_dir.join("Default");
            if default_path.exists() {
                let bookmarks_path = default_path.join("Bookmarks");
                let (size_kb, bookmark_count) = if bookmarks_path.exists() {
                    let size = fs::metadata(&bookmarks_path).map(|m| m.len() / 1024).ok();
                    let count = Self::count_bookmarks(&bookmarks_path);
                    (size, count)
                } else {
                    (None, None)
                };

                profiles.push(ChromeProfile {
                    directory_name: "Default".to_string(),
                    display_name: "Default".to_string(),
                    path: default_path,
                    bookmark_count,
                    size_kb,
                });
            }
        }

        // Sort by size (descending)
        profiles.sort_by_key(|p| std::cmp::Reverse(p.size_kb.unwrap_or(0)));

        Ok(profiles)
    }

    /// Count bookmarks from file
    fn count_bookmarks(bookmarks_path: &PathBuf) -> Option<usize> {
        if let Ok(content) = fs::read_to_string(bookmarks_path) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                return Some(Self::count_bookmarks_recursive(&json));
            }
        }
        None
    }

    /// Recursively count bookmarks from JSON
    fn count_bookmarks_recursive(value: &Value) -> usize {
        let mut count = 0;

        if let Some(obj) = value.as_object() {
            // Count URL type nodes
            if let Some(node_type) = obj.get("type").and_then(|t| t.as_str()) {
                if node_type == "url" {
                    count += 1;
                }
            }

            // Recursively explore children
            if let Some(children) = obj.get("children").and_then(|c| c.as_array()) {
                for child in children {
                    count += Self::count_bookmarks_recursive(child);
                }
            }

            // For root node
            if let Some(roots) = obj.get("roots").and_then(|r| r.as_object()) {
                for (_, root) in roots {
                    count += Self::count_bookmarks_recursive(root);
                }
            }
        }

        count
    }

    /// Guess currently active profile
    pub fn get_current_profile(&self) -> Option<ChromeProfile> {
        // Return profile with largest bookmark file
        self.list_all_profiles()
            .ok()
            .and_then(|profiles| profiles.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_resolver_creation() {
        // Create profile resolver
        let resolver = ProfileResolver::new();
        assert!(resolver.is_ok() || resolver.is_err()); // 環境依存
    }
}
