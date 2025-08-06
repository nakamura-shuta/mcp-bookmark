use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{Semaphore, Mutex};
use tokio::time::{timeout, Duration};
use anyhow::Result;
use tracing::{info, debug, warn};

use crate::bookmark::{BookmarkReader, FlatBookmark};
use crate::content::ContentFetcher;
use super::{SearchManager, SearchResult, SearchParams};

/// ハイブリッド検索マネージャー
#[derive(Debug, Clone)]
pub struct HybridSearchManager {
    /// tantivy検索エンジン
    tantivy_search: Arc<Mutex<SearchManager>>,
    
    /// シンプル検索（フォールバック用）
    simple_search: Arc<BookmarkReader>,
    
    /// コンテンツ取得用
    content_fetcher: Arc<ContentFetcher>,
    
    /// インデックス構築状況
    indexing_status: Arc<IndexingStatus>,
}

/// インデックス構築状況
#[derive(Debug)]
pub struct IndexingStatus {
    /// 総ブックマーク数
    pub total: AtomicUsize,
    
    /// 完了数
    pub completed: AtomicUsize,
    
    /// エラー数
    pub errors: AtomicUsize,
    
    /// 完了フラグ
    pub is_complete: AtomicBool,
    
    /// 開始時刻
    pub started_at: std::time::Instant,
}

impl IndexingStatus {
    pub fn new(total: usize) -> Self {
        Self {
            total: AtomicUsize::new(total),
            completed: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            is_complete: AtomicBool::new(false),
            started_at: std::time::Instant::now(),
        }
    }
    
    /// 進捗率を取得（0.0 - 100.0）
    pub fn progress_percentage(&self) -> f64 {
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            return 100.0;
        }
        let completed = self.completed.load(Ordering::Relaxed);
        (completed as f64 / total as f64) * 100.0
    }
    
    /// ステータス文字列を生成
    pub fn status_string(&self) -> String {
        let total = self.total.load(Ordering::Relaxed);
        let completed = self.completed.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        let elapsed = self.started_at.elapsed();
        
        if self.is_complete.load(Ordering::Relaxed) {
            format!(
                "✅ インデックス構築完了: {}/{} 成功, {} エラー (所要時間: {:.1}秒)",
                completed - errors, total, errors, elapsed.as_secs_f64()
            )
        } else {
            let eta = if completed > 0 {
                let per_item = elapsed.as_secs_f64() / completed as f64;
                let remaining = total - completed;
                Duration::from_secs_f64(per_item * remaining as f64)
            } else {
                Duration::from_secs(0)
            };
            
            format!(
                "📥 インデックス構築中: {}/{} ({:.1}%), {} エラー, 推定残り時間: {:.0}秒",
                completed, total, self.progress_percentage(), errors, eta.as_secs()
            )
        }
    }
}

impl HybridSearchManager {
    /// 新規作成
    pub async fn new(
        reader: Arc<BookmarkReader>,
        fetcher: Arc<ContentFetcher>,
    ) -> Result<Self> {
        // ブックマーク取得
        let bookmarks = reader.get_all_bookmarks()?;
        let total = bookmarks.len();
        
        info!("📚 ハイブリッド検索マネージャーを初期化中 ({}件のブックマーク)", total);
        
        // SearchManager作成
        let mut search_manager = SearchManager::new(None)?;
        
        // Step 1: メタデータのみを即座にインデックス
        info!("📝 メタデータをインデックス化中...");
        search_manager.build_index(&bookmarks)?;
        
        // マネージャー作成
        let manager = Self {
            tantivy_search: Arc::new(Mutex::new(search_manager)),
            simple_search: reader,
            content_fetcher: fetcher,
            indexing_status: Arc::new(IndexingStatus::new(total)),
        };
        
        // Step 2: バックグラウンドでコンテンツ取得開始
        manager.start_background_indexing(bookmarks).await;
        
        Ok(manager)
    }
    
