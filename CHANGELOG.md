# Changelog

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
