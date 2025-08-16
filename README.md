# Rust Sci-Hub MCP Server

A Rust-based Model Context Protocol (MCP) server that provides search, download, and metadata extraction capabilities for academic papers through Sci-Hub integration.

## Overview

This project implements an MCP server that enables AI assistants (like Claude) to:
- Search for academic papers by DOI, title, or author
- Download papers from Sci-Hub mirrors 
- Extract bibliographic metadata from downloaded papers
- Run as a background service on macOS

## Features

- **Robust Sci-Hub Integration**: Automatic mirror discovery and failover
- **MCP Protocol Support**: Compatible with Claude Desktop and other MCP clients
- **Background Service**: Runs as macOS LaunchAgent with automatic startup
- **Rate Limiting**: Respectful request patterns to avoid overwhelming servers
- **Error Resilience**: Circuit breakers, retries, and graceful degradation
- **Security First**: Input validation, secure HTTP, minimal permissions

## Installation

### Prerequisites

- Rust 1.70+ (latest stable recommended)
- macOS 10.15+ (for LaunchAgent integration)

### Building from Source

```bash
git clone <repository-url>
cd rust-sci-hub-mcp
cargo build --release
```

### Running

```bash
# Development mode
cargo run

# With verbose logging
cargo run -- --verbose

# With custom config
cargo run -- --config /path/to/config.toml

# As daemon (when implemented)
cargo run -- --daemon
```

## Development

### Setup

```bash
# Install development tools
rustup component add clippy rustfmt
cargo install cargo-audit cargo-tarpaulin

# Run tests
cargo test

# Run lints
cargo clippy -- -D warnings
cargo fmt --check

# Security audit
cargo audit
```

### Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration

# With coverage
cargo tarpaulin --out html
```

### Code Quality

This project enforces strict code quality standards:
- All code must pass Clippy with zero warnings
- Code coverage should be >90%
- All public APIs must be documented
- Security audit must pass

## Configuration

Configuration is loaded from:
1. Command-line arguments (highest priority)
2. Environment variables  
3. `~/.config/rust-sci-hub-mcp/config.toml`
4. Built-in defaults (lowest priority)

Example configuration:

```toml
[server]
port = 8080
host = "127.0.0.1" 
timeout_secs = 30

[sci_hub]
mirrors = [
    "https://sci-hub.se",
    "https://sci-hub.st", 
    "https://sci-hub.ru"
]
rate_limit_per_sec = 1
timeout_secs = 30
max_retries = 3

[downloads]
directory = "~/Downloads/papers"
max_concurrent = 3
max_file_size_mb = 100

[logging]
level = "info"
format = "json"
```

## Architecture

```
rust-sci-hub-mcp/
├── src/
│   ├── main.rs              # Entry point and CLI
│   ├── lib.rs               # Library exports
│   ├── server/              # MCP server implementation
│   ├── tools/               # MCP tools (search, download, metadata)
│   ├── client/              # Sci-Hub client with mirror management
│   ├── config/              # Configuration management
│   ├── service/             # Background service logic
│   └── error.rs             # Error types
├── tests/                   # Integration tests
├── benches/                 # Performance benchmarks
└── docs/                    # Documentation
```

## License

MIT License - see LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Ensure all tests pass and code follows style guidelines
4. Submit a pull request

## Disclaimer

This tool is designed for personal research use only. Users are responsible for ensuring their use complies with local laws and institutional policies regarding academic paper access.