    /// バックグラウンドでコンテンツをインデックス化
    async fn start_background_indexing(&self, bookmarks: Vec<FlatBookmark>) {
        let search_manager = self.tantivy_search.clone();
        let fetcher = self.content_fetcher.clone();
        let status = self.indexing_status.clone();
        
        tokio::spawn(async move {
            info!("🚀 バックグラウンドインデックス構築を開始");
            
            // 優先度でソート（重要なドメインを先に）
            let mut bookmarks = bookmarks;
            bookmarks.sort_by_key(|b| {
                let domain = extract_domain(&b.url).unwrap_or_default();
                match domain.as_str() {
                    // ドキュメントサイトは最優先
                    "docs.rs" | "doc.rust-lang.org" => 0,
                    "react.dev" | "reactjs.org" => 1,
                    "developer.mozilla.org" => 2,
                    "docs.github.com" => 3,
                    "docs.aws.amazon.com" => 4,
                    // 技術ブログ
                    "medium.com" | "dev.to" => 10,
                    "stackoverflow.com" => 11,
                    // その他
                    _ => 100,
                }
            });
            
            // 並列度制限（10並列）
            let semaphore = Arc::new(Semaphore::new(10));
            let mut handles = vec![];
            
            for bookmark in bookmarks {
                let sem = semaphore.clone();
                let search = search_manager.clone();
                let fetcher = fetcher.clone();
                let status = status.clone();
                
                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    
                    debug!("📄 コンテンツ取得開始: {}", bookmark.url);
                    
                    // コンテンツ取得（タイムアウト3秒）
                    let fetch_result = timeout(
                        Duration::from_secs(3),
                        fetcher.fetch_page(&bookmark.url)
                    ).await;
                    
                    match fetch_result {
                        Ok(Ok(html)) => {
                            // コンテンツ抽出
                            let content = fetcher.extract_content(&html);
                            
                            // tantivyインデックスを更新
                            let mut search = search.lock().await;
                            let content_text = content.text_content.as_deref();
                            if let Err(e) = search.update_bookmark(
                                &bookmark,
                                content_text
                            ) {
                                warn!("インデックス更新失敗 {}: {}", bookmark.url, e);
                                status.errors.fetch_add(1, Ordering::Relaxed);
                            } else {
                                debug!("✅ インデックス更新成功: {}", bookmark.url);
                            }
                        }
                        Ok(Err(e)) => {
                            debug!("コンテンツ取得失敗 {}: {}", bookmark.url, e);
                            status.errors.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(_) => {
                            debug!("タイムアウト: {}", bookmark.url);
                            status.errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    
                    // 進捗更新
                    let completed = status.completed.fetch_add(1, Ordering::Relaxed) + 1;
                    let total = status.total.load(Ordering::Relaxed);
                    
                    // 進捗表示
                    info!("{}", status.status_string());
                    
                    if completed == total {
                        status.is_complete.store(true, Ordering::Relaxed);
                        info!("🎉 {}", status.status_string());
                    }
                });
                
                handles.push(handle);
            }
            
            // 全タスク完了を待つ
            for handle in handles {
                let _ = handle.await;
            }
        });
    }
    
    /// ハイブリッド検索実行
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // 1. まずtantivyで検索
        let tantivy_results = {
            let search = self.tantivy_search.lock().await;
            search.search(query, limit)?
        };
        
        if !tantivy_results.is_empty() {
            debug!("Tantivyでヒット: {}件", tantivy_results.len());
            return Ok(tantivy_results);
        }
        
        // 2. tantivyでヒットなし、インデックス構築中ならフォールバック
        if !self.indexing_status.is_complete.load(Ordering::Relaxed) {
            info!(
                "Tantivyでヒットなし。{} フォールバック検索を使用",
                self.indexing_status.status_string()
            );
            
            // シンプル検索を実行
            let simple_results = self.simple_search.search_bookmarks(query)?;
            
            // SearchResult形式に変換
            let results: Vec<SearchResult> = simple_results
                .into_iter()
                .take(limit)
                .map(|b| SearchResult {
                    id: b.id,
                    url: b.url.clone(),
                    title: b.name,
                    folder_path: b.folder_path.join("/"),
                    domain: extract_domain(&b.url).unwrap_or_default(),
                    score: 1.0, // フォールバック検索は固定スコア
                    date_added: parse_date(&b.date_added).unwrap_or(0),
                    date_modified: parse_date(&b.date_modified).unwrap_or(0),
                })
                .collect();
                
            debug!("フォールバック検索でヒット: {}件", results.len());
            return Ok(results);
        }
        
        // 3. インデックス構築完了後でもヒットなし
        debug!("検索結果なし（インデックス構築完了済み）");
        Ok(vec![])
    }
    
    /// 高度な検索（フィルター付き）
    pub async fn search_advanced(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        // tantivyのみ使用（フィルター検索はtantivyの機能）
        let search = self.tantivy_search.lock().await;
        search.search_advanced(params)
    }
    
    /// インデックス構築状況を取得
    pub fn get_indexing_status(&self) -> String {
        self.indexing_status.status_string()
    }
    
    /// インデックス構築が完了しているか
    pub fn is_indexing_complete(&self) -> bool {
        self.indexing_status.is_complete.load(Ordering::Relaxed)
    }
}

/// ドメイン抽出ヘルパー
fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
}

/// 日付パースヘルパー
fn parse_date(date: &Option<String>) -> Option<i64> {
    date.as_ref()?.parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_hybrid_search_manager_creation() {
        // テスト用の設定
        let config = crate::config::Config::default();
        let reader = Arc::new(BookmarkReader::with_config(config).unwrap());
        let fetcher = Arc::new(ContentFetcher::new().unwrap());
        
        // ハイブリッド検索マネージャー作成
        let manager = HybridSearchManager::new(reader, fetcher).await.unwrap();
        
        // インデックス構築状況を確認
        assert!(!manager.is_indexing_complete());
        let status = manager.get_indexing_status();
        assert!(status.contains("インデックス構築"));
    }
}