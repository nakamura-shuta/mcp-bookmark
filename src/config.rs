use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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
    /// 設定ファイルから読み込み
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 環境変数から読み込み
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // MCP_BOOKMARK_INCLUDE_FOLDERS="Bookmarks Bar/Development,Other Bookmarks/Work"
        if let Ok(include) = std::env::var("MCP_BOOKMARK_INCLUDE_FOLDERS") {
            config.include_folders = include
                .split(',')
                .map(|path| path.split('/').map(String::from).collect())
                .collect();
        }

        // MCP_BOOKMARK_EXCLUDE_FOLDERS="Bookmarks Bar/Personal"
        if let Ok(exclude) = std::env::var("MCP_BOOKMARK_EXCLUDE_FOLDERS") {
            config.exclude_folders = exclude
                .split(',')
                .map(|path| path.split('/').map(String::from).collect())
                .collect();
        }

        // MCP_BOOKMARK_MAX=100
        if let Ok(max) = std::env::var("MCP_BOOKMARK_MAX") {
            if let Ok(max_num) = max.parse() {
                config.max_bookmarks = max_num;
            }
        }

        config
    }

    /// デフォルト設定ファイルパスを取得
    pub fn default_config_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|p| p.join("mcp-bookmark").join("config.json"))
    }

    /// 設定を読み込み（ファイル → 環境変数 → デフォルトの優先順位）
    pub fn load() -> Self {
        // 1. 設定ファイルを探す
        if let Some(config_path) = Self::default_config_path() {
            if config_path.exists() {
                if let Ok(config) = Self::from_file(&config_path) {
                    return config;
                }
            }
        }

        // 2. 環境変数から読み込み
        let env_config = Self::from_env();
        if !env_config.include_folders.is_empty() || !env_config.exclude_folders.is_empty() {
            return env_config;
        }

        // 3. デフォルト設定
        Self::default()
    }

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

/// サンプル設定ファイルを生成
pub fn generate_sample_config() -> String {
    let sample = Config {
        include_folders: vec![
            vec!["Bookmarks Bar".to_string(), "Development".to_string()],
            vec!["Bookmarks Bar".to_string(), "Work".to_string()],
        ],
        exclude_folders: vec![vec!["Bookmarks Bar".to_string(), "Personal".to_string()]],
        max_bookmarks: 100,
        profile_name: Some("Nakamura".to_string()),
        target_folder: Some("hoge".to_string()),
    };

    serde_json::to_string_pretty(&sample).unwrap()
}
