#!/bin/bash

echo "Testing simplified Chrome extension..."

# Test list indexes command
echo "Testing list_indexes command..."
echo '{"jsonrpc":"2.0","id":"test1","method":"list_indexes","params":{}}' | \
    ./target/release/mcp-bookmark-native 2>/dev/null | \
    head -c 1000000 | \
    tail -c +5 | \
    jq '.'

echo "Test complete. The extension should now:"
echo "✓ Display existing indexes"
echo "✓ Allow custom index naming"
echo "✓ Index folders with 5-second wait time"
echo "✗ No longer have 'Index Current Tab' button"
echo "✗ No longer have 'Clear Current Index' button"