#!/bin/bash

echo "Testing MCP bookmark search with correct environment variables"
echo "============================================"

# Set correct environment variables
export CHROME_PROFILE_NAME="Extension"
export CHROME_TARGET_FOLDER="index-test"
export RUST_LOG=debug

echo "Environment:"
echo "  CHROME_PROFILE_NAME=$CHROME_PROFILE_NAME"
echo "  CHROME_TARGET_FOLDER=$CHROME_TARGET_FOLDER"
echo ""

# Create a named pipe for communication
PIPE=$(mktemp -u)
mkfifo "$PIPE"

# Start the MCP server in background
./target/release/mcp-bookmark < "$PIPE" 2>&1 &
SERVER_PID=$!

# Give server time to start
sleep 0.5

# Send requests to the pipe
echo "Sending MCP requests..."
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"0.1.0","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}},"id":1}' > "$PIPE"
sleep 0.2
echo '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' > "$PIPE"
sleep 0.2
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"search_bookmarks_fulltext","arguments":{"query":"Auth0 export"}},"id":2}' > "$PIPE"
sleep 1

# Kill the server
kill $SERVER_PID 2>/dev/null

# Clean up
rm -f "$PIPE"

echo ""
echo "Test completed"