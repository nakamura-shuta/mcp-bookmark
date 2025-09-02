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

### オプション 1: ソースからビルド（Rust 必須）

```bash
git clone https://github.com/nakamura-shuta/mcp-bookmark.git
cd mcp-bookmark
./install.sh
```

インストールスクリプトが以下の処理をガイド：

1. **バイナリのビルド** - MCP サーバーとネイティブメッセージングホストをコンパイル
2. **Chrome 拡張機能のインストール** - 手動でロードして Extension ID を提供
3. **最初のインデックス作成** - Chrome 拡張機能でブックマークフォルダをインデックス化
4. **.mcp.json の生成** - 選択したインデックス名で設定ファイルを作成

### オプション 2: ビルド済みバイナリを使用（Rust 不要）

1. インストール用ディレクトリを作成：
```bash
mkdir ~/mcp-bookmark
cd ~/mcp-bookmark
```

2. [最新リリース](https://github.com/nakamura-shuta/mcp-bookmark/releases/latest)からビルド済みバイナリをダウンロード：

#### macOS (Intel)
```bash
# バイナリをダウンロード
curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-x64 -o mcp-bookmark
curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-x64-native -o mcp-bookmark-native
chmod +x mcp-bookmark mcp-bookmark-native
```

#### macOS (Apple Silicon)
```bash
# バイナリをダウンロード
curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-arm64 -o mcp-bookmark
curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-arm64-native -o mcp-bookmark-native
chmod +x mcp-bookmark mcp-bookmark-native
```

### 詳細手順

#### オプション 1: ソースからビルドする場合

##### ステップ 1: インストールスクリプトを実行

スクリプトがビルドとセットアップをガイドします。

#### オプション 2: ビルド済みバイナリを使用する場合

##### ステップ 1: バイナリのダウンロードとセットアップ

上記のコマンドでバイナリをダウンロードした後、ネイティブメッセージングホストを設定：

```bash
# ネイティブメッセージングホストのマニフェストを作成（~/mcp-bookmark ディレクトリにいることを確認）
mkdir -p ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/
cat > ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json << EOF
{
  "name": "com.mcp_bookmark",
  "description": "MCP Bookmark Native Messaging Host",
  "path": "$HOME/mcp-bookmark/mcp-bookmark-native",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID_HERE/"
  ]
}
EOF
```

#### ステップ 2: Chrome 拡張機能をインストール

**オプション 1（ソースからビルド）の場合：**
1. Chrome で `chrome://extensions/` を開く
2. 「デベロッパーモード」を有効化（右上）
3. 「パッケージ化されていない拡張機能を読み込む」をクリック
4. `mcp-bookmark/bookmark-indexer-extension` フォルダを選択
5. Extension ID をコピーして、プロンプトに貼り付け

**オプション 2（ビルド済み）の場合：**
1. 拡張機能をダウンロード：
   ```bash
   curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/bookmark-indexer-chrome-extension.zip -o extension.zip
   unzip extension.zip -d bookmark-indexer-extension
   ```
2. Chrome で `chrome://extensions/` を開く
3. 「デベロッパーモード」を有効化（右上）
4. 「パッケージ化されていない拡張機能を読み込む」をクリックし、展開した `bookmark-indexer-extension` フォルダを選択
5. Extension ID をコピー
6. ネイティブメッセージングホストのマニフェストを Extension ID で更新：
   ```bash
   # YOUR_EXTENSION_ID_HERE を実際の Extension ID に置き換え
   sed -i '' "s/YOUR_EXTENSION_ID_HERE/実際のExtension ID/" ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json
   ```

#### ステップ 3: 最初のインデックスを作成（プロンプトが表示されたら）

1. Chrome ツールバーの拡張機能アイコンをクリック
2. インデックス名を入力（例: "my-bookmarks"）
3. インデックス化するブックマークフォルダを選択
4. 「Index Selected Folder」をクリック
5. 完了を待ってターミナルに戻る

#### ステップ 4: セットアップを完了

**オプション 1（ソースからビルド）の場合：**
1. 作成したインデックス名を入力
2. `.mcp.json` をプロジェクトにコピー：
   ```bash
   cp .mcp.json ~/your-project/
   ```

**オプション 2（ビルド済み）の場合：**
1. `.mcp.json` 設定ファイルを作成（~/mcp-bookmark ディレクトリにいることを確認）：
   ```bash
   cat > .mcp.json << EOF
   {
     "mcpServers": {
       "mcp-bookmark": {
         "command": "$HOME/mcp-bookmark/mcp-bookmark",
         "args": [],
         "env": {
           "RUST_LOG": "info",
           "INDEX_NAME": "YOUR_INDEX_NAME"
         }
       }
     }
   }
   EOF
   ```
2. `YOUR_INDEX_NAME` をステップ 3 で作成したインデックス名に置き換え
3. プロジェクトにコピー：
   ```bash
   cp .mcp.json ~/your-project/
   ```

#### ステップ 5: Claude Code で使用

1. Claude Code で `/mcp` を実行
2. 「mcp-bookmark」を選択して有効化
3. 試してみる：
   ```
   「React hooks のドキュメントをブックマークから検索して」
   ```

## 使い方

### Chrome 拡張機能を使用（推奨）

1. Chrome 拡張機能のポップアップを開く
2. （オプション）カスタムインデックス名を入力
3. インデックス化するフォルダを選択
4. 「Index Selected Folder」をクリック
5. AI アシスタントでインデックス化されたコンテンツを使用

### 注意事項

#### Notion ページのインデキシング

Notion のページをブックマークからインデキシングする場合、以下の点にご注意ください：

- **ブラウザで Notion を開く**: ブラウザで Notion のページを開いたとき、デスクトップの Notion アプリで開くように設定してある場合、ブラウザで開くように設定を変更してください
- Chrome 拡張機能はブラウザ内のコンテンツのみアクセス可能なため、デスクトップアプリのコンテンツはインデキシングできません
- Notion の URL をブラウザで開いた状態でブックマークし、その後インデキシングしてください

### コマンドラインオプション

```bash
# 特定のインデックスで MCP サーバーを実行
# ソースからビルドした場合：
INDEX_NAME="work_Development" ./target/release/mcp-bookmark

# ビルド済みバイナリの場合（~/mcp-bookmark ディレクトリから）：
INDEX_NAME="work_Development" ./mcp-bookmark

# 複数インデックス検索（カンマ区切り）
INDEX_NAME="work,personal,research" ./mcp-bookmark

# インデックス管理コマンド
./mcp-bookmark --list-indexes      # 利用可能なインデックス一覧
./mcp-bookmark --clear-index       # 現在のインデックスをクリア
./mcp-bookmark --clear-all-indexes # すべてのインデックスをクリア
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
