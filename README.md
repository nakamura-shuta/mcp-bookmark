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
- **Phrase Search**: Use quotes for exact phrase matching (e.g., "React hooks")
- **Chrome Extension**: Index bookmark content directly from browser
- **Custom Indexes**: Create and manage multiple independent indexes
- **Folder Filtering**: Expose only specific bookmark folders

## Quick Start

### Option 1: Build from Source (Requires Rust)

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

### Option 2: Use Pre-built Binaries (No Rust Required)

1. Create a directory for the installation:
```bash
mkdir ~/mcp-bookmark
cd ~/mcp-bookmark
```

2. Download pre-built binaries from the [latest release](https://github.com/nakamura-shuta/mcp-bookmark/releases/latest):

#### macOS (Intel)
```bash
# Download binaries
curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-x64 -o mcp-bookmark
curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-x64-native -o mcp-bookmark-native
chmod +x mcp-bookmark mcp-bookmark-native
```

#### macOS (Apple Silicon)
```bash
# Download binaries
curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-arm64 -o mcp-bookmark
curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-arm64-native -o mcp-bookmark-native
chmod +x mcp-bookmark mcp-bookmark-native
```

### Detailed Steps

#### For Option 1: Building from Source

##### Step 1: Run the Install Script

The script will build everything and guide you through setup.

#### For Option 2: Using Pre-built Binaries

##### Step 1: Download and Setup Binaries

After downloading the binaries as shown above, configure the native messaging host:

```bash
# Create native messaging host manifest (make sure you're in ~/mcp-bookmark directory)
mkdir -p ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/
cat > ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json << EOF
{
  "name": "com.mcp_bookmark",
  "description": "MCP Bookmark Native Messaging Host",
  "path": "$HOME/mcp-bookmark/mcp-bookmark-native",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID_HERE/"
  ]
}
EOF
```

#### Step 2: Install Chrome Extension

**For Option 1 (Built from source):**
1. Open Chrome and go to `chrome://extensions/`
2. Enable "Developer mode" (top right)
3. Click "Load unpacked"
4. Select `mcp-bookmark/bookmark-indexer-extension` folder
5. Copy the Extension ID and paste it when prompted

**For Option 2 (Pre-built):**
1. Download the extension: 
   ```bash
   curl -L https://github.com/nakamura-shuta/mcp-bookmark/releases/latest/download/bookmark-indexer-chrome-extension.zip -o extension.zip
   unzip extension.zip -d bookmark-indexer-extension
   ```
2. Open Chrome and go to `chrome://extensions/`
3. Enable "Developer mode" (top right)
4. Click "Load unpacked" and select the extracted `bookmark-indexer-extension` folder
5. Copy the Extension ID
6. Update the native messaging host manifest with the Extension ID:
   ```bash
   # Replace YOUR_EXTENSION_ID_HERE with the actual Extension ID
   sed -i '' "s/YOUR_EXTENSION_ID_HERE/YOUR_ACTUAL_EXTENSION_ID/" ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json
   ```

#### Step 3: Create Your First Index (when prompted)

1. Click the extension icon in Chrome toolbar
2. Enter an index name (e.g., "my-bookmarks")
3. Select a bookmark folder to index
4. Click "Index Selected Folder"
5. Wait for completion, then return to terminal

#### Step 4: Complete Setup

**For Option 1 (Built from source):**
1. Enter the index name you just created
2. Copy `.mcp.json` to your project:
   ```bash
   cp .mcp.json ~/your-project/
   ```

**For Option 2 (Pre-built):**
1. Create `.mcp.json` configuration file (make sure you're in ~/mcp-bookmark directory):
   ```bash
   cat > .mcp.json << EOF
   {
     "mcpServers": {
       "mcp-bookmark": {
         "command": "$HOME/mcp-bookmark/mcp-bookmark",
         "args": [],
         "env": {
           "RUST_LOG": "info",
           "INDEX_NAME": "YOUR_INDEX_NAME"
         }
       }
     }
   }
   EOF
   ```
2. Replace `YOUR_INDEX_NAME` with the index name you created in Step 3
3. Copy to your project:
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
# For built from source:
INDEX_NAME="work_Development" ./target/release/mcp-bookmark

# For pre-built binaries (from ~/mcp-bookmark directory):
INDEX_NAME="work_Development" ./mcp-bookmark

# Multi-index search (comma-separated)
INDEX_NAME="work,personal,research" ./mcp-bookmark

# Index management commands
./mcp-bookmark --list-indexes      # List all available indexes
./mcp-bookmark --clear-index       # Clear current index  
./mcp-bookmark --clear-all-indexes # Clear all indexes
```

## MCP Tools Available

- `search_bookmarks_fulltext` - Full-text content search (searches titles, URLs, and page content)
  - Supports phrase search with quotes (e.g., "exact phrase")
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
