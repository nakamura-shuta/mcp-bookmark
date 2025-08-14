[日本語](README.ja.md) | English

# Chrome Bookmark MCP Server

**Search inside your bookmarked pages with AI** - Index even login-required sites with Chrome extension, enabling fast full-text search

💡 **Key Features**:
- 🔐 **Works with authenticated pages** - Chrome extension fetches content from your logged-in browser
- ⚡ **Fast local search** - Indexed with Tantivy engine, no external API calls
- 🎯 **AI-friendly** - Claude can search bookmark contents to answer your questions

**⚠️ Requirements**: macOS + Google Chrome

## Features

- **Full-Text Search**: Search bookmark content using Tantivy search engine
- **Chrome Extension**: Index bookmark content directly from browser
- **Multiple Profiles**: Support for multiple Chrome profiles
- **Folder Filtering**: Expose only specific bookmark folders

## Quick Start

### 1. Build the Server

```bash
# Clone and build
git clone https://github.com/USERNAME/mcp-bookmark.git
cd mcp-bookmark
cargo build --release

# Verify installation
./target/release/mcp-bookmark --help
```

### 2. Install Chrome Extension (Recommended)

The Chrome extension provides better content indexing:

1. Build the native messaging host:
   ```bash
   cargo build --release --bin mcp-bookmark-native
   ```

2. Install the extension - see [Extension README](bookmark-indexer-extension/README.md)

3. Verify index creation:
   ```bash
   # List all created indexes
   ./target/release/mcp-bookmark --list-indexes
   # Example: Extension_Development (123 documents, 5.2MB)
   ```

### 3. Configure MCP

Add to your Claude Desktop config file (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "env": {
        "CHROME_PROFILE_NAME": "Extension",
        "CHROME_TARGET_FOLDER": "your-folder-name"
      }
    }
  }
}
```

**Important**:
- Replace `/path/to/mcp-bookmark` with your actual project path
- Replace `your-folder-name` with the exact folder name you indexed with the Chrome extension
- `CHROME_PROFILE_NAME` should always be `"Extension"` when using the Chrome extension

## Usage

### With Chrome Extension (Recommended)

1. Open the Chrome extension popup
2. Select a folder to index
3. Click "Index Selected Folder"
4. Use the indexed content in your AI assistant

### Command Line Options

```bash
# Use pre-built index from Chrome extension
CHROME_PROFILE_NAME="Extension" CHROME_TARGET_FOLDER="Development" ./target/release/mcp-bookmark

# Index management
./target/release/mcp-bookmark --list-indexes
./target/release/mcp-bookmark --clear-index
```

## MCP Tools Available

- `search_bookmarks` - Search by title/URL
- `search_bookmarks_fulltext` - Full-text content search
- `get_bookmark_content` - Get content for specific URL
- `list_bookmark_folders` - List available folders
- `get_indexing_status` - Check indexing progress

## Index Storage

Indexes are stored at:
- macOS: `~/Library/Application Support/mcp-bookmark/`

Each profile/folder combination has its own index.

## License

MIT