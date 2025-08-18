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

### Automated Installation (Recommended)

```bash
# Clone and run setup script
git clone https://github.com/nakamura-shuta/mcp-bookmark.git
cd mcp-bookmark
./install.sh
```

The setup script will:

- ✅ Check prerequisites (macOS, Chrome, Rust)
- ✅ Build all required binaries
- ✅ Configure Chrome extension
- ✅ Create local .mcp.json configuration
- ✅ Verify installation

### Manual Installation

<details>
<summary>Click for manual installation steps</summary>

#### 1. Build the Server

```bash
# Clone and build
git clone https://github.com/nakamura-shuta/mcp-bookmark.git
cd mcp-bookmark
cargo build --release

# Verify installation
./target/release/mcp-bookmark --help
```

#### 2. Install Chrome Extension

1. Build the native messaging host:

   ```bash
   cargo build --release --bin mcp-bookmark-native
   ```

2. Install the extension - see [Extension README](bookmark-indexer-extension/README.md)

3. Verify index creation:
   ```bash
   # List all created indexes
   ./target/release/mcp-bookmark --list-indexes
   # Example: work_Development (123 documents, 5.2MB)
   ```

#### 3. Configure MCP

Create a `.mcp.json` configuration file in the project root:

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "./target/release/mcp-bookmark",
      "args": [],
      "env": {
        "RUST_LOG": "info",
        "INDEX_NAME": "your-index-name"
      }
    }
  }
}
```

**Important**:

- Replace `your-index-name` with the index name created by Chrome extension
- Run `./target/release/mcp-bookmark --list-indexes` to see available indexes

</details>

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
