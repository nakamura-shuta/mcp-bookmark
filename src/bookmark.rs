use crate::config::Config;
use anyhow::Result;
use serde::{Deserialize, Serialize};
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

        tracing::trace!("Looking for folder '{}' in '{}'", path[0], self.name);
        if let Some(children) = &self.children {
            for child in children {
                if child.is_folder() && child.name == path[0] {
                    tracing::trace!(
                        "Found folder '{}', continuing with path: {:?}",
                        child.name,
                        &path[1..]
                    );
                    return child.find_folder(&path[1..]);
                }
            }
        }

        tracing::trace!(
            "Folder '{}' not found in '{}'. Available folders: {:?}",
            path[0],
            self.name,
            self.children
                .as_ref()
                .map(|c| c
                    .iter()
                    .filter(|n| n.is_folder())
                    .map(|n| n.name.as_str())
                    .collect::<Vec<_>>())
                .unwrap_or_default()
        );
        None
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

/// Chrome bookmark reader
#[derive(Debug)]
pub struct BookmarkReader {
    pub bookmarks_path: PathBuf,
    pub config: Config,
}

impl BookmarkReader {
    pub fn with_config(config: Config) -> Result<Self> {
        // With INDEX_NAME approach, BookmarkReader is not used
        // The data comes from the pre-built Tantivy index
        if config.index_name.is_some() {
            // When using INDEX_NAME, don't read Chrome bookmarks
            tracing::debug!(
                "Using pre-built index: {}",
                config.index_name.as_deref().unwrap_or("")
            );
            return Ok(Self {
                bookmarks_path: PathBuf::from("/nonexistent/Bookmarks"),
                config,
            });
        }
        
        // INDEX_NAME is required
        anyhow::bail!("INDEX_NAME environment variable is required. Please specify the index to use.")
    }

    #[cfg(test)]
    pub fn new_with_path(bookmarks_path: PathBuf, config: Config) -> Self {
        Self {
            bookmarks_path,
            config,
        }
    }

    pub fn read(&self) -> Result<ChromeBookmarks> {
        // This should not be called when using INDEX_NAME
        anyhow::bail!("BookmarkReader::read() should not be called when using INDEX_NAME")
    }

    /// Read bookmarks from Chrome, filtering by folder if specified
    pub fn read_bookmarks(&self) -> Result<Vec<FlatBookmark>> {
        // Skip reading if using pre-built index
        if self.config.index_name.is_some() {
            tracing::debug!("Skipping bookmark file read (using pre-built index)");
            return Ok(Vec::new());
        }

        // Should not reach here
        anyhow::bail!("INDEX_NAME is required")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_bookmarks() -> ChromeBookmarks {
        ChromeBookmarks {
            checksum: "test".to_string(),
            version: 1,
            roots: BookmarkRoots {
                bookmark_bar: BookmarkNode {
                    children: Some(vec![
                        BookmarkNode {
                            children: None,
                            date_added: Some("13350000000000000".to_string()),
                            date_last_used: None,
                            date_modified: Some("13350000000000000".to_string()),
                            guid: "test1".to_string(),
                            id: "1".to_string(),
                            name: "Example 1".to_string(),
                            node_type: "url".to_string(),
                            url: Some("https://example.com".to_string()),
                            folder_path: vec![],
                        },
                        BookmarkNode {
                            children: Some(vec![BookmarkNode {
                                children: None,
                                date_added: Some("13350000000000000".to_string()),
                                date_last_used: None,
                                date_modified: Some("13350000000000000".to_string()),
                                guid: "test2".to_string(),
                                id: "2".to_string(),
                                name: "Example 2".to_string(),
                                node_type: "url".to_string(),
                                url: Some("https://example2.com".to_string()),
                                folder_path: vec![],
                            }]),
                            date_added: Some("13350000000000000".to_string()),
                            date_last_used: None,
                            date_modified: Some("13350000000000000".to_string()),
                            guid: "folder1".to_string(),
                            id: "3".to_string(),
                            name: "Folder 1".to_string(),
                            node_type: "folder".to_string(),
                            url: None,
                            folder_path: vec![],
                        },
                    ]),
                    date_added: Some("13350000000000000".to_string()),
                    date_last_used: None,
                    date_modified: Some("13350000000000000".to_string()),
                    guid: "bookmark_bar".to_string(),
                    id: "0".to_string(),
                    name: "Bookmarks Bar".to_string(),
                    node_type: "folder".to_string(),
                    url: None,
                    folder_path: vec![],
                },
                other: BookmarkNode {
                    children: None,
                    date_added: Some("13350000000000000".to_string()),
                    date_last_used: None,
                    date_modified: Some("13350000000000000".to_string()),
                    guid: "other".to_string(),
                    id: "4".to_string(),
                    name: "Other Bookmarks".to_string(),
                    node_type: "folder".to_string(),
                    url: None,
                    folder_path: vec![],
                },
                synced: BookmarkNode {
                    children: None,
                    date_added: Some("13350000000000000".to_string()),
                    date_last_used: None,
                    date_modified: Some("13350000000000000".to_string()),
                    guid: "synced".to_string(),
                    id: "5".to_string(),
                    name: "Mobile Bookmarks".to_string(),
                    node_type: "folder".to_string(),
                    url: None,
                    folder_path: vec![],
                },
            },
        }
    }

    #[test]
    fn test_bookmark_node_flatten() {
        let mut bookmarks = create_test_bookmarks();
        bookmarks
            .roots
            .bookmark_bar
            .set_folder_paths(vec!["Bookmarks Bar".to_string()]);

        let flat = bookmarks.roots.bookmark_bar.flatten();
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].url, "https://example.com");
        assert_eq!(flat[0].folder_path, vec!["Bookmarks Bar"]);
        assert_eq!(flat[1].url, "https://example2.com");
        assert_eq!(flat[1].folder_path, vec!["Bookmarks Bar", "Folder 1"]);
    }

    #[test]
    fn test_find_folder() {
        let mut bookmarks = create_test_bookmarks();
        bookmarks
            .roots
            .bookmark_bar
            .set_folder_paths(vec!["Bookmarks Bar".to_string()]);

        // Find root
        let folder = bookmarks.roots.bookmark_bar.find_folder(&[]);
        assert!(folder.is_some());
        assert_eq!(folder.unwrap().name, "Bookmarks Bar");

        // Find subfolder
        let folder = bookmarks
            .roots
            .bookmark_bar
            .find_folder(&["Folder 1".to_string()]);
        assert!(folder.is_some());
        assert_eq!(folder.unwrap().name, "Folder 1");

        // Non-existent folder
        let folder = bookmarks
            .roots
            .bookmark_bar
            .find_folder(&["NonExistent".to_string()]);
        assert!(folder.is_none());
    }
}