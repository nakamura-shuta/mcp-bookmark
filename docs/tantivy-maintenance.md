# tantivy インデックス管理ガイド

## 概要

Chrome Bookmark MCP サーバーは tantivy を使用した全文検索インデックスを管理しています。このドキュメントではインデックスの初期化、削除、再構築の手順を説明します。

## インデックスの場所

tantivy インデックスは以下の場所に保存されます：

### macOS
```
~/Library/Application Support/mcp-bookmark/index/
```

### Linux
```
~/.local/share/mcp-bookmark/index/
```

### Windows
```
%APPDATA%\mcp-bookmark\index\
```

## インデックスの初期化・削除手順

### 1. インデックスを完全に削除する

インデックスに問題が発生した場合や、完全に再構築したい場合：

```bash
# macOS の場合
rm -rf ~/Library/Application\ Support/mcp-bookmark/index/

# Linux の場合
rm -rf ~/.local/share/mcp-bookmark/index/

# Windows の場合（PowerShell）
Remove-Item -Recurse -Force "$env:APPDATA\mcp-bookmark\index"
```

### 2. インデックスの自動再構築

インデックスを削除した後、サーバーを起動すると自動的に新しいインデックスが作成されます：

```bash
cargo run --release
```

起動時のログ：
```
[INFO] Creating new index at "~/Library/Application Support/mcp-bookmark/index"
[INFO] 📚 ハイブリッド検索マネージャーを初期化中 (638件のブックマーク)
[INFO] 📝 メタデータをインデックス化中...
[INFO] 🚀 バックグラウンドインデックス構築を開始
```

### 3. インデックスの状態確認

現在のインデックスサイズを確認：

```bash
# macOS
du -sh ~/Library/Application\ Support/mcp-bookmark/index/

# 出力例：
# 12M    /Users/username/Library/Application Support/mcp-bookmark/index/
```

インデックスファイルの詳細：

```bash
ls -la ~/Library/Application\ Support/mcp-bookmark/index/

# 出力例：
# -rw-r--r--  meta.json          # インデックスメタデータ
# -rw-r--r--  .managed.json      # tantivy管理ファイル
# drwxr-xr-x  seg_1/            # セグメントディレクトリ
# drwxr-xr-x  seg_2/            # セグメントディレクトリ
```

## トラブルシューティング

### 問題: インデックスが破損した

症状：
- 検索が常に空の結果を返す
- "Failed to open existing index" エラー

解決策：
```bash
# 1. インデックスを削除
rm -rf ~/Library/Application\ Support/mcp-bookmark/index/

# 2. サーバーを再起動
cargo run --release
```

### 問題: インデックスサイズが大きすぎる

症状：
- インデックスが数百MB以上になる
- ディスク容量不足

解決策：
```bash
# 1. 現在のサイズを確認
du -sh ~/Library/Application\ Support/mcp-bookmark/index/

# 2. インデックスを再構築
rm -rf ~/Library/Application\ Support/mcp-bookmark/index/
cargo run --release

# 3. 不要なコンテンツインデックスを制限（将来実装予定）
# config.tomlで設定可能にする予定
```

### 問題: 検索精度が低い

症状：
- 期待する検索結果が出ない
- コンテンツが検索されない

解決策：
```bash
# インデックス構築状況を確認
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_indexing_status","arguments":{}}}' | cargo run --release

# 出力例：
# "status": "✅ インデックス構築完了: 626/638 成功, 12 エラー (所要時間: 95.3秒)"
```

コンテンツインデックスが完了していない場合は、サーバーを起動したまま待機してください。

### 問題: 特定のブックマークが検索されない

症状：
- 特定のURLのブックマークが検索結果に出ない

原因と解決策：
1. **コンテンツ取得エラー**: タイムアウトや認証エラー
   - ログを確認: `RUST_LOG=debug cargo run --release`
   
2. **フォルダフィルター**: 除外設定を確認
   ```bash
   cargo run --release -- --exclude "Private" --exclude "Archive"
   ```

3. **最大ブックマーク数制限**: 制限を増やす
   ```bash
   cargo run --release -- --max 10000
   ```

## インデックスのバックアップとリストア

### バックアップ

```bash
# インデックスディレクトリをバックアップ
tar -czf mcp-bookmark-index-backup.tar.gz \
    ~/Library/Application\ Support/mcp-bookmark/index/
```

### リストア

```bash
# 既存のインデックスを削除
rm -rf ~/Library/Application\ Support/mcp-bookmark/index/

# バックアップからリストア
tar -xzf mcp-bookmark-index-backup.tar.gz -C ~/
```

## パフォーマンス最適化

### インデックス構築の高速化

環境変数で並列度を調整（将来実装予定）：
```bash
export MCP_BOOKMARK_PARALLEL_FETCHES=20  # デフォルト: 10
export MCP_BOOKMARK_FETCH_TIMEOUT=5      # デフォルト: 3秒
cargo run --release
```

### メモリ使用量の削減

小さいヒープサイズで実行：
```bash
export MCP_BOOKMARK_INDEX_HEAP_SIZE=10000000  # 10MB（最小15MB推奨）
cargo run --release
```

## 開発者向け情報

### インデックススキーマ

現在のインデックスは以下のフィールドを持ちます：

| フィールド | 型 | 用途 |
|-----------|-----|------|
| id | STRING | ブックマークID |
| url | TEXT | URL（全文検索対象） |
| title | TEXT | タイトル（全文検索対象） |
| content | TEXT | ページコンテンツ（全文検索対象） |
| folder_path | STRING | フォルダパス |
| domain | STRING+FAST | ドメイン（高速フィルタリング） |
| date_added | i64+FAST | 追加日時 |
| date_modified | i64+FAST | 更新日時 |

### プログラムからインデックスを操作

```rust
use mcp_bookmark::search::SearchManager;

// インデックスを特定の場所に作成
let index_path = PathBuf::from("/custom/path/to/index");
let mut manager = SearchManager::new(Some(index_path))?;

// インデックスを再構築
manager.rebuild_index(&bookmarks)?;

// インデックスサイズを取得
let size = manager.get_index_size()?;
println!("Index size: {} bytes", size);
```

## 関連ドキュメント

- [ハイブリッド検索実装ガイド](../internal_docs/HYBRID_SEARCH_IMPLEMENTATION.md)
- [機能仕様書](./features.md)
- [TODO](../internal_docs/TODO.md)