# Changelog

## [Unreleased]

## [v0.5.1] - 2025-09-08

### Added

- **Configurable snippet length** for search results
  - New `max_snippet_length` field in Config struct (default: 600 chars)
  - Consistent snippet length handling across all search components
  - Dynamic context window calculation based on snippet length

### Improved

- **Search result quality**
  - Better context detection with proportional window sizes
  - UTF-8 safe snippet truncation for all languages
  - More consistent snippet generation across different search paths

- **Documentation**
  - Added detailed instructions for indexing local PDF files
  - Clear step-by-step guide for adding PDFs to bookmarks
  - Updated README with local PDF workflow

### Fixed

- **Chrome extension PDF detection**
  - Improved detection for local PDF files (file:/// protocol)
  - Better handling of various PDF URL patterns
  - Fixed edge cases in PDF identification logic

## [v0.5.0] - 2025-09-05

### Added

- **Client-side PDF processing** using Chrome extension Offscreen API
  - PDF.js integration for full PDF text extraction in browser
  - Offscreen document to bypass Service Worker limitations
  - Complete PDF content indexing without server processing
  - Support for multi-page PDF text extraction

### Changed

- **Simplified PDF architecture**
  - Moved PDF processing from Rust server to Chrome extension
  - Chrome extension extracts PDF text and sends as regular content
  - Removed server-side PDF processing complexity
  - Unified content handling for all bookmark types

### Removed

- **Server-side PDF processing code**
  - Removed `pdf_processor` module and dependencies
  - Removed PDF-specific fields from `FlatBookmark` struct
  - Removed `pdf-extract`, `sha2`, and `tempfile` dependencies
  - Cleaned up PDF special handling in native messaging host

### Fixed

- **PDF content search functionality**
  - PDFs now properly indexed with full text content
  - Search works across all PDF pages (previously metadata only)
  - Resolved timeout issues with large PDF files
  - Service Worker compatibility issues with PDF.js resolved

### Improved

- Code simplicity and maintainability
- Reduced server-side complexity
- Better error handling for PDF processing
- Consistent content processing workflow

## [v0.4.5] - 2025-09-01

### Changed

- **Token consumption optimization**
  - Removed duplicate `content` field from `SearchResult` struct
  - Reduced token usage by ~50-100 tokens per search result
  - Approximately 1000-2000 tokens saved for 20 search results
  - Search accuracy maintained while improving efficiency

### Improved

- Documentation and project maintenance
  - Simplified improvement proposals document
  - Updated SOW documentation for token optimization

## [v0.4.4] - 2025-8-28

### Fixed

- Chrome extension parallel indexing progress tracking
  - Added proper progress callbacks to track indexing status
  - Fixed metrics counting for success and error cases
  - Improved progress reporting to popup UI
- Removed non-functional `list_bookmark_folders` method
  - Method didn't work with INDEX_NAME configuration
  - Simplified API by removing unused functionality
- Fixed Japanese search test failures
  - Restored `reload()` method for index updates
  - Ensured proper index refreshing after content updates

### Changed

- Code cleanup and optimization
  - Removed dead code warnings
  - Fixed `dropping_references` warning in native messaging host
  - Fixed `vec_init_then_push` clippy warning
  - Partially removed unused constants and methods while preserving Chrome extension dependencies

### Improved

- Test reliability with proper index reloading
- Build warning reduction for cleaner compilation

## [v0.4.3] - 2025-08-26

### Added

- Parallel indexing for Chrome extension
  - 5-10x faster indexing with concurrent processing
  - Batch processing with configurable buffer size
  - Progress tracking for large bookmark collections
  - Native messaging protocol optimizations

### Changed

- Improved memory management during batch indexing
- Optimized buffer commit strategy for better performance

### Fixed

- Native messaging batch state management
- Duplicate index detection in parallel processing

## [v0.4.2] - 2025-08-26

### Added

- Incremental index updates for Chrome extension
  - Skip unchanged bookmarks during re-indexing
  - Metadata tracking with content hashing
  - check_for_updates and sync_bookmarks native messaging methods
  - Reduced re-indexing time from ~50 seconds to ~1 second for unchanged bookmarks
- Extension icon support with multiple sizes (16x16, 32x32, 64x64, 128x128)

### Changed

- Chrome extension now uses incremental mode by default
- Improved indexing performance for large bookmark collections

### Fixed

- Metadata saving for small bookmark collections (5 or fewer bookmarks)

## [v0.4.1] - 2025-08-25

### Added

- Multi-index search support with comma-separated INDEX_NAME
- MultiIndexSearchManager for parallel searching across multiple indices
- URL deduplication and score-based result merging
- Documentation updates for multi-index search feature
- Phrase search support with quoted queries
  - Exact phrase matching with quotes (e.g., "React hooks")
  - Mixed phrase and word queries
  - Support for special characters and Japanese text in phrases
- QueryParser module for parsing search queries with phrase detection

## [v0.4.0] - 2025-08-19

### Added

- Japanese language support with Lindera tokenizer
- Multi-language search capabilities
- Improved tokenization for CJK (Chinese, Japanese, Korean) text

### Changed

- Enhanced search architecture with language-specific tokenizers
- Restructured search module for better modularity
- Updated indexing to handle multiple languages

### Fixed

- Japanese text search and indexing
- Tokenizer registration in native messaging host

## [v0.3.0] - 2025-08-18

### Changed

- Simplified to use only INDEX_NAME environment variable
- Removed profile/folder-based configuration
- Improved test coverage and fixed all failing tests
- Updated documentation to reflect current implementation

### Fixed

- Test failures in search and content modules
- Context type detection for scored snippets
- Title boosting in search results
- Integration test configuration

### Removed

- Unused MCP tools from documentation
- Legacy profile-based bookmark reading

## [v0.2.0] - 2025-08-17

### Added

- Chrome extension for bookmark indexing
- Native messaging host for browser integration
- Installation script (install.sh)
- Custom index naming support

### Changed

- Simplified MCP server architecture
- Improved search relevance with boosting

## [v0.1.0] - 2025-08-01

### Added

- Initial release
- Basic bookmark search functionality
- Tantivy full-text search engine integration
- MCP server implementation
