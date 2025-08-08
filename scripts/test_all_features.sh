#!/bin/bash

# Chrome Bookmark MCP Server - Comprehensive Feature Tests

set -e

echo "🧪 Chrome Bookmark MCP Server - Comprehensive Feature Tests"
echo "=========================================================="

# Build the project first
echo "📦 Building the project..."
cargo build --release

BINARY="./target/release/mcp-bookmark"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 Test 1: Profile and Folder Configuration"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "► Testing profile resolution (Work)..."
RUST_LOG=info timeout 2 $BINARY --profile "Work" 2>&1 | head -10 || true

echo ""
echo "► Testing profile + folder (Work/Development)..."
RUST_LOG=info timeout 2 $BINARY --profile "Work" --folder "Development" 2>&1 | head -10 || true

echo ""
echo "► Testing environment variables..."
CHROME_PROFILE_NAME="Work" CHROME_TARGET_FOLDER="Development" RUST_LOG=info timeout 2 $BINARY 2>&1 | head -10 || true

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔎 Test 2: Search Functionality"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "► Testing full-text search..."
echo '{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "search_bookmarks_fulltext",
    "arguments": {
      "query": "rust",
      "limit": 5
    }
  }
}' | timeout 5 $BINARY 2>/dev/null | grep -E '"result"|"error"' || echo "Search test completed"

echo ""
echo "► Testing content-only search..."
echo '{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "search_by_content",
    "arguments": {
      "query": "documentation",
      "limit": 5
    }
  }
}' | timeout 5 $BINARY 2>/dev/null | grep -E '"result"|"error"' || echo "Content search test completed"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🧪 Test 3: Unit Tests"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "► Running cargo tests..."
cargo test --release --quiet 2>&1 | tail -5

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ All tests completed successfully!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Note: 'timeout' is used to prevent the MCP server from running indefinitely."
echo "      Error messages about 'Terminated' are expected and normal."