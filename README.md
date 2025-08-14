[日本語](README.ja.md) | English

# Chrome Bookmark MCP Server

**⚠️ macOS and Chrome Only**: This tool currently supports only macOS and Google Chrome.

MCP (Model Context Protocol) server providing AI assistants with access to your Chrome bookmarks with full-text search capabilities.

## Features

- **Full-Text Search**: Search bookmark content using the tantivy search engine
- **Chrome Profile Support**: Works with multiple Chrome profiles
- **Folder Filtering**: Expose only specific bookmark folders
- **Auto-indexing**: Background indexing of web page content
- **Chrome Extension**: Optional extension for enhanced content indexing

## Requirements

- macOS
- Google Chrome
- Rust 1.70+ (for building from source)

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/USERNAME/mcp-bookmark.git
cd mcp-bookmark

# Build the release binary
cargo build --release

# Test the build
./target/release/mcp-bookmark --help
```

## Configuration

### Basic Setup

Create a `.mcp.json` file in your project root:

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/path/to/mcp-bookmark/target/release/mcp-bookmark"
    }
  }
}
```

### Advanced Options

#### Specific Folder
```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "env": {
        "CHROME_TARGET_FOLDER": "Development"
      }
    }
  }
}
```

#### Chrome Profile
```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "env": {
        "CHROME_PROFILE_NAME": "Work"
      }
    }
  }
}
```

Note: Use the display name shown in Chrome (e.g., "Work", "Personal"), not internal directory names.

## Usage

### Command Line

```bash
# Basic usage
mcp-bookmark                        # All bookmarks
mcp-bookmark Development            # Specific folder
mcp-bookmark Development 10         # Max 10 from Development

# Profile and folder options
mcp-bookmark --profile Work --folder Development

# Index management
mcp-bookmark --list-indexes         # List indexes
mcp-bookmark --clear-index          # Clear index for current config
mcp-bookmark --clear-index Work_Tech  # Clear specific index
mcp-bookmark --clear-all-indexes    # Clear all indexes
```

### Available MCP Tools

1. **search_bookmarks** - Search by title or URL
2. **search_bookmarks_fulltext** - Full-text content search
3. **get_bookmark_content** - Retrieve full page content
4. **list_bookmark_folders** - List available folders
5. **get_available_profiles** - List Chrome profiles

### Usage with AI Assistant

```
"Search bookmarks in Development folder"
"Find React documentation"
"Show recently added bookmarks"
```

## Chrome Extension (Optional)

The extension enhances content indexing by fetching web page content directly from Chrome.

### Installation Steps

1. **Build Native Host**:
```bash
cargo build --release --bin mcp-bookmark-native
```

2. **Configure Native Messaging** (run from project root):
```bash
# Run this from the project root directory
cat > ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json << EOF
{
  "name": "com.mcp_bookmark",
  "description": "Bookmark Indexer Native Host",
  "path": "$(pwd)/target/release/mcp-bookmark-native",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID/"
  ]
}
EOF
```

3. **Install Extension**:
   - Open `chrome://extensions/`
   - Enable "Developer mode"
   - Click "Load unpacked"
   - Select `bookmark-indexer-extension` folder
   - Note the Extension ID

4. **Update Configuration**:
```bash
# Replace with your actual extension ID
EXTENSION_ID="your-extension-id"
sed -i "" "s/YOUR_EXTENSION_ID/$EXTENSION_ID/g" \
  ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json
```

5. **Restart Chrome completely**

### Extension Usage

- Click extension icon in toolbar
- Select bookmark folder
- Click "Index Selected Folder"

## Index Location

Indexes are stored at:
```
~/Library/Application Support/mcp-bookmark/index/
```

## License

MIT