# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2025-08-17

### Added
- Multi-provider search system with plugin architecture
- arXiv provider for computer science and physics papers
- CrossRef provider for DOI and metadata search
- Configurable download directory via CLI and environment variables
- User-friendly error messages with helpful suggestions
- Graceful failure handling that doesn't interrupt Claude's workflow
- Enhanced search results with source, year, and DOI information
- E2E test suites for MCP protocol compliance

### Changed
- Simplified JSON schemas for better Claude Desktop compatibility
- Improved error handling to return informative messages instead of failures
- Updated search results format to be more informative
- Refactored from single Sci-Hub source to multi-provider architecture

### Fixed
- Claude Desktop tool invocation issues with complex schemas
- Download failures now provide helpful alternatives
- MCP protocol compliance for error responses

## [0.2.0] - 2025-08-16

### Added
- Model Context Protocol (MCP) server implementation
- Integration with Claude Desktop
- search_papers tool for academic paper discovery
- download_paper tool for PDF retrieval
- extract_metadata tool for bibliographic information
- Resilience features (circuit breakers, retries)
- Comprehensive test coverage

### Changed
- Migrated from standalone CLI to MCP server architecture
- Updated to use rmcp SDK for MCP protocol support

## [0.1.0] - 2025-08-15

### Added
- Initial release
- Basic Sci-Hub integration
- Command-line interface
- PDF download functionality
- Mirror discovery and failover
- Rate limiting for respectful access

---

For a complete list of changes, see the [commit history](https://github.com/yourusername/rust-research-mcp/commits/main).