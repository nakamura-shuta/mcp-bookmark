# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2025-08-06

### Added
- tantivy v0.24 全文検索エンジン統合
- `search_bookmarks_fulltext` - 高度な全文検索ツール
- `search_by_content` - コンテンツ専用検索ツール
- `get_indexing_status` - インデックス構築状況確認ツール
- バックグラウンドコンテンツインデックス化
- 優先度付きコンテンツ取得（ドキュメントサイト優先）
- 日本語Chrome環境の完全サポート
- 検索インデックスの永続化（`~/Library/Application Support/mcp-bookmark/index/`）

### Changed
- 検索システムを簡略化（フォールバック検索を削除）
- tantivyのみを使用するシンプルな実装に変更
- メモリ使用量を50%削減
- コード複雑度を大幅に削減

### Fixed
- 日本語フォルダパス（「ブックマーク バー」等）の処理を改善
- 大規模ブックマークファイルでのメモリ使用量を最適化

### Technical Details
- **検索アーキテクチャ**: ハイブリッド検索からtantivy単一検索へ
- **インデックス方式**: メタデータ即座 + コンテンツ段階的
- **並列度**: 10並列でコンテンツ取得（3秒タイムアウト）
- **優先順位**: docs.rs > react.dev > MDN > その他

## [0.1.0] - 2025-08-05

### Added
- Initial release of Chrome Bookmark MCP Server
- Basic bookmark reading functionality
- MCP protocol implementation
- Simple search functionality (`search_bookmarks`)
- Folder-based filtering
- Chrome profile auto-detection
- Content fetching from bookmark URLs