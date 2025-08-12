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

## インデックス管理

### 保存場所と命名規則

インデックスはプロファイルとフォルダの組み合わせごとに独立管理されます：

```
~/Library/Application Support/mcp-bookmark/
├── Default_Development/      # Defaultプロファイル、Developmentフォルダ
├── Work_Tech_React/         # Workプロファイル、Tech/Reactフォルダ（/は_に変換）
├── Personal_all/            # Personalプロファイル、全ブックマーク
└── logs/                    # ログファイル
```

### インデックス分離の仕組み

1. **キー生成**: `{プロファイル名}_{フォルダ名}` 形式
   - プロファイル未指定時: "Default"
   - フォルダ未指定時: "all"
   - スラッシュ（/）はアンダースコア（_）に変換

2. **共有と分離**:
   - 同じプロファイル・フォルダ設定なら、複数プロジェクトで同じインデックスを共有
   - 異なる設定は完全に独立したインデックスを使用
   - プロジェクト間での干渉なし

3. **メタデータ管理**: 各インデックスディレクトリに`meta.json`を保存
   ```json
   {
     "version": "1.0.0",
     "profile": "Work",
     "folder": "Development",
     "created_at": "2025-01-12T10:00:00Z",
     "last_updated": "2025-01-12T14:00:00Z",
     "bookmark_count": 150,
     "indexed_count": 145,
     "index_size_bytes": 524288
   }
   ```

### 管理コマンド

```bash
mcp-bookmark --list-indexes          # インデックス一覧表示
mcp-bookmark --clear-index [key]     # 特定インデックスをクリア
mcp-bookmark --clear-all-indexes     # 全インデックスをクリア
```