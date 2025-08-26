#!/bin/bash

# Test script for incremental index updates

set -e

echo "Testing incremental index update functionality..."

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test index name
TEST_INDEX="test_incremental_update"

# Function to send JSON message to native host
send_message() {
    local json="$1"
    local length=${#json}
    # Native messaging format: 4 bytes length + JSON
    printf '\x%02x\x%02x\x%02x\x%02x' \
        $((length & 0xFF)) \
        $(((length >> 8) & 0xFF)) \
        $(((length >> 16) & 0xFF)) \
        $(((length >> 24) & 0xFF))
    printf '%s' "$json"
}

# Test 1: Check for updates on empty index
echo -e "${YELLOW}Test 1: Check for updates on empty index${NC}"
RESPONSE=$(echo '{"jsonrpc":"2.0","id":"test1","method":"check_for_updates","params":{"bookmarks":[{"id":"1","date_modified":"2024-01-01"},{"id":"2","date_modified":"2024-01-02"}],"index_name":"'$TEST_INDEX'"}}' | ./target/release/mcp-bookmark-native 2>/dev/null | tail -n 1)
echo "Response: $RESPONSE"

if echo "$RESPONSE" | grep -q '"new_bookmarks":\["1","2"\]'; then
    echo -e "${GREEN}✓ Empty index correctly identified all bookmarks as new${NC}"
else
    echo -e "${RED}✗ Failed to identify new bookmarks${NC}"
    exit 1
fi

# Test 2: Index a bookmark
echo -e "\n${YELLOW}Test 2: Index a bookmark${NC}"
RESPONSE=$(echo '{"jsonrpc":"2.0","id":"test2","method":"index_bookmark","params":{"id":"1","url":"https://example.com","title":"Example","content":"Test content","folder_path":[],"date_added":"2024-01-01","date_modified":"2024-01-01","index_name":"'$TEST_INDEX'"}}' | ./target/release/mcp-bookmark-native 2>/dev/null | tail -n 1)
echo "Response: $RESPONSE"

if echo "$RESPONSE" | grep -q '"status":"indexed"'; then
    echo -e "${GREEN}✓ Bookmark indexed successfully${NC}"
else
    echo -e "${RED}✗ Failed to index bookmark${NC}"
    exit 1
fi

# Test 3: Check for updates with existing bookmark
echo -e "\n${YELLOW}Test 3: Check for updates with existing bookmark${NC}"
RESPONSE=$(echo '{"jsonrpc":"2.0","id":"test3","method":"check_for_updates","params":{"bookmarks":[{"id":"1","date_modified":"2024-01-01"},{"id":"2","date_modified":"2024-01-02"}],"index_name":"'$TEST_INDEX'"}}' | ./target/release/mcp-bookmark-native 2>/dev/null | tail -n 1)
echo "Response: $RESPONSE"

if echo "$RESPONSE" | grep -q '"new_bookmarks":\["2"\]' && echo "$RESPONSE" | grep -q '"updated_bookmarks":\[\]'; then
    echo -e "${GREEN}✓ Correctly identified bookmark 1 as existing and bookmark 2 as new${NC}"
else
    echo -e "${RED}✗ Failed to identify bookmark status${NC}"
    exit 1
fi

# Test 4: Skip unchanged bookmark
echo -e "\n${YELLOW}Test 4: Skip unchanged bookmark${NC}"
RESPONSE=$(echo '{"jsonrpc":"2.0","id":"test4","method":"index_bookmark","params":{"id":"1","url":"https://example.com","title":"Example","content":"Test content","folder_path":[],"date_added":"2024-01-01","date_modified":"2024-01-01","index_name":"'$TEST_INDEX'","skip_if_unchanged":true}}' | ./target/release/mcp-bookmark-native 2>/dev/null | tail -n 1)
echo "Response: $RESPONSE"

if echo "$RESPONSE" | grep -q '"status":"skipped"'; then
    echo -e "${GREEN}✓ Unchanged bookmark correctly skipped${NC}"
else
    echo -e "${RED}✗ Failed to skip unchanged bookmark${NC}"
    exit 1
fi

# Test 5: Update modified bookmark
echo -e "\n${YELLOW}Test 5: Update modified bookmark${NC}"
RESPONSE=$(echo '{"jsonrpc":"2.0","id":"test5","method":"index_bookmark","params":{"id":"1","url":"https://example.com","title":"Example Updated","content":"Updated content","folder_path":[],"date_added":"2024-01-01","date_modified":"2024-01-10","index_name":"'$TEST_INDEX'"}}' | ./target/release/mcp-bookmark-native 2>/dev/null | tail -n 1)
echo "Response: $RESPONSE"

if echo "$RESPONSE" | grep -q '"status":"indexed"'; then
    echo -e "${GREEN}✓ Modified bookmark re-indexed successfully${NC}"
else
    echo -e "${RED}✗ Failed to re-index modified bookmark${NC}"
    exit 1
fi

# Test 6: Sync metadata
echo -e "\n${YELLOW}Test 6: Sync metadata${NC}"
RESPONSE=$(echo '{"jsonrpc":"2.0","id":"test6","method":"sync_bookmarks","params":{"index_name":"'$TEST_INDEX'"}}' | ./target/release/mcp-bookmark-native 2>/dev/null | tail -n 1)
echo "Response: $RESPONSE"

if echo "$RESPONSE" | grep -q '"status":"synced"'; then
    echo -e "${GREEN}✓ Metadata synced successfully${NC}"
else
    echo -e "${RED}✗ Failed to sync metadata${NC}"
    exit 1
fi

# Cleanup
echo -e "\n${YELLOW}Cleaning up test index...${NC}"
rm -rf ~/Library/Application\ Support/mcp-bookmark/$TEST_INDEX

echo -e "\n${GREEN}All tests passed!${NC}"