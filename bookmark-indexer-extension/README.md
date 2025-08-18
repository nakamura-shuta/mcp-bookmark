# Bookmark Indexer Chrome Extension

Chrome extension that indexes bookmark content for the MCP server using local Tantivy search engine.

## Quick Installation

### 1. Build the binaries
```bash
# From project root
cargo build --release --bin mcp-bookmark-native
```

### 2. Load the extension in Chrome
1. Open Chrome and go to `chrome://extensions/`
2. Enable "Developer mode" (top right)
3. Click "Load unpacked"
4. Select the `bookmark-indexer-extension` folder
5. Copy the Extension ID that appears

### 3. Setup Native Messaging
Create the configuration file:
```bash
# Replace YOUR_EXTENSION_ID with the ID from step 2
cat > ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json << EOF
{
  "name": "com.mcp_bookmark",
  "description": "MCP Bookmark Native Host",
  "path": "$(pwd)/target/release/mcp-bookmark-native",
  "type": "stdio",
  "allowed_origins": ["chrome-extension://YOUR_EXTENSION_ID/"]
}
EOF
```

### 4. Restart Chrome completely (Cmd+Q)

## Usage

### Creating an Index
1. Click the extension icon in Chrome toolbar
2. Enter an index name (e.g., "work", "personal", "research")
3. Select a bookmark folder to index
4. Click "Index Selected Folder"
5. Wait for "Indexing complete!" message

### Verify the Index
```bash
# List all indexes
./target/release/mcp-bookmark --list-indexes

# You should see your index listed, e.g.:
# Extension_work (123 documents, 5.2MB)
```

## Using with MCP Server

After creating an index, configure your MCP client:

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "./target/release/mcp-bookmark",
      "env": {
        "RUST_LOG": "info",
        "INDEX_NAME": "Extension_work"
      }
    }
  }
}
```

Replace `Extension_work` with your actual index name from `--list-indexes`.

## Features

- **Index Selected Folder** - Index all bookmarks in a folder
- **Index Current Tab** - Add current page to index
- **Test Connection** - Verify Native Messaging setup
- **Clear Index** - Remove the current index

## Storage Location

Indexes are stored at:
```
~/Library/Application Support/mcp-bookmark/Extension_[IndexName]/
```

## Troubleshooting

### "Specified native messaging host not found"
- Restart Chrome completely (Cmd+Q)
- Verify the path in the JSON config is absolute and correct
- Check the Extension ID matches

### Connection test fails
- Ensure `mcp-bookmark-native` binary exists and is executable
- Check the path is absolute (starts with `/`)
- Verify Extension ID in the config file

### Some pages fail to index
- Login-required pages work if you're logged in
- JavaScript-heavy sites may need time to load (3-second delay for Notion)
- Some sites block content extraction

### Check logs
```bash
# Native messaging logs
tail -f /tmp/mcp-bookmark-native.log

# List indexes
./target/release/mcp-bookmark --list-indexes
```

## Architecture

```
Chrome Extension → Native Messaging → mcp-bookmark-native → Tantivy Index
                                                               ↑
                                                        MCP Server reads
```

## License

MIT