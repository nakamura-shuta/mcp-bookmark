# Architecture

## Overview

Chrome Bookmark MCP Server provides read-only access to Chrome bookmarks via the Model Context Protocol (MCP). It features full-text search with background indexing and multi-profile support.

## Core Components

- `bookmark.rs` - Chrome bookmark JSON parsing and filtering
- `chrome_profile.rs` - Chrome profile detection and management  
- `mcp_server.rs` - MCP protocol implementation with 6 tools
- `search/` - Tantivy full-text search engine integration
- `content.rs` - Web page content fetching

## MCP Tools (6 Available)

1. **search_bookmarks** - Search by title or URL
2. **list_bookmark_folders** - List bookmark folders
3. **search_bookmarks_fulltext** - Full-text search with content snippets
4. **get_indexing_status** - Check indexing progress
5. **get_available_profiles** - List Chrome profiles
6. **get_bookmark_content** - Get full content (index-first strategy)

## Configuration

Set via environment variables:
- `CHROME_PROFILE_NAME` - Chrome profile (e.g., "Work", "Default")
- `CHROME_TARGET_FOLDER` - Bookmark folder (e.g., "Development", "Tech/React", "all")

## Index Management

### Storage Structure
```
~/Library/Application Support/mcp-bookmark/
├── Work_Development/        # Profile: Work, Folder: Development
├── Default_Development/     # Profile: Default, Folder: Development  
├── Default_all/             # Profile: Default, Folder: all
└── logs/
```

### Index Isolation
- Each profile-folder combination has its own independent index
- Example keys: `Work_Development`, `Default_Development`, `Default_all`
- Subfolder support with slash separator (e.g., "Tech/React" → "Tech_React")

### Search Features
- **Background indexing** starts on server startup
- **Priority content fetching** (docs.rs, react.dev, MDN prioritized)
- **Index-first retrieval** for get_bookmark_content
- **Full-text search** returns content_snippet and has_full_content fields