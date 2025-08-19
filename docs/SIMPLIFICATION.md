# Search System Design

## Current Implementation

The search system uses a Chrome extension-based indexing approach with Japanese language support.

## Key Design Decisions

### Chrome Extension Integration
- **Pre-built indexes** created by Chrome extension
- **Native messaging host** for communication
- **Read-only access** to extension-created indexes
- No server-side content fetching needed

### Japanese Language Support
- **Lindera tokenizer** for proper Japanese text segmentation
- **UTF-8 safe** text processing
- Works seamlessly with English content
- Automatic language detection

### Index-First Strategy
The `get_bookmark_content` tool follows this approach:
1. Check if content exists in Chrome extension index
2. Return indexed content directly
3. No web fetching - all content pre-indexed

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
| 0s | Full-text search with pre-indexed content |
| 0s | Japanese and English language support |
| 0s | All bookmark content immediately available |

## Benefits

- **Instant availability** - All content pre-indexed by Chrome extension
- **Multi-language support** - Japanese tokenization with Lindera
- **Zero latency** - No runtime content fetching
- **Better reliability** - No network dependencies
- **Simplified architecture** - Read-only index access