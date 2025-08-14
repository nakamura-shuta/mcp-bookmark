日本語 | [English](README.md)

# Chrome Bookmark MCP Server

**⚠️ macOS と Chrome 限定**: このツールは現在 macOS と Google Chrome のみをサポートしています。

Chrome ブックマークを AI アシスタントから全文検索できる MCP (Model Context Protocol) サーバー

## 機能

- **全文検索**: Tantivy エンジンによるブックマークコンテンツ検索
- **Chrome プロファイル対応**: 複数の Chrome プロファイルに対応
- **フォルダフィルタリング**: 特定のブックマークフォルダのみ公開
- **自動インデックス**: Web ページコンテンツの自動インデックス化
- **Chrome 拡張機能**: コンテンツインデックス化を強化する拡張機能（オプション）

## 動作要件

- macOS
- Google Chrome
- Rust 1.70 以上（ソースからビルドする場合）

## インストール

### ソースからビルド

```bash
# リポジトリをクローン
git clone https://github.com/USERNAME/mcp-bookmark.git
cd mcp-bookmark

# リリースバイナリをビルド
cargo build --release

# ビルドを確認
./target/release/mcp-bookmark --help
```

## 設定

### 基本設定

プロジェクトルートに`.mcp.json`ファイルを作成：

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/path/to/mcp-bookmark/target/release/mcp-bookmark"
    }
  }
}
```

### 詳細設定

#### 特定フォルダのみ

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "env": {
        "CHROME_TARGET_FOLDER": "Development"
      }
    }
  }
}
```

#### Chrome プロファイル指定

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "env": {
        "CHROME_PROFILE_NAME": "仕事"
      }
    }
  }
}
```

注: Chrome に表示される表示名（例："仕事"、"個人用"）を使用してください。

## 使い方

### コマンドライン

```bash
# 基本的な使い方
mcp-bookmark                        # 全ブックマーク
mcp-bookmark Development            # 特定フォルダ
mcp-bookmark Development 10         # Developmentから最大10件

# プロファイルとフォルダ指定
mcp-bookmark --profile 仕事 --folder Development

# インデックス管理
mcp-bookmark --list-indexes         # インデックス一覧
mcp-bookmark --clear-index          # 現在(環境変数やデフォルト）の設定に対応するインデックスをクリア
mcp-bookmark --clear-index 仕事_mcp-rust  # 特定のインデックスをクリア
mcp-bookmark --clear-all-indexes    # 全インデックスをクリア
```

### MCP ツール一覧

1. **search_bookmarks** - タイトルや URL で検索
2. **search_bookmarks_fulltext** - 全文検索
3. **get_bookmark_content** - ページ全文を取得
4. **list_bookmark_folders** - フォルダ一覧を取得
5. **get_available_profiles** - Chrome プロファイル一覧

### AI アシスタントでの使用例

```
「Developmentフォルダのブックマークを検索」
「React関連のドキュメントを探して」
「最近追加したブックマークを表示」
```

## Chrome 拡張機能（オプション）

Chrome 拡張機能を使用すると、Web ページコンテンツを直接 Chrome から取得してインデックス化できます。

### インストール手順

1. **Native Host をビルド**:

```bash
cargo build --release --bin mcp-bookmark-native
```

2. **Native Messaging を設定**（プロジェクトルートで実行）:

```bash
# プロジェクトルートから実行してください
cat > ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json << EOF
{
  "name": "com.mcp_bookmark",
  "description": "Bookmark Indexer Native Host",
  "path": "$(pwd)/target/release/mcp-bookmark-native",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID/"
  ]
}
EOF
```

3. **拡張機能をインストール**:

   - `chrome://extensions/`を開く
   - 「デベロッパーモード」を有効化
   - 「パッケージ化されていない拡張機能を読み込む」をクリック
   - `bookmark-indexer-extension`フォルダを選択
   - 表示された拡張機能 ID をメモ

4. **設定を更新**:

```bash
# 実際の拡張機能IDに置き換え
EXTENSION_ID="拡張機能ID"
sed -i "" "s/YOUR_EXTENSION_ID/$EXTENSION_ID/g" \
  ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json
```

5. **Chrome を完全に再起動**

### 拡張機能の使い方

- ツールバーの拡張機能アイコンをクリック
- ブックマークフォルダを選択
- 「Index Selected Folder」をクリック

## インデックスの場所

インデックスは以下に保存されます：

```
~/Library/Application Support/mcp-bookmark/index/
```

## ライセンス

MIT
