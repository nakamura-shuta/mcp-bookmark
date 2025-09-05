use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tempfile::NamedTempFile;
use tokio::fs;
use tracing::{info, warn};

/// PDF処理モジュール
pub struct PDFProcessor {
    client: reqwest::Client,
    cache_dir: PathBuf,
}

impl PDFProcessor {
    pub async fn new(cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir).await
            .context("Failed to create PDF cache directory")?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (compatible; MCP-Bookmark/1.0)")
            .build()?;

        Ok(Self {
            client,
            cache_dir,
        })
    }

    /// PDFのURLからテキストを抽出
    pub async fn extract_from_url(&self, url: &str) -> Result<String> {
        // キャッシュチェック
        if let Some(cached) = self.get_cached(url).await? {
            info!("Using cached PDF text for: {}", url);
            return Ok(cached);
        }

        info!("Downloading PDF from: {}", url);

        // PDFをダウンロード
        let pdf_bytes = self.download_pdf(url).await?;

        // テキスト抽出
        let text = self.extract_text_from_bytes(&pdf_bytes)?;

        // キャッシュに保存
        self.save_to_cache(url, &text).await?;

        Ok(text)
    }

    /// PDFをダウンロード
    async fn download_pdf(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to download PDF")?;

        // ステータスコードチェック
        if !response.status().is_success() {
            anyhow::bail!("Failed to download PDF: HTTP {}", response.status());
        }

        // Content-Typeチェック
        if let Some(content_type) = response.headers().get("content-type") {
            let ct = content_type.to_str().unwrap_or("");
            if !ct.contains("pdf") && !ct.contains("octet-stream") {
                warn!("Unexpected content-type for PDF: {}", ct);
            }
        }

        // サイズ制限チェック (50MB)
        const MAX_SIZE: usize = 50 * 1024 * 1024;
        if let Some(content_length) = response.content_length() {
            if content_length as usize > MAX_SIZE {
                anyhow::bail!("PDF too large: {} bytes", content_length);
            }
        }

        // バイナリデータとして取得
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// バイト配列からテキスト抽出
    fn extract_text_from_bytes(&self, pdf_bytes: &[u8]) -> Result<String> {
        // 一時ファイルに書き込み
        let mut temp_file = NamedTempFile::new()?;
        use std::io::Write;
        temp_file.write_all(pdf_bytes)?;
        temp_file.flush()?;

        // pdf-extractでテキスト抽出
        let text = pdf_extract::extract_text(temp_file.path())
            .context("Failed to extract text from PDF")?;

        // 後処理
        let cleaned_text = self.clean_text(&text);

        Ok(cleaned_text)
    }

    /// テキストのクリーンアップ
    fn clean_text(&self, text: &str) -> String {
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// キャッシュから取得
    async fn get_cached(&self, url: &str) -> Result<Option<String>> {
        let cache_path = self.cache_path_for_url(url);

        if cache_path.exists() {
            // キャッシュの有効期限チェック (7日)
            if let Ok(metadata) = fs::metadata(&cache_path).await {
                if let Ok(modified) = metadata.modified() {
                    let age = SystemTime::now().duration_since(modified)?;
                    if age < Duration::from_secs(7 * 24 * 3600) {
                        let content = fs::read_to_string(&cache_path).await?;
                        return Ok(Some(content));
                    }
                }
            }
        }

        Ok(None)
    }

    /// キャッシュに保存
    async fn save_to_cache(&self, url: &str, text: &str) -> Result<()> {
        let cache_path = self.cache_path_for_url(url);
        fs::write(&cache_path, text).await?;
        Ok(())
    }

    /// URLからキャッシュパスを生成
    fn cache_path_for_url(&self, url: &str) -> PathBuf {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        self.cache_dir.join(format!("{}.txt", hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clean_text() {
        let processor = PDFProcessor::new(PathBuf::from("/tmp/test")).await.unwrap();
        let dirty_text = "  Hello   \n\n  World  \r\n  Test  ";
        let clean = processor.clean_text(dirty_text);
        assert_eq!(clean, "Hello World Test");
    }
}