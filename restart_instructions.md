# MCPサーバー再接続手順

## 問題
MCPサーバーが "Failed to reconnect to mcp-bookmark" と表示される

## 原因
MCPサーバーのコードを大幅に変更したため、Claudeアプリケーションが新しいサーバー仕様を認識できていない

## 解決方法

### 1. Claudeアプリケーションを完全に終了
- Cmd + Q でClaudeアプリケーションを終了
- または、メニューバーから Claude > Quit Claude を選択

### 2. MCPサーバーが正しくビルドされていることを確認
```bash
cd /Users/nakamura.shuta/dev/rust/mcp-bookmark
cargo build --release
```

### 3. .mcp.jsonファイルの確認
現在の設定:
```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/Users/nakamura.shuta/dev/rust/mcp-bookmark/target/release/mcp-bookmark",
      "args": [],
      "env": {
        "RUST_LOG": "debug",
        "INDEX_NAME": "fuga_index_test"
      }
    }
  }
}
```

### 4. 利用可能なインデックスの確認
```bash
./target/release/mcp-bookmark --list-indexes
```

現在利用可能なインデックス:
- Default_all
- Extension_Bookmarks
- Extension_index-test
- fuga_index_test
- hoge_index_test

### 5. Claudeアプリケーションを再起動
- Claudeアプリケーションを起動
- 新しいチャットを開始
- /mcp コマンドでMCPサーバーの状態を確認

### 6. 動作確認
MCPサーバーが正常に接続されたら、以下のツールが利用可能になります:
- `search_bookmarks_fulltext` - ブックマークの全文検索
- `get_bookmark_content` - 特定URLのコンテンツ取得
- `get_indexing_status` - インデックスの状態確認

## トラブルシューティング

### エラーが続く場合
1. ログファイルを確認:
```bash
tail -f ~/Library/Application\ Support/mcp-bookmark/logs/mcp-bookmark.log
```

2. 別のインデックスを試す:
```json
"env": {
  "RUST_LOG": "debug",
  "INDEX_NAME": "Extension_Bookmarks"
}
```

3. MCPサーバーを直接テスト:
```bash
INDEX_NAME=fuga_index_test ./target/release/mcp-bookmark
```