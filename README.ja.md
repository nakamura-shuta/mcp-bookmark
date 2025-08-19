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

### 詳細手順

#### ステップ 1: インストールスクリプトを実行

スクリプトがビルドとセットアップをガイドします。

#### ステップ 2: Chrome 拡張機能をインストール（プロンプトが表示されたら）

1. Chrome で `chrome://extensions/` を開く
2. 「デベロッパーモード」を有効化（右上）
3. 「パッケージ化されていない拡張機能を読み込む」をクリック
4. `mcp-bookmark/bookmark-indexer-extension` フォルダを選択
5. Extension ID をコピーして、プロンプトに貼り付け

#### ステップ 3: 最初のインデックスを作成（プロンプトが表示されたら）

1. Chrome ツールバーの拡張機能アイコンをクリック
2. インデックス名を入力（例: "my-bookmarks"）
3. インデックス化するブックマークフォルダを選択
4. 「Index Selected Folder」をクリック
5. 完了を待ってターミナルに戻る

#### ステップ 4: セットアップを完了

1. 作成したインデックス名を入力
2. `.mcp.json` をプロジェクトにコピー：
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
