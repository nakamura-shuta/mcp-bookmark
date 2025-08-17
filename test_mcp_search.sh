#!/bin/bash

echo "Testing MCP bookmark search with correct environment variables"
echo "============================================"

# Set correct environment variables
export CHROME_PROFILE_NAME="Extension"
export CHROME_TARGET_FOLDER="index-test"
export RUST_LOG=info

echo "Environment:"
echo "  CHROME_PROFILE_NAME=$CHROME_PROFILE_NAME"
echo "  CHROME_TARGET_FOLDER=$CHROME_TARGET_FOLDER"
echo ""

# Start the MCP server and send a search request
echo "Starting MCP server and sending search request..."
echo ""

# Create a test MCP request for search with proper handshake
cat << 'EOF' | ./target/release/mcp-bookmark 2>&1
{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"0.1.0","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}},"id":1}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"search_bookmarks_fulltext","arguments":{"query":"Auth0 export"}},"id":2}
EOF