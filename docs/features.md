# Chrome Bookmark MCP Server - 機能仕様書

## 概要

Chrome Bookmark MCP Serverは、Model Context Protocol (MCP)を通じてChromeのブックマークデータへの高度なアクセスを提供するRust製サーバーです。tantivy全文検索エンジンの統合により、開発者のブックマークコレクションから関連するドキュメントやリソースを瞬時に検索し、AIアシスタントに適切なコンテキストを提供します。

## 主要機能

### 1. 🔍 全文検索機能（tantivy powered）

#### search_bookmarks_fulltext
高速全文検索エンジンtantivyを使用した強力な検索機能です。

**特徴：**
- ブックマークのタイトル、URL、コンテンツを横断検索
- ミリ秒単位の高速レスポンス
- スコアリングによる関連性順位付け
- 大規模ブックマーク（10,000件以上）にも対応
- バックグラウンドでコンテンツを自動インデックス化
- 優先度付き処理（ドキュメントサイトを優先）

**フィルタリングオプション：**
```json
{
  "query": "React hooks",        // 検索クエリ
  "folder": "Bookmarks Bar/Tech", // フォルダパスフィルター
  "domain": "reactjs.org",        // ドメインフィルター
  "limit": 20                     // 結果件数制限
}
```

**使用例：**
- 「React hooksの公式ドキュメントを探す」
- 「github.comにあるRustのサンプルコードを検索」
- 「Tech/AIフォルダ内のLLM関連記事を取得」

### 2. 📚 ブックマーク管理機能

#### search_bookmarks
シンプルなキーワード検索機能。タイトルとURLを対象とした基本検索を提供。

**特徴：**
- 大文字小文字を区別しない検索
- 部分一致検索
- 設定に基づくフォルダフィルタリング

#### list_bookmark_folders
利用可能なブックマークフォルダの階層構造を取得。

**特徴：**
- 完全なフォルダパスの一覧
- 設定による除外フォルダのフィルタリング
- 日本語フォルダ名のサポート（「ブックマーク バー」等）

**出力例：**
```json
[
  ["Bookmarks Bar"],
  ["Bookmarks Bar", "Tech"],
  ["Bookmarks Bar", "Tech", "AI"],
  ["Bookmarks Bar", "ブックマーク バー", "開発"]
]
```

### 3. 🌐 コンテンツ取得機能

#### get_bookmark_content
指定されたURLからWebページのコンテンツとメタデータを取得。

**取得情報：**
- ページタイトル
- メタディスクリプション
- Open Graphメタデータ
- 本文テキスト（HTMLタグ除去済み）
- コードブロック

**特徴：**
- 非同期処理による高速取得
- HTMLパースとテキスト抽出
- メタデータの自動解析

### 4. 🗂️ リソース管理

MCPプロトコルのリソース機能を活用し、ブックマークをAIがアクセス可能なリソースとして提供。

**リソース形式：**
```
URI: bookmark://[folder-path]/[bookmark-id]
例: bookmark://tech/ai/12345
```

**特徴：**
- フォルダ階層に基づくURI構造
- フィルタリング設定の適用
- JSONフォーマットでの詳細情報提供

## システム機能

### 🇯🇵 日本語環境サポート

- 日本語版Chromeの「ブックマーク バー」フォルダ名を自動認識
- 日本語フォルダパスの正確な処理
- UTF-8エンコーディングの完全サポート

### ⚙️ 設定とカスタマイズ

**コマンドライン引数：**
```bash
# 基本起動
cargo run --release

# フォルダ指定
cargo run --release -- "Tech/AI" 50

# 除外フォルダ指定
cargo run --release -- --exclude "Private" --exclude "Archive"

# 最大ブックマーク数制限
cargo run --release -- --max 1000
```

**設定オプション：**
- `include_folders`: 含めるフォルダパス（カンマ区切り）
- `exclude_folders`: 除外するフォルダパス
- `max_bookmarks`: 最大ブックマーク数（0=無制限）

### 🚀 パフォーマンス

#### インデックス性能
- **起動時間**: 10,000件のブックマークを1秒以内でインデックス化
- **メモリ使用**: 最小15MBのヒープサイズで動作
- **検索速度**: 全文検索クエリを100ms以内で処理

