# Search System Design

## Current Implementation

The search system uses a simple, efficient approach focused on practical usage patterns.

## Key Design Decisions

### Single Search System
- **Tantivy-only** full-text search
- No fallback mechanisms or hybrid approaches
- Simpler codebase with better maintainability

### Background Indexing
- Starts automatically on server startup
- Prioritizes documentation sites (docs.rs, react.dev, MDN)
- Indexes content progressively in background

### Index-First Strategy
The `get_bookmark_content` tool follows this approach:
1. Check if content exists in index
2. If not found, fetch from web
3. Store in index for future use

## Search Tools Behavior

### search_bookmarks
- Searches bookmark titles and URLs only
- Fast metadata-based search
- Always available immediately

### search_bookmarks_fulltext  
- Full-text search through page content
- Returns `content_snippet` and `has_full_content` fields
- Includes indexing status in response
- Supports folder and domain filtering

### get_indexing_status
- Shows current indexing progress
- Indicates completion status
- Helps users understand search readiness

## User Experience Timeline

| Time | Available Features |
|------|-------------------|
| 0s | Metadata search (titles/URLs) |
| 10-30s | Core documentation content |
| ~2-5min | Most bookmark content indexed |

## Benefits

- **Simplified codebase** - Single search path
- **Predictable behavior** - No complex fallback logic  
- **Better performance** - Lower memory usage
- **Easier testing** - Deterministic behavior