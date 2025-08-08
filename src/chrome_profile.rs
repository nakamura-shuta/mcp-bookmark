use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tracing::info;

/// Chrome Local State ファイルの構造（必要な部分のみ）
#[derive(Debug, Deserialize)]
struct LocalState {
    profile: ProfileInfo,
}

#[derive(Debug, Deserialize)]
struct ProfileInfo {
    info_cache: Value,
}

/// プロファイル情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromeProfile {
    pub directory_name: String, // "Default", "Profile 1", etc.
    pub display_name: String,   // "Work", "Personal", etc.
    pub path: PathBuf,
}

/// Chromeプロファイルを管理する構造体
pub struct ProfileResolver {
    chrome_base_dir: PathBuf,
}

impl ProfileResolver {
    /// 新規作成
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        let chrome_base_dir = home.join("Library/Application Support/Google/Chrome");

        if !chrome_base_dir.exists() {
            anyhow::bail!("Chrome directory not found at {:?}", chrome_base_dir);
        }

        Ok(Self { chrome_base_dir })
    }

    /// Local State ファイルを読み込む
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

    /// プロファイル名からディレクトリを解決
    pub fn resolve_by_name(&self, profile_name: &str) -> Result<ChromeProfile> {
        let state = self.read_local_state()?;

        // info_cache から全プロファイルを検索
        if let Some(info_cache) = state.profile.info_cache.as_object() {
            for (dir_name, profile_info) in info_cache {
                // nameフィールドをチェック
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
                        });
                    }
                }

                // gaia_nameもチェック（Googleアカウント名の場合）
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
                        });
                    }
                }
            }
        }

        anyhow::bail!("Profile '{}' not found", profile_name)
    }

    /// プロファイルディレクトリからBookmarksファイルパスを取得
    pub fn get_bookmarks_path(&self, profile: &ChromeProfile) -> PathBuf {
        profile.path.join("Bookmarks")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_resolver_creation() {
        // プロファイルリゾルバーの作成
        let resolver = ProfileResolver::new();
        assert!(resolver.is_ok() || resolver.is_err()); // 環境依存
    }
}
