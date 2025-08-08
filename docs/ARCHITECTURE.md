# アーキテクチャ

## 概要

Chrome Bookmark MCP Serverは、Model Context Protocol (MCP)を通じてChromeブックマークへのアクセスを提供するRustサーバーです。

## コンポーネント

### コア機能
- `bookmark.rs` - Chromeブックマークの読み込みとパース
- `chrome_profile.rs` - Chromeプロファイルの自動検出と管理
- `mcp_server.rs` - MCPプロトコルの実装
- `search/` - tantivy全文検索エンジン統合
- `content.rs` - Webページのメタデータ取得

### 検索システム
```
search/
├── mod.rs           # SearchManager - 検索システムの統合
├── schema.rs        # tantivyスキーマ定義
├── indexer.rs       # BookmarkIndexer - インデックス構築
├── searcher.rs      # BookmarkSearcher - 検索実行
└── content_index.rs # ContentIndexManager - バックグラウンドインデックス
```

### データフロー
1. 起動時にChromeプロファイルを自動検出（最大のBookmarksファイルを持つプロファイル）
2. 選択されたプロファイルのBookmarksファイルを読み込み
3. メタデータを即座にtantivyでインデックス化
4. バックグラウンドでWebコンテンツを取得・インデックス化
5. MCPツール経由で検索クエリを受信・処理

## 検索優先度

ドキュメントサイトを優先的にインデックス化：
1. docs.rs, doc.rust-lang.org
2. react.dev, developer.mozilla.org
3. docs.github.com, docs.aws.amazon.com
4. その他のサイト

## インデックス保存場所

```
~/Library/Application Support/mcp-bookmark/
├── index/     # tantivy検索インデックス
└── logs/      # ログファイル
```