# Changelog

## [v0.3.0] - 2025-01-18

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

## [v0.2.0] - 2025-01-17

### Added
- Chrome extension for bookmark indexing
- Native messaging host for browser integration
- Installation script (install.sh)
- Custom index naming support

### Changed
- Simplified MCP server architecture
- Improved search relevance with boosting

## [v0.1.0] - 2025-01-01

### Added
- Initial release
- Basic bookmark search functionality
- Tantivy full-text search engine integration
- MCP server implementation
