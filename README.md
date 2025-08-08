# Chrome Bookmark MCP Server

Chromeブックマークへのアクセスを提供するMCP (Model Context Protocol) サーバー

## 機能

- **高速全文検索**: tantivy検索エンジンによるブックマーク内容の検索
- **自動インデックス**: バックグラウンドでWebページ内容を自動取得
- **プロファイル対応**: 複数のChromeプロファイルから選択可能
- **フォルダフィルタ**: 特定フォルダのブックマークのみ公開

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
```

### AI アシスタントでの使用例

```
「Developmentフォルダのブックマークを検索して」
「React関連のドキュメントを探して」
「最近追加したブックマークを表示」
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