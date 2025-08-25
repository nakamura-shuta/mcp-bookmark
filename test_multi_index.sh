#!/bin/bash

echo "Testing multi-index search feature"
echo "=================================="
echo

# Test with multiple indices
echo "Testing with multiple indices: hogehoge_new_index_test,index0821_new_index_test"
INDEX_NAME="hogehoge_new_index_test,index0821_new_index_test" \
  timeout 3 ./target/release/mcp-bookmark 2>&1 | head -20

echo
echo "=================================="
echo "Test completed"