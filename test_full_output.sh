#!/bin/bash

export CHROME_PROFILE_NAME="Extension"
export CHROME_TARGET_FOLDER="index-test"
export RUST_LOG=info

echo "Full output test"
echo "================"
echo ""

# Send all commands and capture all output
(
  echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"0.1.0","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}},"id":1}'
  sleep 0.1
  echo '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
  sleep 0.1
  echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"search_bookmarks_fulltext","arguments":{"query":"Auth0"}},"id":2}'
  sleep 0.5
  # Send close to terminate cleanly
  echo ''
) | ./target/release/mcp-bookmark 2>&1