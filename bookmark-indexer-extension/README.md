# Bookmark Indexer - Chrome Extension

Chromeブックマークのコンテンツをローカルの Tantivy 検索エンジンにインデックス化する拡張機能

## 概要

- ブックマークしたページのコンテンツを取得
- Native Messaging 経由で `mcp-bookmark-native` に送信  
- Tantivy インデックスに保存（MCPサーバーと共有）

## インストール

### 1. Native Messaging Host のビルド

```bash
# プロジェクトルートで実行
cargo build --release --bin mcp-bookmark-native
```

### 2. Native Messaging Host の設定

```bash
# 設定ファイルの配置
cat > ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json << EOF
{
  "name": "com.mcp_bookmark",
  "description": "Bookmark Indexer Native Host",
  "path": "/path/to/mcp-bookmark/target/release/mcp-bookmark-native",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID/"
  ]
}
EOF
```

### 3. Chrome拡張機能のインストール

1. Chrome で `chrome://extensions/` を開く
2. 「デベロッパーモード」を ON
3. 「パッケージ化されていない拡張機能を読み込む」をクリック
4. `/path/to/mcp-bookmark/bookmark-indexer-extension` ディレクトリを選択
5. 表示された拡張機能IDを確認

### 4. Native Messaging Host 設定を更新

```bash
# 拡張機能IDを設定ファイルに反映
EXTENSION_ID="表示されたID"
sed -i "" "s/YOUR_EXTENSION_ID/$EXTENSION_ID/g" \
  ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json
```

### 5. Chrome を再起動

Native Messaging Host の設定後は Chrome の完全再起動が必要

## 使い方

1. 拡張機能アイコンをクリック
2. インデックス化したいフォルダを選択
3. 「Index Selected Folder」をクリック
4. 進捗バーでインデックス化状況を確認

### その他の機能

- **Index Current Tab**: 現在のタブをインデックス化
- **Test Connection**: Native Messaging の接続テスト
- **Clear Index**: インデックスをクリア

## アーキテクチャ

```
Chrome Extension
    ↓ fetch() でコンテンツ取得
    ↓ Native Messaging
mcp-bookmark-native (Rust)
    ↓ Tantivy API
Tantivy Index (~/.mcp-bookmark/tantivy_index/)
    ↑ 検索
mcp-bookmark (MCP Server)
```

## ファイル構成

```
bookmark-indexer-extension/
├── manifest.json     # 拡張機能マニフェスト
├── background.js     # Service Worker
├── popup.html        # ポップアップUI
├── popup.js          # UIコントローラー
├── icon.png          # アイコン
└── README.md         # このファイル
```

## トラブルシューティング

### "Specified native messaging host not found"
- Chrome を完全に再起動
- Native Messaging Host 設定を確認

### 接続エラー
- バイナリのパスが正しいか確認
- 拡張機能IDが設定ファイルと一致するか確認

### ログの確認

```bash
# Native Messaging ログ
tail -f /tmp/mcp-bookmark-native.log

# インデックスの場所
ls -la ~/.mcp-bookmark/tantivy_index/
```

## 制限事項

- 静的コンテンツのみ取得（JavaScript実行なし）
- ログインが必要なページは取得不可
- 1ページあたり最大10,000文字まで

## ライセンス

MIT