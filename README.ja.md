日本語 | [English](README.md)

# Chrome Bookmark MCP Server

**ブックマークしたページの内容を AI で検索** - ログイン必須サイトも Chrome 拡張機能でインデックス化、高速全文検索を実現

💡 **主な特徴**:

- 🔐 **認証が必要なページも OK** - Chrome 拡張機能がログイン済みブラウザから直接コンテンツを取得
- ⚡ **高速ローカル検索** - Tantivy エンジンでインデックス化、外部 API 不要
- 🎯 **AI が理解しやすい** - Claude がブックマークの内容を検索して質問に答える

**⚠️ 動作環境**: macOS + Google Chrome

## 機能

- **全文検索**: Tantivy 検索エンジンによるコンテンツ検索
- **Chrome 拡張機能**: ブラウザから直接コンテンツをインデックス化
- **カスタムインデックス**: 複数の独立したインデックスを作成・管理可能
- **フォルダフィルタリング**: 特定のブックマークフォルダのみ公開

## クイックスタート

### 自動インストール（推奨）

```bash
# クローンしてセットアップスクリプトを実行
git clone https://github.com/nakamura-shuta/mcp-bookmark.git
cd mcp-bookmark
./install.sh
```

セットアップスクリプトは以下を行います：

- ✅ 前提条件の確認（macOS、Chrome、Rust）
- ✅ 必要なバイナリのビルド
- ✅ Chrome 拡張機能の設定(手動)
- ✅ ローカル .mcp.json 設定の作成
- ✅ インストールの検証

### 手動インストール

<details>
<summary>手動インストール手順はこちら</summary>

#### 1. サーバーのビルド

```bash
# クローンとビルド
git clone https://github.com/nakamura-shuta/mcp-bookmark.git
cd mcp-bookmark
cargo build --release

# インストール確認
./target/release/mcp-bookmark --help
```

#### 2. Chrome 拡張機能のインストール

1. Native Messaging Host をビルド：

   ```bash
   cargo build --release --bin mcp-bookmark-native
   ```

2. 拡張機能をインストール - [拡張機能 README](bookmark-indexer-extension/README.md) 参照

3. インデックスの確認：
   ```bash
   # 作成されたインデックス一覧を確認
   ./target/release/mcp-bookmark --list-indexes
   # 例: work_Development (123 documents, 5.2MB)
   ```

#### 3. MCP の設定

プロジェクトルートに `.mcp.json` 設定ファイルを作成：

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "./target/release/mcp-bookmark",
      "args": [],
      "env": {
        "RUST_LOG": "info",
        "INDEX_NAME": "your-index-name"
      }
    }
  }
}
```

**重要**：

- `your-index-name` を Chrome 拡張機能で作成したインデックス名に置き換えてください
- `./target/release/mcp-bookmark --list-indexes` で利用可能なインデックスを確認できます

</details>

## 使い方

### Chrome 拡張機能を使用（推奨）

1. Chrome 拡張機能のポップアップを開く
2. （オプション）カスタムインデックス名を入力
3. インデックス化するフォルダを選択
4. 「Index Selected Folder」をクリック
5. AI アシスタントでインデックス化されたコンテンツを使用

### コマンドラインオプション

```bash
# 特定のインデックスで MCP サーバーを実行
INDEX_NAME="work_Development" ./target/release/mcp-bookmark

# インデックス管理コマンド
./target/release/mcp-bookmark --list-indexes      # 利用可能なインデックス一覧
./target/release/mcp-bookmark --clear-index       # 現在のインデックスをクリア
./target/release/mcp-bookmark --clear-all-indexes # すべてのインデックスをクリア
```

## 利用可能な MCP ツール

- `search_bookmarks_fulltext` - 全文コンテンツ検索（タイトル、URL、ページ内容を検索）
  - プレビュースニペット（300 文字）を返し、素早い内容確認が可能
  - トークンオーバーフローを防ぐため自動的に制限
  - `limit` パラメータで結果数を制御
- `get_bookmark_content` - 特定 URL の完全なコンテンツ取得
  - 検索後に全ページコンテンツを取得するために使用
  - サイズ制限なし
- `get_indexing_status` - インデックス化の進捗確認

## インデックスの保存場所

インデックスは以下に保存されます：

- macOS: `~/Library/Application Support/mcp-bookmark/`

各インデックスは独立して管理されます。

## ライセンス

MIT
