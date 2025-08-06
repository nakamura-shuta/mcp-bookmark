# Chrome Bookmark MCP Server

Chromeブックマークを Model Context Protocol (MCP) 経由で AI アシスタントに提供する Rust サーバー

## 機能

### リソース
- `bookmark://tree` - Chrome ブックマーク全体のツリー構造
- `bookmark://folder/{path}` - 特定フォルダのブックマーク

### ツール
- `search_bookmarks` - タイトルまたは URL でブックマークを検索
- `get_bookmark_content` - ブックマーク URL からページメタデータを取得
- `list_bookmark_folders` - 全フォルダ一覧を取得

## インストール

### ビルド済みバイナリ（推奨）

#### macOS (Apple Silicon)
```bash
curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-arm64 -o mcp-bookmark
chmod +x mcp-bookmark
sudo mv mcp-bookmark /usr/local/bin/
```

#### macOS (Intel)
```bash
curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-x64 -o mcp-bookmark
chmod +x mcp-bookmark
sudo mv mcp-bookmark /usr/local/bin/
```

### ソースからビルド
```bash
git clone https://github.com/nakamura-shuta/mcp-bookmark.git
cd mcp-bookmark
cargo build --release
sudo cp target/release/mcp-bookmark /usr/local/bin/
```

## Claude Codeの設定

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

## 使い方

```bash
mcp-bookmark                   # 全ブックマーク
mcp-bookmark Development        # Developmentフォルダのみ
mcp-bookmark Development 10     # 最大10件
mcp-bookmark Work,Tech 20       # 複数フォルダを指定
```

### オプション

特定のChromeプロファイルを使用:
```json
{
  "mcpServers": {
    "chrome-bookmarks": {
      "command": "mcp-bookmark",
      "env": {"CHROME_PROFILE": "Profile 1"}
    }
  }
}
```

## トラブルシューティング

Chromeプロファイルの確認:
```bash
ls -lh ~/Library/Application\ Support/Google/Chrome/*/Bookmarks
```

chrome://version/ でプロファイルパスを確認できます。

## 開発

### テスト
```bash
cargo test --release
```

### プロジェクト構造
```
src/
├── main.rs           # エントリーポイント
├── mcp_server.rs     # MCP サーバー実装
├── bookmark.rs       # ブックマーク読み取り
├── content.rs        # Web コンテンツ取得
├── config.rs         # 設定管理
└── cache.rs          # キャッシュ機能
```

## ライセンス

MIT