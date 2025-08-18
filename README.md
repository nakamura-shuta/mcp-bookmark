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
- **Custom Indexes**: Create and manage multiple independent indexes
- **Folder Filtering**: Expose only specific bookmark folders

## Quick Start

```bash
git clone https://github.com/nakamura-shuta/mcp-bookmark.git
cd mcp-bookmark
./install.sh
```

The install script will guide you through:

1. **Building the binaries** - Compiles the MCP server and native messaging host
2. **Installing Chrome extension** - You'll manually load it and provide the Extension ID
3. **Creating your first index** - Using the Chrome extension to index bookmark folders
4. **Generating .mcp.json** - With your chosen index name

### Detailed Steps

#### Step 1: Run the Install Script
The script will build everything and guide you through setup.

#### Step 2: Install Chrome Extension (when prompted)
1. Open Chrome and go to `chrome://extensions/`
2. Enable "Developer mode" (top right)
3. Click "Load unpacked"
4. Select `mcp-bookmark/bookmark-indexer-extension` folder
5. Copy the Extension ID and paste it when prompted

#### Step 3: Create Your First Index (when prompted)
1. Click the extension icon in Chrome toolbar
2. Enter an index name (e.g., "my-bookmarks")
3. Select a bookmark folder to index
4. Click "Index Selected Folder"
5. Wait for completion, then return to terminal

#### Step 4: Complete Setup
1. Enter the index name you just created
2. Copy `.mcp.json` to your project:
   ```bash
   cp .mcp.json ~/your-project/
   ```

#### Step 5: Use in Claude Code
1. In Claude Code, run: `/mcp`
2. Select "mcp-bookmark" to activate
3. Try it out:
   ```
   "Search my bookmarks for React hooks documentation"
   ```

## Usage

### With Chrome Extension (Recommended)

1. Open the Chrome extension popup
2. (Optional) Enter a custom index name
3. Select a folder to index
4. Click "Index Selected Folder"
5. Use the indexed content in your AI assistant

### Command Line Options

```bash
# Run MCP server with specific index
INDEX_NAME="work_Development" ./target/release/mcp-bookmark

# Index management commands
./target/release/mcp-bookmark --list-indexes      # List all available indexes
./target/release/mcp-bookmark --clear-index       # Clear current index
./target/release/mcp-bookmark --clear-all-indexes # Clear all indexes
```

## MCP Tools Available

- `search_bookmarks_fulltext` - Full-text content search (searches titles, URLs, and page content)
  - Returns preview snippets (300 chars) for quick identification
  - Automatically limited to prevent token overflow
  - Use `limit` parameter to control result count
- `get_bookmark_content` - Get complete content for specific URL
  - Use after search to get full page content
  - No size limitations
- `get_indexing_status` - Check indexing progress

## Index Storage

Indexes are stored at:

- macOS: `~/Library/Application Support/mcp-bookmark/`

Each index is managed independently.

## License

MIT
