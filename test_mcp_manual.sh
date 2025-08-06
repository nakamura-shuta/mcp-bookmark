#!/bin/bash

echo "Manual test for MCP bookmark server with tantivy search"
echo "========================================================="
echo ""
echo "Building the server..."
cargo build --release

echo ""
echo "To test the MCP server, run it and send commands via stdin:"
echo ""
echo "1. Start the server:"
echo "   cargo run --release"
echo ""
echo "2. In another terminal, you can test it with:"
echo ""
echo "   # List available tools:"
echo '   echo '"'"'{"jsonrpc":"2.0","id":1,"method":"tools/list"}'"'"' | cargo run --release'
echo ""
echo "   # Search with full-text search:"
echo '   echo '"'"'{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_bookmarks_fulltext","arguments":{"query":"rust"}}}'"'"' | cargo run --release'
echo ""
echo "   # Search with domain filter:"
echo '   echo '"'"'{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search_bookmarks_fulltext","arguments":{"query":"","domain":"github.com"}}}'"'"' | cargo run --release'
echo ""

# Quick test to verify it responds to tools/list
echo "Quick test - listing available tools:"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | timeout 2 cargo run --release 2>/dev/null | grep -o '"name":"[^"]*"' | head -5

echo ""
echo "Available tools should include:"
echo "  - search_bookmarks"
echo "  - search_bookmarks_fulltext (NEW!)"
echo "  - list_bookmark_folders"
echo "  - get_bookmark_content"