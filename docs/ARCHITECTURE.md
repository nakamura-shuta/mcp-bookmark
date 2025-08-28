# Architecture

## Overview

Chrome Bookmark MCP Server provides read-only access to Chrome bookmarks via the Model Context Protocol (MCP). It features full-text search with Chrome extension-based indexing, Japanese language support via Lindera, multi-index search, and parallel indexing for superior performance.

## Core Components

- `bookmark.rs` - Chrome bookmark JSON parsing and filtering
- `chrome_profile.rs` - Chrome profile detection and management  
- `mcp_server.rs` - MCP protocol implementation with 6 tools
- `batch_manager.rs` - Parallel batch processing for high-performance indexing
- `search/` - Tantivy full-text search with advanced features
  - `common.rs` - Shared types and utilities
  - `search_manager.rs` - Index management and search operations
  - `tokenizer.rs` - Lindera tokenizer configuration for Japanese
  - `unified_searcher.rs` - Unified search interface
  - `query_parser.rs` - Advanced query parsing with phrase support
  - `multi_index.rs` - Multi-index parallel search capability
  - `content_index.rs` - Chrome extension index integration
  - `scored_snippet.rs` - Intelligent snippet generation with scoring
- `bin/mcp-bookmark-native.rs` - Native messaging host for Chrome extension

## MCP Tools (5 Available)

1. **search_bookmarks** - Search by title or URL
2. **search_bookmarks_fulltext** - Full-text search with content snippets
3. **get_indexing_status** - Check indexing progress
4. **get_available_profiles** - List Chrome profiles
5. **get_bookmark_content** - Get full content (index-first strategy)

## Advanced Features

### Parallel Indexing (v0.4.3)
- **5-10x faster indexing** with concurrent tab processing
- **Batch processing** with configurable buffer size (default: 50 bookmarks)
- **Smart concurrency** - 1-2 bookmarks use sequential, 3+ use parallel (5 concurrent tabs)
- **Progress tracking** for large bookmark collections
- **Native messaging protocol optimizations** for reduced overhead

### Incremental Index Updates (v0.4.2)
- **Skip unchanged bookmarks** during re-indexing
- **Content hashing** for change detection
- **Metadata tracking** with indexed timestamps
- `check_for_updates` and `sync_bookmarks` native messaging methods
- Reduces re-indexing time from ~50 seconds to ~1 second for unchanged bookmarks

### Multi-Index Search (v0.4.1)
- **Comma-separated INDEX_NAME** support (e.g., "index1,index2,index3")
- **Parallel searching** across multiple indices
- **URL deduplication** with score-based result merging
- **MultiIndexSearchManager** for efficient multi-index operations

### Phrase Search Support (v0.4.1)
- **Quoted queries** for exact phrase matching (e.g., "React hooks")
- **Mixed phrase and word queries** supported
- **Special characters and Japanese text** in phrases
- **QueryParser module** for intelligent query parsing

## Configuration

Set via environment variables:
- `CHROME_PROFILE_NAME` - Chrome profile (e.g., "Work", "Default")
- `CHROME_TARGET_FOLDER` - Bookmark folder (e.g., "Development", "Tech/React", "all")
- `INDEX_NAME` - Custom index name or comma-separated list for multi-index search

## Index Management

### Storage Structure
```
~/Library/Application Support/mcp-bookmark/
├── Work_Development/        # Profile: Work, Folder: Development
├── Default_Development/     # Profile: Default, Folder: Development  
├── Default_all/             # Profile: Default, Folder: all
├── Extension_Bookmarks/     # Chrome extension created index
├── metadata.json            # Index metadata with content hashes
└── logs/
```

### Index Isolation
- Each profile-folder combination has its own independent index
- Example keys: `Work_Development`, `Default_Development`, `Default_all`
- Subfolder support with slash separator (e.g., "Tech/React" → "Tech_React")
- Extension indexes follow pattern: `Extension_{FolderName}`

### Performance Characteristics

| Operation | Performance |
|-----------|------------|
| Initial Indexing | 5-10x faster with parallel processing |
| Re-indexing (unchanged) | ~1 second (skip unchanged) |
| Re-indexing (with changes) | Only changed bookmarks processed |
| Search Query | < 100ms |
| Multi-index Search | Parallel execution |
| Batch Processing | 50 bookmarks per commit |

### Search Features
- **Chrome extension indexing** - Pre-built indexes via browser extension
- **Japanese language support** - Lindera tokenizer for proper segmentation
- **Native messaging** - Bidirectional communication with Chrome extension
- **Read-only indexes** - No runtime content fetching needed
- **Full-text search** returns content_snippet and has_full_content fields
- **Phrase search** with quoted query support
- **Multi-index search** for searching across multiple bookmark sets