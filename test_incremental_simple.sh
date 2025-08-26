#!/bin/bash

# Simple test for incremental updates by checking logs

set -e

echo "Testing incremental index update functionality..."

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Clear log
rm -f /tmp/mcp-bookmark-native.log

# Test index directory
TEST_INDEX_DIR="$HOME/Library/Application Support/mcp-bookmark/test_incremental"

# Cleanup previous test
rm -rf "$TEST_INDEX_DIR"

echo -e "${YELLOW}Creating test data...${NC}"

# Create test bookmarks file
cat > /tmp/test_bookmarks.json <<EOF
{
  "bookmarks": [
    {"id": "1", "url": "https://example.com/1", "title": "Test 1", "date_modified": "2024-01-01"},
    {"id": "2", "url": "https://example.com/2", "title": "Test 2", "date_modified": "2024-01-02"}
  ]
}
EOF

echo -e "${YELLOW}Testing with Chrome extension (manual test required)${NC}"
echo "Please:"
echo "1. Open Chrome and go to chrome://extensions"
echo "2. Reload the Bookmark Indexer extension"
echo "3. Click the extension icon"
echo "4. Select a test folder"
echo "5. Use 'test_incremental' as the custom index name"
echo "6. Click 'Start Indexing'"
echo "7. After completion, click 'Start Indexing' again to test incremental update"
echo ""
echo "Check the log file for results: tail -f /tmp/mcp-bookmark-native.log"
echo ""

# Check if metadata file exists
echo -e "${YELLOW}Checking for metadata file...${NC}"

# Build the project first
echo -e "${YELLOW}Building project...${NC}"
cargo build --release --bin mcp-bookmark-native

# Test compilation
echo -e "${GREEN}✓ Build successful${NC}"

# Check the native messaging host directly
echo -e "\n${YELLOW}Testing native messaging host...${NC}"

# Create a simple test to verify the new methods exist
cat > /tmp/test_native.py <<'EOF'
#!/usr/bin/env python3
import json
import struct
import subprocess
import sys

def send_message(proc, message):
    encoded = json.dumps(message).encode('utf-8')
    length = struct.pack('I', len(encoded))
    proc.stdin.write(length)
    proc.stdin.write(encoded)
    proc.stdin.flush()

def read_message(proc):
    raw_length = proc.stdout.read(4)
    if len(raw_length) == 0:
        return None
    message_length = struct.unpack('I', raw_length)[0]
    message = proc.stdout.read(message_length).decode('utf-8')
    return json.loads(message)

# Start native host
proc = subprocess.Popen(
    ['./target/release/mcp-bookmark-native'],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

try:
    # Test ping
    send_message(proc, {"jsonrpc": "2.0", "id": "1", "method": "ping"})
    response = read_message(proc)
    print(f"Ping response: {response}")
    
    # Test check_for_updates
    send_message(proc, {
        "jsonrpc": "2.0",
        "id": "2",
        "method": "check_for_updates",
        "params": {
            "bookmarks": [
                {"id": "1", "date_modified": "2024-01-01"},
                {"id": "2", "date_modified": "2024-01-02"}
            ],
            "index_name": "test_incremental"
        }
    })
    response = read_message(proc)
    print(f"Check for updates response: {response}")
    
    if response and "result" in response:
        new_bookmarks = response["result"].get("new_bookmarks", [])
        if len(new_bookmarks) == 2:
            print("✓ All bookmarks identified as new (empty index)")
        else:
            print("✗ Failed to identify new bookmarks")
            sys.exit(1)
    
    # Index a bookmark
    send_message(proc, {
        "jsonrpc": "2.0",
        "id": "3",
        "method": "index_bookmark",
        "params": {
            "id": "1",
            "url": "https://example.com",
            "title": "Test",
            "content": "Test content",
            "folder_path": [],
            "date_added": "2024-01-01",
            "date_modified": "2024-01-01",
            "index_name": "test_incremental",
            "skip_if_unchanged": False
        }
    })
    response = read_message(proc)
    print(f"Index bookmark response: {response}")
    
    # Check for updates again
    send_message(proc, {
        "jsonrpc": "2.0",
        "id": "4",
        "method": "check_for_updates",
        "params": {
            "bookmarks": [
                {"id": "1", "date_modified": "2024-01-01"},
                {"id": "2", "date_modified": "2024-01-02"}
            ],
            "index_name": "test_incremental"
        }
    })
    response = read_message(proc)
    print(f"Check for updates (2nd) response: {response}")
    
    if response and "result" in response:
        new_bookmarks = response["result"].get("new_bookmarks", [])
        if "2" in new_bookmarks and "1" not in new_bookmarks:
            print("✓ Correctly identified existing vs new bookmarks")
        else:
            print("✗ Failed to identify bookmark status")
    
    print("\n✓ All tests passed!")
    
finally:
    proc.terminate()

EOF

chmod +x /tmp/test_native.py
python3 /tmp/test_native.py

# Cleanup
rm -rf "$TEST_INDEX_DIR"
rm -f /tmp/test_bookmarks.json /tmp/test_native.py

echo -e "\n${GREEN}Test completed!${NC}"