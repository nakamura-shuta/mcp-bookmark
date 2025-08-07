use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// 読み込むフォルダパスのリスト（空の場合は全て読み込む）
    #[serde(default)]
    pub include_folders: Vec<Vec<String>>,

    /// 除外するフォルダパスのリスト
    #[serde(default)]
    pub exclude_folders: Vec<Vec<String>>,

    /// 最大取得ブックマーク数（0は無制限）
    #[serde(default)]
    pub max_bookmarks: usize,

    /// Chromeプロファイル名（表示名）
    #[serde(default)]
    pub profile_name: Option<String>,

    /// 特定フォルダ名で検索（言語非依存）
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
