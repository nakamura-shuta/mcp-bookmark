# Chrome Bookmark MCP Server

Chromeブックマークへのアクセスを提供するMCP (Model Context Protocol) サーバー

## 機能

- **高速全文検索**: tantivy検索エンジンによるブックマーク内容の検索（検索結果に抜粋付き）
- **コンテンツキャッシュ**: インデックスDBから直接コンテンツ取得（リモート再取得不要）
- **自動インデックス**: バックグラウンドでWebページ内容を自動取得・保存
- **プロファイル対応**: 複数のChromeプロファイルから選択可能
- **フォルダフィルタ**: 特定フォルダのブックマークのみ公開
- **独立インデックス管理**: プロファイル・フォルダごとに独立したインデックス

## インストール

### macOS (Apple Silicon)
```bash
curl -L https://github.com/your-org/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-arm64 -o mcp-bookmark
chmod +x mcp-bookmark
sudo mv mcp-bookmark /usr/local/bin/
```

### macOS (Intel)
```bash
curl -L https://github.com/your-org/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-x64 -o mcp-bookmark
chmod +x mcp-bookmark
sudo mv mcp-bookmark /usr/local/bin/
```

## 設定

### 基本設定

`~/.config/claude/config.json`:

```json
{
  "mcpServers": {
    "chrome-bookmarks": {
      "command": "mcp-bookmark"
    }
  }
}
```

### プロジェクト単位での設定

プロジェクトのルートディレクトリに`.mcp.json`を配置することで、そのプロジェクト専用のMCP設定を有効にできます。

`.mcp.json`:

```json
{
  "mcpServers": {
    "chrome-bookmarks": {
      "command": "mcp-bookmark",
      "args": ["Development", "100"]
    }
  }
}
```

この設定により、プロジェクトごとに異なるブックマークフォルダや設定を使い分けることができます。

### 特定フォルダのみ公開

```json
{
  "mcpServers": {
    "chrome-bookmarks": {
      "command": "mcp-bookmark",
      "args": ["Development", "100"]
    }
  }
}
```

### サブフォルダの指定

スラッシュ（`/`）を使ってサブフォルダを指定できます：

```json
{
  "mcpServers": {
    "chrome-bookmarks": {
      "command": "mcp-bookmark",
      "env": {
        "CHROME_TARGET_FOLDER": "Development/React"
      }
    }
  }
}
```

この機能により、ネストされた特定のサブフォルダのみを公開できます。

### プロファイル指定

```json
{
  "mcpServers": {
    "chrome-bookmarks": {
      "command": "mcp-bookmark",
      "args": ["--profile", "Work"]
    }
  }
}
```

## 使い方

### コマンドライン

```bash
mcp-bookmark                        # 全ブックマーク
mcp-bookmark Development            # Developmentフォルダのみ
mcp-bookmark Development 100        # 最大100件
mcp-bookmark Work,Tech              # 複数フォルダ

mcp-bookmark --profile Work         # Workプロファイル
mcp-bookmark --folder Development   # 特定フォルダ
mcp-bookmark --exclude Archive      # フォルダ除外

# インデックス管理
mcp-bookmark --list-indexes         # インデックス一覧
mcp-bookmark --clear-index          # 現在設定のインデックスをクリア
mcp-bookmark --clear-all-indexes    # 全インデックスをクリア
```

### 利用可能なツール（MCPクライアント向け）

1. **search_bookmarks** - タイトルやURLでブックマークを検索
2. **search_bookmarks_fulltext** - 全文検索（コンテンツ含む、結果に抜粋付き）
3. **get_bookmark_content** - URLから完全なコンテンツを取得（インデックスDBから）
4. **list_bookmark_folders** - ブックマークフォルダ一覧を取得
5. **get_indexing_status** - インデックス構築状況を確認
6. **get_available_profiles** - 利用可能なChromeプロファイル一覧を取得

### AI アシスタントでの使用例

```
「Developmentフォルダのブックマークを検索して」
「React関連のドキュメントを探して」
「最近追加したブックマークを表示」
「このURLのページ内容を詳しく教えて」（get_bookmark_contentで全文取得）
```

## インデックス管理

検索インデックスは、プロファイルとフォルダの組み合わせごとに独立して管理されます：

```
~/Library/Application Support/mcp-bookmark/
├── Default_Development/      # Defaultプロファイル、Developmentフォルダ
├── Work_Tech_React/         # Workプロファイル、Tech/Reactフォルダ
└── Personal_all/            # Personalプロファイル、全ブックマーク
```

### 特徴

- **分離管理**: 異なるプロジェクトで同じプロファイル・フォルダを指定すれば、同じインデックスを共有
- **自動作成**: 初回起動時に自動でインデックスを作成
- **バックグラウンド更新**: サーバー起動後、コンテンツを段階的にインデックス化

### 管理コマンド

```bash
# インデックス一覧（サイズと更新日時を表示）
mcp-bookmark --list-indexes

# 特定インデックスをクリア
mcp-bookmark --clear-index Default_Development

# 全インデックスをクリア
mcp-bookmark --clear-all-indexes
```

## トラブルシューティング

### Chromeプロファイルの確認

```bash
# プロファイル一覧
ls ~/Library/Application\ Support/Google/Chrome/*/Bookmarks

# chrome://version/ でプロファイルパスを確認
```

### ログファイル

```
~/Library/Application Support/mcp-bookmark/logs/
```

ログレベル変更:
```json
{
  "mcpServers": {
    "chrome-bookmarks": {
      "command": "mcp-bookmark",
      "env": {"RUST_LOG": "debug"}
    }
  }
}
```

## 検索インデックス

インデックスは自動的に構築され、以下に保存されます：
```
~/Library/Application Support/mcp-bookmark/index/
```

## ライセンス

MIT