#!/bin/bash

echo "Testing MCP search with token limit fixes"
echo "=========================================="
echo ""

export CHROME_PROFILE_NAME="Extension"
export CHROME_TARGET_FOLDER="index-test"
export RUST_LOG=info

echo "Environment:"
echo "  CHROME_PROFILE_NAME=$CHROME_PROFILE_NAME"
echo "  CHROME_TARGET_FOLDER=$CHROME_TARGET_FOLDER"
echo ""

# Test 1: Search with limit=1 (should work)
echo "Test 1: Search with limit=1"
echo "----------------------------"
(
  echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"0.1.0","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}},"id":1}'
  echo '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
  echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"search_bookmarks_fulltext","arguments":{"query":"Auth0","limit":1}},"id":2}'
  echo ''
) | timeout 3 ./target/release/mcp-bookmark 2>&1 | grep -E "(results|total_results|Error)" | head -5

echo ""

# Test 2: Search with limit=5 (should work with reduced snippets)
echo "Test 2: Search with limit=5"
echo "----------------------------"
(
  echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"0.1.0","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}},"id":1}'
  echo '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
  echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"search_bookmarks_fulltext","arguments":{"query":"RDS","limit":5}},"id":2}'
  echo ''
) | timeout 3 ./target/release/mcp-bookmark 2>&1 | grep -E "(results|total_results|Error)" | head -5

echo ""
echo "Test completed"