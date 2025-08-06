#!/bin/bash

# Test script for MCP bookmark server with tantivy search

# Start the server in background
echo "Starting MCP bookmark server..."
cargo run --release &
SERVER_PID=$!

# Give server time to start and index
sleep 3

echo "Server started with PID: $SERVER_PID"

# Test search_bookmarks_fulltext tool
echo "Testing full-text search..."
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_bookmarks_fulltext","arguments":{"query":"test"}}}' | nc localhost 3000

# Kill the server
kill $SERVER_PID
echo "Server stopped"