#### スケーラビリティ
- 大規模ブックマーク（50,000件以上）のサポート
- 効率的なメモリ管理
- 並行処理による高速化

## ユースケース

### 開発者向けドキュメント検索
```
「React useEffectの使い方を教えて」
→ ブックマークからReact公式ドキュメントを自動検索・参照
```

### プロジェクト固有のリソース管理
```
「現在のプロジェクトで使用しているAWSサービスのドキュメントを探して」
→ AWSフォルダ内のブックマークを検索・フィルタリング
```

### 技術スタック別の情報取得
```
「Rustのエラーハンドリングについての記事を見つけて」
→ rust-lang.orgドメインフィルターで公式リソースを優先検索
```

### チーム知識の共有
```
「チームで共有しているベストプラクティスの記事を表示」
→ Team/Best Practicesフォルダ内のブックマークを一覧表示
```

## 技術スタック

- **言語**: Rust (Edition 2024)
- **検索エンジン**: tantivy 0.24
- **プロトコル**: Model Context Protocol (MCP)
- **非同期処理**: tokio
- **HTTPクライアント**: reqwest
- **HTMLパース**: scraper

## 制限事項と注意点

1. **読み取り専用**: ブックマークの追加・編集・削除は非対応
2. **macOS専用**: Chrome bookmarkファイルパスがmacOS固有
3. **認証非対応**: 認証が必要なWebページのコンテンツ取得は不可
4. **リアルタイム更新なし**: ブックマーク変更の自動検知は未実装

## 今後の拡張予定

- **増分インデックス更新**: ブックマーク変更の自動検知と差分更新
- **コンテンツインデックス**: Webページ内容の事前取得とインデックス化
- **キャッシュ機能**: 頻繁にアクセスされるコンテンツのメモリキャッシュ
- **セマンティック検索**: ベクトル埋め込みによる意味的類似検索
- **マルチプロファイル対応**: 複数のChromeプロファイルの同時サポート

## APIリファレンス

### Tools (MCP Tools)

| ツール名 | 説明 | パラメータ |
|---------|------|-----------|
| `search_bookmarks` | 基本的なキーワード検索 | `query: string` |
| `search_bookmarks_fulltext` | tantivy全文検索 | `query: string`, `folder?: string`, `domain?: string`, `limit?: number` |
| `search_by_content` | ページコンテンツのみ検索 | `query: string`, `limit?: number` |
| `list_bookmark_folders` | フォルダ一覧取得 | なし |
| `get_indexing_status` | インデックス構築状況確認 | なし |
| `get_bookmark_content` | Webコンテンツ取得 | `url: string` |

### Resources (MCP Resources)

| リソースタイプ | URI形式 | 内容 |
|---------------|---------|------|
| Bookmark | `bookmark://[path]/[id]` | ブックマーク詳細情報（JSON） |

## サンプルコード

### MCPクライアントからの使用例

```javascript
// 全文検索の実行
const results = await mcp.callTool('search_bookmarks_fulltext', {
  query: 'machine learning',
  domain: 'arxiv.org',
  limit: 10
});

// フォルダ内検索
const techDocs = await mcp.callTool('search_bookmarks_fulltext', {
  query: 'typescript',
  folder: 'Bookmarks Bar/Tech/Frontend'
});

// コンテンツ取得
const content = await mcp.callTool('get_bookmark_content', {
  url: 'https://docs.rust-lang.org/book/'
});
```

## トラブルシューティング

### インデックスが作成されない
- ログレベルを`RUST_LOG=debug`に設定して詳細を確認
- ブックマークファイルの読み取り権限を確認

### 検索結果が空
- フォルダフィルタリング設定を確認
- 検索クエリの形式を確認（大文字小文字は区別しない）

### 日本語フォルダが認識されない
- Chromeの言語設定を確認
- 「ブックマーク バー」が正しく設定されているか確認

## ライセンスと貢献

このプロジェクトはオープンソースプロジェクトです。
貢献やフィードバックは[GitHubリポジトリ](https://github.com/yourusername/mcp-bookmark)まで。

---

最終更新: 2025-08-06
バージョン: 0.2.0 (tantivy統合版)