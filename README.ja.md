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
- **複数プロファイル対応**: Chrome の複数プロファイルをサポート
- **フォルダフィルタリング**: 特定のブックマークフォルダのみ公開

## クイックスタート

### 1. サーバーのビルド

```bash
# クローンとビルド
git clone https://github.com/nakamura-shuta/mcp-bookmark.git
cd mcp-bookmark
cargo build --release

# インストール確認
./target/release/mcp-bookmark --help
```

### 2. Chrome 拡張機能のインストール（推奨）

Chrome 拡張機能でより良いコンテンツインデックスを実現：

1. Native Messaging Host をビルド：
   ```bash
   cargo build --release --bin mcp-bookmark-native
   ```

2. 拡張機能をインストール - [拡張機能 README](bookmark-indexer-extension/README.md) 参照

3. インデックスの確認：
   ```bash
   # 作成されたインデックス一覧を確認
   ./target/release/mcp-bookmark --list-indexes
   # 例: Extension_Development (123 documents, 5.2MB)
   ```

### 3. MCP の設定

Claude Desktop の設定ファイル (`~/Library/Application Support/Claude/claude_desktop_config.json`) に追加：

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "env": {
        "CHROME_PROFILE_NAME": "Extension",
        "CHROME_TARGET_FOLDER": "your-folder-name"
      }
    }
  }
}
```

**重要**：
- `/path/to/mcp-bookmark` を実際のプロジェクトパスに置き換えてください
- `your-folder-name` を Chrome 拡張機能でインデックス化したフォルダ名に置き換えてください
- `CHROME_PROFILE_NAME` は Chrome 拡張機能使用時は常に `"Extension"` にしてください

## 使い方

### Chrome 拡張機能を使用（推奨）

1. Chrome 拡張機能のポップアップを開く
2. インデックス化するフォルダを選択
3. 「Index Selected Folder」をクリック
4. AI アシスタントでインデックス化されたコンテンツを使用

### コマンドラインオプション

```bash
# Chrome 拡張機能の事前ビルド済みインデックスを使用
CHROME_PROFILE_NAME="Extension" CHROME_TARGET_FOLDER="Development" ./target/release/mcp-bookmark

# インデックス管理
./target/release/mcp-bookmark --list-indexes
./target/release/mcp-bookmark --clear-index
```

## 利用可能な MCP ツール

- `search_bookmarks` - タイトル/URL で検索
- `search_bookmarks_fulltext` - 全文コンテンツ検索
- `get_bookmark_content` - 特定 URL のコンテンツ取得
- `list_bookmark_folders` - 利用可能なフォルダ一覧
- `get_indexing_status` - インデックス化の進捗確認

## インデックスの保存場所

インデックスは以下に保存されます：
- macOS: `~/Library/Application Support/mcp-bookmark/`

プロファイル/フォルダの組み合わせごとに独自のインデックスを持ちます。

## ライセンス

MIT