[日本語](README.ja.md) | English

# Chrome Bookmark MCP Server

MCP (Model Context Protocol) server providing access to Chrome bookmarks

## Features

- **Fast Full-Text Search**: Bookmark content search powered by tantivy search engine (with snippets in results)
- **Content Caching**: Direct content retrieval from index DB (no remote fetching required)
- **Auto-Indexing**: Automatic background fetching and storing of web page content
- **Profile Support**: Select from multiple Chrome profiles
- **Folder Filtering**: Expose only specific folder bookmarks
- **Independent Index Management**: Separate indexes per profile/folder combination

## Quick Start (Easiest Way)

For other projects, simply use the absolute path in your `.mcp.json`:

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

**No installation required!** Just build once and reference the binary path.

## Installation

### Option 1: Download Pre-built Binary

#### macOS (Apple Silicon)

```bash
curl -L https://github.com/USERNAME/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-arm64 -o mcp-bookmark
chmod +x mcp-bookmark
sudo mv mcp-bookmark /usr/local/bin/
```

#### macOS (Intel)

```bash
curl -L https://github.com/USERNAME/mcp-bookmark/releases/latest/download/mcp-bookmark-darwin-x64 -o mcp-bookmark
chmod +x mcp-bookmark
sudo mv mcp-bookmark /usr/local/bin/
```

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/USERNAME/mcp-bookmark.git
cd mcp-bookmark

# Build the release binary
cargo build --release

# The binary will be available at:
# $(pwd)/target/release/mcp-bookmark
```

### Verify Build

```bash
# Test the binary
./target/release/mcp-bookmark --help
```

## Configuration

### Basic Configuration

`~/.config/claude/config.json`:

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/full/path/to/mcp-bookmark/target/release/mcp-bookmark"
    }
  }
}
```

### Project-Specific Configuration

Place `.mcp.json` in your project root directory to enable project-specific MCP configuration.

`.mcp.json`:

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/full/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "args": ["Development", "100"]
    }
  }
}
```

This allows different bookmark folders and settings per project.

### Expose Specific Folder Only

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/full/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "args": ["Development", "100"]
    }
  }
}
```

### Subfolder Specification

Use slash (`/`) to specify subfolders:

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/full/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "env": {
        "CHROME_TARGET_FOLDER": "Development/React"
      }
    }
  }
}
```

This feature allows exposing only specific nested subfolders.

### Profile Specification

You can specify which Chrome profile to use by setting the `CHROME_PROFILE_NAME` environment variable. The profile name should be the display name shown in Chrome, not the internal directory name.

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/full/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "env": {
        "CHROME_PROFILE_NAME": "Work"  // Use display name, not directory name like "Default"
      }
    }
  }
}
```

To check available profiles, use the MCP tool or run:
```bash
mcp-bookmark --list-profiles
```

**Note**: If `CHROME_PROFILE_NAME` is not specified, the server will auto-detect and use the profile with the largest bookmarks file.

## Usage

### Command Line

```bash
mcp-bookmark                        # All bookmarks
mcp-bookmark Development            # Development folder only
mcp-bookmark Development 100        # Max 100 items
mcp-bookmark Work,Tech              # Multiple folders

mcp-bookmark --profile Work         # Work profile
mcp-bookmark --folder Development   # Specific folder
mcp-bookmark --exclude Archive      # Exclude folder

# Index management
mcp-bookmark --list-indexes         # List indexes
mcp-bookmark --clear-index          # Clear current config index
mcp-bookmark --clear-all-indexes    # Clear all indexes
```

### Available Tools (for MCP Clients)

1. **search_bookmarks** - Search bookmarks by title or URL
2. **search_bookmarks_fulltext** - Full-text search (including content, with snippets in results)
3. **get_bookmark_content** - Get full content from URL (from index DB)
4. **list_bookmark_folders** - Get list of bookmark folders
5. **get_indexing_status** - Check indexing status
6. **get_available_profiles** - Get list of available Chrome profiles

### Usage Examples with AI Assistant

```
"Search bookmarks in Development folder"
"Find React-related documentation"
"Show recently added bookmarks"
"Tell me more about the content of this URL" (retrieves full text with get_bookmark_content)
```

## Index Management

Search indexes are managed independently for each profile and folder combination:

```
~/Library/Application Support/mcp-bookmark/
├── Default_Development/      # Default profile, Development folder
├── Work_Tech_React/         # Work profile, Tech/React folder
└── Personal_all/            # Personal profile, all bookmarks
```

### Features

- **Isolated Management**: Projects with same profile/folder settings share the same index
- **Auto-Creation**: Index created automatically on first launch
- **Background Updates**: Content indexed progressively after server starts

### Management Commands

```bash
# List indexes (shows size and update time)
mcp-bookmark --list-indexes

# Clear specific index
mcp-bookmark --clear-index Default_Development

# Clear all indexes
mcp-bookmark --clear-all-indexes
```

## Using in Other Projects

Create a `.mcp.json` file in your project root with the full path to the binary:

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/full/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "env": {
        "RUST_LOG": "info",
        "CHROME_PROFILE_NAME": "Work",  // Use display name (e.g., "Work", "Personal")
        "CHROME_TARGET_FOLDER": "YourProjectFolder"
      }
    }
  }
}
```

**Note**: 
- `CHROME_PROFILE_NAME` should be the display name shown in Chrome (e.g., "Work", "Personal"), not the internal directory name (e.g., "Default", "Profile 1")
- Adjust `CHROME_TARGET_FOLDER` to match your project's bookmark folder
- If `CHROME_PROFILE_NAME` is omitted, the server will auto-detect the profile with the largest bookmarks file

## Troubleshooting

### Common Issues

#### "Connection failed: MCP error -32000"

This error typically means the binary path is incorrect or the binary doesn't exist.

**Solution**:
1. Ensure the binary is built: `cargo build --release`
2. Verify the full path to the binary is correct
3. Ensure your `.mcp.json` uses the full absolute path to the binary

### Check Chrome Profiles

```bash
# List profiles
ls ~/Library/Application\ Support/Google/Chrome/*/Bookmarks

# Check profile path at chrome://version/
```

### Log Files

```
~/Library/Application Support/mcp-bookmark/logs/
```

Change log level:

```json
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "/full/path/to/mcp-bookmark/target/release/mcp-bookmark",
      "env": { "RUST_LOG": "debug" }
    }
  }
}
```

## Search Index

The index is built automatically and stored at:

```
~/Library/Application Support/mcp-bookmark/index/
```

## License

MIT
