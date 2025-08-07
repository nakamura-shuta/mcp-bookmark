use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromeBookmarks {
    pub checksum: String,
    pub roots: BookmarkRoots,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkRoots {
    pub bookmark_bar: BookmarkNode,
    pub other: BookmarkNode,
    pub synced: BookmarkNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkNode {
    #[serde(default)]
    pub children: Option<Vec<BookmarkNode>>,
    pub date_added: Option<String>,
    pub date_last_used: Option<String>,
    pub date_modified: Option<String>,
    pub guid: String,
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub url: Option<String>,
    #[serde(skip)]
    pub folder_path: Vec<String>,
}

impl BookmarkNode {
    pub fn is_folder(&self) -> bool {
        self.node_type == "folder"
    }

    pub fn is_url(&self) -> bool {
        self.node_type == "url"
    }

    pub fn set_folder_paths(&mut self, parent_path: Vec<String>) {
        let mut current_path = parent_path.clone();
        if !self.name.is_empty() && self.is_folder() {
            current_path.push(self.name.clone());
        }
        self.folder_path = current_path.clone();

        if let Some(children) = &mut self.children {
            for child in children {
                child.set_folder_paths(current_path.clone());
            }
        }
    }

    pub fn flatten(&self) -> Vec<FlatBookmark> {
        let mut result = Vec::new();
        self.flatten_recursive(&mut result);
        result
    }

    fn flatten_recursive(&self, result: &mut Vec<FlatBookmark>) {
        if self.is_url() {
            if let Some(url) = &self.url {
                result.push(FlatBookmark {
                    id: self.id.clone(),
                    name: self.name.clone(),
                    url: url.clone(),
                    date_added: self.date_added.clone(),
                    date_modified: self.date_modified.clone(),
                    folder_path: self.folder_path.clone(),
                });
            }
        }

        if let Some(children) = &self.children {
            for child in children {
                child.flatten_recursive(result);
            }
        }
    }

    pub fn find_folder(&self, path: &[String]) -> Option<&BookmarkNode> {
        if path.is_empty() {
            return Some(self);
        }

        if let Some(children) = &self.children {
            for child in children {
                if child.is_folder() && child.name == path[0] {
                    return child.find_folder(&path[1..]);
                }
            }
        }
        None
    }

    /// フォルダ名で検索（言語非依存）
    pub fn find_folder_by_name(&self, folder_name: &str) -> Option<&BookmarkNode> {
        // 自分自身がマッチする場合
        if self.is_folder() && self.name == folder_name {
            return Some(self);
        }

        // 子要素を再帰的に検索
        if let Some(children) = &self.children {
            for child in children {
                if let Some(found) = child.find_folder_by_name(folder_name) {
                    return Some(found);
                }
            }
        }
        None
    }

    pub fn collect_all_folders(&self) -> Vec<Vec<String>> {
        let mut folders = Vec::new();
        self.collect_folders_recursive(&mut folders);
        folders
    }

    fn collect_folders_recursive(&self, folders: &mut Vec<Vec<String>>) {
        if self.is_folder() && !self.folder_path.is_empty() {
            folders.push(self.folder_path.clone());
        }

        if let Some(children) = &self.children {
            for child in children {
                child.collect_folders_recursive(folders);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatBookmark {
    pub id: String,
    pub name: String,
    pub url: String,
    pub date_added: Option<String>,
    pub date_modified: Option<String>,
    pub folder_path: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BookmarkReader {
    bookmarks_path: PathBuf,
    config: Config,
}

impl BookmarkReader {
    pub fn with_config(config: Config) -> Result<Self> {
        // プロファイル名指定がある場合は優先的に使用
        let bookmarks_path = if let Some(profile_name) = &config.profile_name {
            let resolver = crate::chrome_profile::ProfileResolver::new()?;
            let profile = resolver.resolve_by_name(profile_name)?;
            let path = resolver.get_bookmarks_path(&profile);
            tracing::info!(
                "Using specified profile '{}' ({})",
                profile.display_name,
                profile.directory_name
            );
            path
        } else {
            Self::find_bookmarks_path()?
        };

        Ok(Self {
            bookmarks_path,
            config,
        })
    }

    /// ブックマークファイルを探す（環境変数またはデフォルト）
    fn find_bookmarks_path() -> Result<PathBuf> {
        // 環境変数でプロファイルを指定可能
        if let Ok(profile) = std::env::var("CHROME_PROFILE") {
            let home = dirs::home_dir().context("Failed to get home directory")?;
            let path = home.join(format!(
                "Library/Application Support/Google/Chrome/{profile}/Bookmarks"
            ));
            if path.exists() {
                tracing::info!("Using Chrome profile: {}", profile);
                return Ok(path);
            } else {
                tracing::warn!(
                    "Specified profile '{}' not found, falling back to auto-detection",
                    profile
                );
            }
        }

        // 環境変数でプロファイル名を指定可能
        if let Ok(profile_name) = std::env::var("CHROME_PROFILE_NAME") {
            if let Ok(resolver) = crate::chrome_profile::ProfileResolver::new() {
                if let Ok(profile) = resolver.resolve_by_name(&profile_name) {
                    let bookmarks_path = resolver.get_bookmarks_path(&profile);
                    if bookmarks_path.exists() {
                        tracing::info!(
                            "Using Chrome profile '{}' ({})",
                            profile.display_name,
                            profile.directory_name
                        );
                        return Ok(bookmarks_path);
                    }
                }
            }
        }

        // 自動検出: 最もサイズが大きいブックマークファイルを選択
        let home = dirs::home_dir().context("Failed to get home directory")?;
        let chrome_dir = home.join("Library/Application Support/Google/Chrome");

        let mut candidates = Vec::new();

        // 各プロファイルをチェック
        for profile in &["Default", "Profile 1", "Profile 2", "Profile 3"] {
            let path = chrome_dir.join(profile).join("Bookmarks");
            if path.exists() {
                if let Ok(metadata) = fs::metadata(&path) {
                    candidates.push((path, metadata.len(), profile.to_string()));
                }
            }
        }

        // サイズが最大のものを選択（メインで使用している可能性が高い）
        candidates.sort_by_key(|&(_, size, _)| std::cmp::Reverse(size));

        if let Some((path, size, profile)) = candidates.first() {
            tracing::info!(
                "Auto-detected Chrome profile: {} ({}KB)",
                profile,
                size / 1024
            );
            Ok(path.clone())
        } else {
            // フォールバック: Defaultを使用
            let default_path = chrome_dir.join("Default/Bookmarks");
            if default_path.exists() {
                Ok(default_path)
            } else {
                anyhow::bail!(
                    "No Chrome bookmarks file found. Please check if Chrome is installed."
                )
            }
        }
    }

    pub fn read(&self) -> Result<ChromeBookmarks> {
        let content = fs::read_to_string(&self.bookmarks_path)
            .with_context(|| format!("Failed to read bookmarks from {:?}", self.bookmarks_path))?;

        let mut bookmarks: ChromeBookmarks =
            serde_json::from_str(&content).context("Failed to parse bookmarks JSON")?;

        // Initialize folder paths for each root node
        self.initialize_root_folder_paths(&mut bookmarks);

        Ok(bookmarks)
    }

    /// Initialize folder paths for root bookmark nodes, handling Japanese environment
    fn initialize_root_folder_paths(&self, bookmarks: &mut ChromeBookmarks) {
        // Define root folder mappings (English canonical name -> Japanese display name)
        const BOOKMARK_BAR_JP: &str = "ブックマーク バー";
        const OTHER_BOOKMARKS_JP: &str = "その他のブックマーク";
        const SYNCED_BOOKMARKS_JP: &str = "モバイルのブックマーク";

        // Helper function to set up root folder with proper path
        fn setup_root_folder(node: &mut BookmarkNode, canonical_name: &str, japanese_name: &str) {
            if node.name == japanese_name {
                // Japanese environment: set dual-language path
                node.folder_path = vec![canonical_name.to_string(), japanese_name.to_string()];
                if let Some(children) = &mut node.children {
                    for child in children {
                        child.set_folder_paths(vec![
                            canonical_name.to_string(),
                            japanese_name.to_string(),
                        ]);
                    }
                }
            } else {
                // English environment: use standard path
                node.set_folder_paths(vec![canonical_name.to_string()]);
            }
        }

        setup_root_folder(
            &mut bookmarks.roots.bookmark_bar,
            "Bookmarks Bar",
            BOOKMARK_BAR_JP,
        );
        setup_root_folder(
            &mut bookmarks.roots.other,
            "Other Bookmarks",
            OTHER_BOOKMARKS_JP,
        );
        setup_root_folder(
            &mut bookmarks.roots.synced,
            "Synced Bookmarks",
            SYNCED_BOOKMARKS_JP,
        );
    }

    pub fn get_all_bookmarks(&self) -> Result<Vec<FlatBookmark>> {
        // target_folderが指定されている場合は特定フォルダのみ取得
        if let Some(target_folder) = &self.config.target_folder {
            return self.get_folder_bookmarks_by_name(target_folder);
        }

        let bookmarks = self.read()?;

        // Collect all bookmarks from all root nodes
        let mut all = Vec::new();
        all.extend(bookmarks.roots.bookmark_bar.flatten());
        all.extend(bookmarks.roots.other.flatten());
        all.extend(bookmarks.roots.synced.flatten());

        // Apply filtering and limits
        let filtered = self.apply_folder_filter(all);
        Ok(self.apply_max_limit(filtered))
    }

    /// Apply maximum bookmark limit if configured
    fn apply_max_limit(&self, bookmarks: Vec<FlatBookmark>) -> Vec<FlatBookmark> {
        if self.config.max_bookmarks > 0 && bookmarks.len() > self.config.max_bookmarks {
            bookmarks
                .into_iter()
                .take(self.config.max_bookmarks)
                .collect()
        } else {
            bookmarks
        }
    }

    pub fn search_bookmarks(&self, query: &str) -> Result<Vec<FlatBookmark>> {
        let all_bookmarks = self.get_all_bookmarks()?;
        let query_lower = query.to_lowercase();

        // Search in both bookmark name and URL
        let results = all_bookmarks
            .into_iter()
            .filter(|bookmark| {
                bookmark.name.to_lowercase().contains(&query_lower)
                    || bookmark.url.to_lowercase().contains(&query_lower)
            })
            .collect();

        Ok(results)
    }

    pub fn get_folder_bookmarks(&self, folder_path: &[String]) -> Result<Vec<FlatBookmark>> {
        if folder_path.is_empty() {
            return self.get_all_bookmarks();
        }

        let bookmarks = self.read()?;

        // Find the appropriate root node and adjust path for Japanese environment
        let (root_node, adjusted_path) = self.find_root_node_and_path(&bookmarks, folder_path);

        match root_node.and_then(|node| node.find_folder(adjusted_path)) {
            Some(n) => {
                let all_bookmarks = n.flatten();
                // Apply filtering based on configuration
                Ok(self.apply_folder_filter(all_bookmarks))
            }
            None => Ok(Vec::new()),
        }
    }

    /// Find the appropriate root node and adjust the folder path for Japanese environment
    fn find_root_node_and_path<'a, 'b>(
        &self,
        bookmarks: &'a ChromeBookmarks,
        folder_path: &'b [String],
    ) -> (Option<&'a BookmarkNode>, &'b [String]) {
        if folder_path.is_empty() {
            return (None, &[]);
        }

        // Japanese folder name constants
        const BOOKMARK_BAR_JP: &str = "ブックマーク バー";
        const OTHER_BOOKMARKS_JP: &str = "その他のブックマーク";
        const SYNCED_BOOKMARKS_JP: &str = "モバイルのブックマーク";

        let (root_node, japanese_name) = match folder_path[0].as_str() {
            "Bookmarks Bar" => (Some(&bookmarks.roots.bookmark_bar), BOOKMARK_BAR_JP),
            "Other Bookmarks" => (Some(&bookmarks.roots.other), OTHER_BOOKMARKS_JP),
            "Synced Bookmarks" => (Some(&bookmarks.roots.synced), SYNCED_BOOKMARKS_JP),
            _ => return (None, &[]),
        };

        // Skip Japanese folder name if present in path
        let adjusted_path = if folder_path.len() > 1 && folder_path[1] == japanese_name {
            &folder_path[2..]
        } else {
            &folder_path[1..]
        };

        (root_node, adjusted_path)
    }

    /// Apply folder filtering based on configuration
    fn apply_folder_filter(&self, bookmarks: Vec<FlatBookmark>) -> Vec<FlatBookmark> {
        bookmarks
            .into_iter()
            .filter(|b| self.config.should_include_folder(&b.folder_path))
            .collect()
    }

    /// List bookmark folders with configuration filtering applied
    pub fn list_filtered_folders(&self) -> Result<Vec<Vec<String>>> {
        let bookmarks = self.read()?;
        let all_folders = self.list_all_folders_internal(&bookmarks)?;

        // Apply folder filtering based on configuration
        let filtered = all_folders
            .into_iter()
            .filter(|folder_path| self.config.should_include_folder(folder_path))
            .collect();

        Ok(filtered)
    }

    fn list_all_folders_internal(&self, bookmarks: &ChromeBookmarks) -> Result<Vec<Vec<String>>> {
        let mut folders = Vec::new();

        folders.extend(bookmarks.roots.bookmark_bar.collect_all_folders());
        folders.extend(bookmarks.roots.other.collect_all_folders());
        folders.extend(bookmarks.roots.synced.collect_all_folders());

        Ok(folders)
    }

    /// フォルダ名で検索してブックマークを取得（言語非依存）
    pub fn get_folder_bookmarks_by_name(&self, folder_name: &str) -> Result<Vec<FlatBookmark>> {
        let bookmarks = self.read()?;

        // 各ルートノードから検索
        let folders = vec![
            bookmarks
                .roots
                .bookmark_bar
                .find_folder_by_name(folder_name),
            bookmarks.roots.other.find_folder_by_name(folder_name),
            bookmarks.roots.synced.find_folder_by_name(folder_name),
        ];

        // 最初に見つかったフォルダのブックマークを返す
        if let Some(folder) = folders.into_iter().flatten().next() {
            tracing::info!("Found folder '{}' in bookmarks", folder_name);
            let results = folder.flatten();
            return Ok(self.apply_max_limit(results));
        }

        tracing::warn!("Folder '{}' not found in bookmarks", folder_name);
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_node_is_folder() {
        let node = BookmarkNode {
            children: None,
            date_added: None,
            date_last_used: None,
            date_modified: None,
            guid: "test".to_string(),
            id: "1".to_string(),
            name: "Test Folder".to_string(),
            node_type: "folder".to_string(),
            url: None,
            folder_path: vec![],
        };
        assert!(node.is_folder());
        assert!(!node.is_url());
    }

    #[test]
    fn test_bookmark_node_is_url() {
        let node = BookmarkNode {
            children: None,
            date_added: None,
            date_last_used: None,
            date_modified: None,
            guid: "test".to_string(),
            id: "1".to_string(),
            name: "Test URL".to_string(),
            node_type: "url".to_string(),
            url: Some("https://example.com".to_string()),
            folder_path: vec![],
        };
        assert!(!node.is_folder());
        assert!(node.is_url());
    }
}
