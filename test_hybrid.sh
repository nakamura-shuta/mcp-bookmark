#!/bin/bash

echo "========================================="
echo "ハイブリッド検索システムのテスト"
echo "========================================="
echo ""

# テスト用の環境変数設定
export RUST_LOG=info

echo "📚 MCPサーバーを起動してインデックス状況を確認..."
echo ""

# テスト1: インデックス状況の確認
echo "1. インデックス状況の確認"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_indexing_status","arguments":{}}}' | timeout 3 cargo run --release 2>/dev/null | grep -A20 '"result"' | head -20

echo ""
echo "2. 検索テスト（バックグラウンドインデックス構築中）"
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_bookmarks_fulltext","arguments":{"query":"test"}}}' | timeout 3 cargo run --release 2>/dev/null | grep -A20 '"indexing_status"' | head -20

echo ""
echo "3. 実際にサーバーを起動してログを確認（10秒間）"
echo ""
timeout 10 cargo run --release 2>&1 | grep -E "(ハイブリッド|インデックス|バックグラウンド|📚|📥|✅|🚀)" 

echo ""
echo "========================================="
echo "テスト完了"
echo "========================================="