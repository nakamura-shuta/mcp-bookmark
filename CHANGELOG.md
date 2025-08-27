# Changelog

## [Unreleased]

### Fixed

- Chrome extension parallel indexing progress tracking
  - Added proper progress callbacks to track indexing status
  - Fixed metrics counting for success and error cases
  - Improved progress reporting to popup UI

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
