#!/bin/bash

# Set environment variables
export CHROME_PROFILE_NAME="Extension"
export CHROME_TARGET_FOLDER="index-test"
export RUST_LOG=debug

echo "Testing search with Extension_index-test"
echo "========================================"
echo ""

# Use printf to send JSON-RPC messages with proper line endings
(
  echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"0.1.0","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}},"id":1}'
  echo '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
  echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"search_bookmarks_fulltext","arguments":{"query":"Auth0 export","limit":5}},"id":2}'
) | timeout 3 ./target/release/mcp-bookmark 2>&1 | grep -v "^2025" | grep -v "^DEBUG"

echo ""
echo "Test completed"