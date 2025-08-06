use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
    pub fn new() -> Result<Self> {
        let bookmarks_path = Self::find_bookmarks_path()?;
        Ok(Self {
            bookmarks_path,
            config: Config::default(),
        })
    }

    pub fn with_config(config: Config) -> Result<Self> {
        let bookmarks_path = Self::find_bookmarks_path()?;
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

    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            bookmarks_path: path.as_ref().to_path_buf(),
            config: Config::default(),
        }
    }

    /// 指定されたフォルダが存在するか検証
    pub fn validate_folders(&self) -> Result<Vec<String>> {
        let bookmarks = self.read()?;
        let all_folders = self.list_all_folders_internal(&bookmarks)?;
        let mut warnings = Vec::new();

        // include_foldersの検証
        for include in &self.config.include_folders {
            if !all_folders
                .iter()
                .any(|f| f == include || f.starts_with(include))
            {
                warnings.push(format!("Warning: Include folder not found: {include:?}"));
            }
        }

        // exclude_foldersの検証
        for exclude in &self.config.exclude_folders {
            if !all_folders
                .iter()
                .any(|f| f == exclude || f.starts_with(exclude))
            {
                warnings.push(format!("Warning: Exclude folder not found: {exclude:?}"));
            }
        }

        Ok(warnings)
    }

    /// 利用可能なフォルダ一覧を表示用に取得
    pub fn get_available_folders(&self) -> Result<Vec<Vec<String>>> {
        let bookmarks = self.read()?;
        self.list_all_folders_internal(&bookmarks)
    }

    pub fn read(&self) -> Result<ChromeBookmarks> {
        let content = fs::read_to_string(&self.bookmarks_path)
            .with_context(|| format!("Failed to read bookmarks from {:?}", self.bookmarks_path))?;

        let mut bookmarks: ChromeBookmarks =
            serde_json::from_str(&content).context("Failed to parse bookmarks JSON")?;

        bookmarks
            .roots
            .bookmark_bar
            .set_folder_paths(vec!["Bookmarks Bar".to_string()]);
        bookmarks
            .roots
            .other
            .set_folder_paths(vec!["Other Bookmarks".to_string()]);
        bookmarks
            .roots
            .synced
            .set_folder_paths(vec!["Synced Bookmarks".to_string()]);

        Ok(bookmarks)
    }

    pub fn get_all_bookmarks(&self) -> Result<Vec<FlatBookmark>> {
        let bookmarks = self.read()?;
        let mut all = Vec::new();

        all.extend(bookmarks.roots.bookmark_bar.flatten());
        all.extend(bookmarks.roots.other.flatten());
        all.extend(bookmarks.roots.synced.flatten());

        // フィルタリング適用
        let filtered: Vec<FlatBookmark> = all
            .into_iter()
            .filter(|b| self.config.should_include_folder(&b.folder_path))
            .collect();

        // 最大数制限
        if self.config.max_bookmarks > 0 && filtered.len() > self.config.max_bookmarks {
            Ok(filtered
                .into_iter()
                .take(self.config.max_bookmarks)
                .collect())
        } else {
            Ok(filtered)
        }
    }

    pub fn search_bookmarks(&self, query: &str) -> Result<Vec<FlatBookmark>> {
        let all_bookmarks = self.get_all_bookmarks()?;
        let query_lower = query.to_lowercase();

        Ok(all_bookmarks
            .into_iter()
            .filter(|bookmark| {
                bookmark.name.to_lowercase().contains(&query_lower)
                    || bookmark.url.to_lowercase().contains(&query_lower)
            })
            .collect())
    }

    pub fn get_folder_bookmarks(&self, folder_path: &[String]) -> Result<Vec<FlatBookmark>> {
        let bookmarks = self.read()?;

        let node = if folder_path.is_empty() {
            return self.get_all_bookmarks();
        } else if folder_path[0] == "Bookmarks Bar" {
            bookmarks.roots.bookmark_bar.find_folder(&folder_path[1..])
        } else if folder_path[0] == "Other Bookmarks" {
            bookmarks.roots.other.find_folder(&folder_path[1..])
        } else if folder_path[0] == "Synced Bookmarks" {
            bookmarks.roots.synced.find_folder(&folder_path[1..])
        } else {
            None
        };

        match node {
            Some(n) => Ok(n.flatten()),
            None => Ok(Vec::new()),
        }
    }

    pub fn list_all_folders(&self) -> Result<Vec<Vec<String>>> {
        let bookmarks = self.read()?;
        self.list_all_folders_internal(&bookmarks)
    }

    fn list_all_folders_internal(&self, bookmarks: &ChromeBookmarks) -> Result<Vec<Vec<String>>> {
        let mut folders = Vec::new();

        folders.extend(bookmarks.roots.bookmark_bar.collect_all_folders());
        folders.extend(bookmarks.roots.other.collect_all_folders());
        folders.extend(bookmarks.roots.synced.collect_all_folders());

        Ok(folders)
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
