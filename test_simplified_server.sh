#!/bin/bash

echo "Testing simplified MCP server..."
echo

echo "1. Testing without INDEX_NAME (should fail):"
timeout 2 ./target/release/mcp-bookmark 2>&1 | head -10
echo

echo "2. Testing with INDEX_NAME:"
INDEX_NAME=Extension_Bookmarks timeout 2 ./target/release/mcp-bookmark 2>&1 | head -10
echo

echo "3. Testing --help:"
./target/release/mcp-bookmark --help
echo

echo "Test complete!"