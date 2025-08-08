# Chrome Bookmark MCP Server

Chromeブックマークを Model Context Protocol (MCP) 経由で AI アシスタントに提供する Rust サーバー

## 機能

### リソース
- `bookmark://tree` - Chrome ブックマーク全体のツリー構造
- `bookmark://folder/{path}` - 特定フォルダのブックマーク

### ツール
- `search_bookmarks` - タイトルまたは URL でブックマークを検索（シンプル検索）
- `search_bookmarks_fulltext` - tantivy による全文検索（タイトル、URL、コンテンツ）
- `search_by_content` - ページコンテンツのみで検索
- `get_bookmark_content` - ブックマーク URL からページメタデータを取得
- `list_bookmark_folders` - 全フォルダ一覧を取得
- `get_indexing_status` - インデックス構築状況を確認

### 検索システム
- **tantivy全文検索エンジン**: 高速な全文検索
- **バックグラウンドインデックス**: 起動後に自動でコンテンツを取得・インデックス化
- **優先度付きインデックス**: ドキュメントサイト（docs.rs等）を優先的に処理

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

# プロファイルとフォルダ指定
mcp-bookmark --profile "Work" --folder "Development"  # 特定プロファイルの特定フォルダ
mcp-bookmark --exclude Personal,Archive            # 指定フォルダを除外
```

### オプション

#### プロファイル名で指定（推奨）
```json
{
  "mcpServers": {
    "chrome-bookmarks": {
      "command": "mcp-bookmark",
      "args": ["--profile", "Work", "--folder", "Development"]
    }
  }
}
```

#### 環境変数で指定
```json
{
  "mcpServers": {
    "chrome-bookmarks": {
      "command": "mcp-bookmark",
      "env": {
        "CHROME_PROFILE_NAME": "Work",
        "CHROME_TARGET_FOLDER": "Development"
      }
    }
  }
}
```

#### レガシー: プロファイルディレクトリで指定
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