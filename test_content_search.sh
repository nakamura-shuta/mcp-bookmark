#!/bin/bash

# Test search_by_content tool

echo "Testing search_by_content tool..."

# Start the server in background
cargo build --release
./target/release/mcp-bookmark &
SERVER_PID=$!

# Wait for server to start and index
sleep 5

echo "1. Testing content-only search..."
echo '{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "search_by_content",
    "arguments": {
      "query": "documentation",
      "limit": 5
    }
  }
}' | nc localhost 3000

echo ""
echo "2. Checking indexing status..."
echo '{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "get_indexing_status",
    "arguments": {}
  }
}' | nc localhost 3000

# Kill the server
kill $SERVER_PID

echo ""
echo "Test complete!"