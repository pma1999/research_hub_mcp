# rust-research-mcp

A Model Context Protocol (MCP) server that provides AI assistants with academic paper search and retrieval capabilities through multiple research sources.

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green)](https://modelcontextprotocol.io)

## ⚠️ Legal Disclaimer

**IMPORTANT: This tool is intended for personal academic use only.**

This software is provided for educational and research purposes. Users are responsible for ensuring their use complies with:
- All applicable laws and regulations
- Publisher terms of service
- Institutional policies
- Copyright restrictions

The developers of this tool do not condone or support any illegal activities. Users should:
- Only access papers they have legal rights to access
- Respect intellectual property rights
- Use retrieved materials in accordance with fair use principles
- Consider supporting authors and publishers through legitimate channels

**By using this software, you acknowledge that you understand and will comply with all applicable laws and regulations regarding access to academic content.**

## Features

- 🔍 **Multi-Source Search**: Searches across multiple academic databases including arXiv, CrossRef, and Sci-Hub
- 📥 **Smart Downloads**: Automated paper retrieval with configurable download directory
- 📊 **Metadata Extraction**: Extract bibliographic information from PDFs
- 🤖 **MCP Integration**: Seamlessly works with Claude Desktop and other MCP-compatible AI assistants
- ⚡ **High Performance**: Built with Rust for speed and reliability
- 🔄 **Resilient**: Automatic retries, fallback mirrors, and graceful error handling

## Installation

### Quick Start (Recommended)

**Download the latest release binary:**

```bash
# macOS (Apple Silicon)
curl -L -o rust-research-mcp https://github.com/Ladvien/research_hub_mcp/releases/latest/download/rust-research-mcp
chmod +x rust-research-mcp

# Move to a permanent location
sudo mv rust-research-mcp /usr/local/bin/
```

### Building from Source

If you prefer to build from source:

```bash
# Prerequisites: Rust 1.70+ (install from https://rustup.rs/)

# Clone the repository
git clone https://github.com/Ladvien/research_hub_mcp.git
cd research_hub_mcp

# Build the release binary
cargo build --release

# The binary will be at ./target/release/rust-research-mcp
```

### Configuration for Claude Desktop

Add the following to your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
**Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "rust-research-mcp": {
      "command": "/usr/local/bin/rust-research-mcp",
      "args": [
        "--download-dir", "~/Downloads/Research-Papers",
        "--log-level", "info"
      ],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Usage

Once configured, you can ask Claude to:

- **Search for papers**: "Search for recent papers on quantum computing"
- **Download papers**: "Download the first paper from the search results"
- **Extract metadata**: "Extract metadata from the PDF file"

### Command Line Options

```bash
rust-research-mcp [OPTIONS]

Options:
  --download-dir <PATH>    Directory for downloaded papers [default: ~/Downloads/papers]
  --log-level <LEVEL>      Log level (error, warn, info, debug, trace) [default: info]
  --config <PATH>          Path to configuration file
  --help                   Print help information
  --version                Print version information
```

### Environment Variables

- `RSH_DOWNLOAD_DIRECTORY`: Override download directory
- `RSH_LOG_LEVEL`: Override log level
- `RUST_LOG`: Standard Rust logging configuration

## Available Tools

### search_papers
Search for academic papers across multiple sources.

**Parameters:**
- `query` (required): Search query (DOI, title, or author)
- `limit` (optional): Maximum results to return (default: 10)

### download_paper
Download a paper PDF by DOI.

**Parameters:**
- `doi` (required): The DOI of the paper to download
- `filename` (optional): Custom filename for the downloaded PDF

### extract_metadata
Extract bibliographic metadata from a PDF file.

**Parameters:**
- `file_path` (required): Path to the PDF file

## Configuration File

Create a configuration file at `~/.config/rust-research-mcp/config.toml`:

```toml
[server]
port = 8080
host = "127.0.0.1"

[downloads]
directory = "~/Downloads/Research-Papers"
max_concurrent = 3
max_file_size_mb = 100

[logging]
level = "info"
format = "json"
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Run benchmarks
cargo bench
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Security audit
cargo audit
```

## Architecture

The project follows a modular architecture:

```
src/
├── server/          # MCP server implementation
├── tools/           # MCP tools (search, download, metadata)
├── client/          # Research source clients
│   └── providers/   # Source-specific implementations
├── resilience/      # Error handling and retry logic
└── config/          # Configuration management
```

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Troubleshooting

### Common Issues

**Issue**: Papers not downloading
- **Solution**: Some papers may not be available through Sci-Hub, especially very recent publications. The tool will provide helpful suggestions for alternatives.

**Issue**: Connection errors
- **Solution**: Check your internet connection and firewall settings. The tool requires access to academic databases.

**Issue**: Claude Desktop not recognizing the server
- **Solution**: Ensure the path in `claude_desktop_config.json` is absolute and the binary has execute permissions.

### Logs

Logs are available at:
- **macOS**: `~/Library/Logs/Claude/mcp-server-rust-research-mcp.log`
- **Linux**: `~/.local/share/Claude/logs/`
- **Windows**: `%APPDATA%\Claude\logs\`

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [rmcp](https://github.com/anthropics/rmcp) - Rust SDK for Model Context Protocol
- Uses the [Model Context Protocol](https://modelcontextprotocol.io) specification
- Searches academic databases including [arXiv](https://arxiv.org) and [CrossRef](https://www.crossref.org)

## Disclaimer

This tool is provided "as is" without warranty of any kind. The authors and contributors are not responsible for any misuse or legal issues arising from the use of this software. Users must ensure they comply with all applicable laws, regulations, and terms of service when accessing academic content.

**For personal academic use only.**

## Support

For issues, questions, or suggestions, please [open an issue](https://github.com/yourusername/rust-research-mcp/issues) on GitHub.

---

Made with ❤️ for the academic community. Please use responsibly.