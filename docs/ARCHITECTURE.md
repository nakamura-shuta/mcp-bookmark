# Architecture

## Overview

Chrome Bookmark MCP Server provides read-only access to Chrome bookmarks via the Model Context Protocol (MCP). It features full-text search with Chrome extension-based indexing, Japanese language support via Lindera, and multi-index search.

## Core Components

- `bookmark.rs` - Chrome bookmark JSON parsing and filtering
- `chrome_profile.rs` - Chrome profile detection and management
- `mcp_server.rs` - MCP protocol implementation
- `search/` - Tantivy full-text search with advanced features
  - `common.rs` - Shared types and utilities (IndexStats, IndexingStatus)
  - `search_manager.rs` - Index management and search operations
  - `tokenizer.rs` - Lindera tokenizer configuration for Japanese
  - `unified_searcher.rs` - Unified search interface
  - `query_parser.rs` - Advanced query parsing with phrase support
  - `multi_index.rs` - Multi-index parallel search capability
  - `indexer.rs` - Bookmark indexing with page-based splitting
  - `scored_snippet.rs` - Intelligent snippet generation with scoring
- `bin/mcp-bookmark-native.rs` - Native messaging host for Chrome extension

## MCP Tools (4 Available)

1. **search_bookmarks_fulltext** - Full-text search with content snippets
2. **get_indexing_status** - Check indexing progress
3. **get_bookmark_content** - Get full content (index-first strategy)
4. **get_bookmark_content_range** - Get specific pages from PDF bookmarks

## Chrome Extension

### Supported Native APIs
The native messaging host (`mcp-bookmark-native`) supports the following methods:

| Method | Description |
|--------|-------------|
| `ping` | Health check, returns indexer status |
| `index_bookmark` | Index a single bookmark with content |
| `list_indexes` | List all available indexes |
| `get_stats` | Get index statistics |
| `initialize` | MCP protocol compatibility |

### Minimum Extension Version
The current native host requires Extension version 1.0.0 or later.
Older extensions using deprecated APIs (`batch_*`, `sync_bookmarks`, `check_for_updates`, `index_bookmark_chunk`) are not supported.

## Advanced Features

### Extension-based Indexing
- **Single bookmark indexing** - Each bookmark indexed individually via `index_bookmark`
- **Content fetching in Extension** - Extension handles web page content extraction
- **PDF support** - PDF.js integration for text extraction with CJK support
- **Progress tracking** - Real-time progress updates during indexing

### Incremental Index Updates
- **Skip unchanged bookmarks** during re-indexing
- **Content hashing** for change detection
- **Metadata tracking** with indexed timestamps
- Reduces re-indexing time for unchanged bookmarks

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
| Search Query | < 100ms |
| Multi-index Search | Parallel execution |
| Re-indexing (unchanged) | ~1 second (skip unchanged) |

### Page-based Content Retrieval (v0.6.0)
- **Page-based PDF access** - Retrieve specific pages without loading entire PDF
- **Token limit protection** - Prevents token overflow on large documents (17MB+ PDFs)
- **Page markers** - Content indexed with `[PAGE:n]` markers for navigation
- **Smart warnings** - Alerts when content >100k chars, suggests using page retrieval
- **Flexible retrieval** - Single page or page ranges (e.g., pages 40-45)
- **Schema extension** - page_count, page_offsets, content_type fields in index

### Search Features
- **Chrome extension indexing** - Pre-built indexes via browser extension
- **Japanese language support** - Lindera tokenizer for proper segmentation
- **Native messaging** - Communication with Chrome extension
- **Read-only indexes** - No runtime content fetching needed
- **Full-text search** returns content_snippet and has_full_content fields
- **Phrase search** with quoted query support
- **Multi-index search** for searching across multiple bookmark sets
