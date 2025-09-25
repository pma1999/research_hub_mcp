# knowledge_accumulator_mcp

A Model Context Protocol (MCP) server that helps accumulate and organize academic knowledge through intelligent paper search, retrieval, and categorization.

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green)](https://modelcontextprotocol.io)
[![Crates.io](https://img.shields.io/crates/v/knowledge_accumulator_mcp.svg)](https://crates.io/crates/knowledge_accumulator_mcp)
[![CI](https://github.com/Ladvien/knowledge_accumulator_mcp/workflows/CI/badge.svg)](https://github.com/Ladvien/knowledge_accumulator_mcp/actions)
[![Coverage](https://codecov.io/gh/Ladvien/knowledge_accumulator_mcp/branch/main/graph/badge.svg)](https://codecov.io/gh/Ladvien/knowledge_accumulator_mcp)
[![Security Audit](https://github.com/Ladvien/knowledge_accumulator_mcp/workflows/Security%20Audit/badge.svg)](https://github.com/Ladvien/knowledge_accumulator_mcp/actions)
[![MSRV](https://img.shields.io/badge/MSRV-1.70.0-blue)](https://github.com/rust-lang/rust/releases/tag/1.70.0)

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

- 🔍 **Multi-Provider Search**: Comprehensive search across 12+ academic sources:
  - **CrossRef** - Authoritative metadata for 130M+ papers
  - **Semantic Scholar** - AI-powered search with PDF access
  - **arXiv** - Physics, CS, and math preprints
  - **PubMed Central** - Biomedical and life science papers
  - **OpenReview** - ML conference papers (NeurIPS, ICLR, etc.)
  - **CORE** - 350M+ open access papers
  - **Unpaywall** - Legal free PDF discovery
  - **SSRN** - Social science working papers
  - **bioRxiv** - Biology preprints
  - **MDPI** - Open access journals
  - **ResearchGate** - Academic social network (ethical access)
  - **Sci-Hub** - Full-text fallback (lowest priority)

- 🧠 **Intelligent Routing**: Smart provider prioritization based on:
  - Academic domain detection (CS/ML, biomedical, physics, social sciences)
  - Search type optimization (DOI, author, title, keywords)
  - Content availability (PDF access, recent papers, open access)
  - Temporal relevance (recent vs. historical content)

- 📥 **Robust Downloads**: Multi-provider fallback with zero-byte protection
- ⚡ **Batch Processing**: Parallel downloads, metadata extraction, and bibliography generation
  - **Download multiple papers**: Up to 9 concurrent downloads (5-10x faster)
  - **Batch metadata extraction**: Process 12 PDFs simultaneously (4-8x faster)
  - **Parallel citation generation**: 30 concurrent metadata fetches (10-20x faster)
- 📊 **Metadata Extraction**: Extract bibliographic information from PDFs
- 🔍 **Code Pattern Search**: Regex-powered search for algorithm implementations in papers
- 📚 **Bibliography Generation**: Multi-format citations (BibTeX, APA, MLA, Chicago, IEEE, Harvard)
- 🤖 **MCP Integration**: Enhanced for Claude Desktop and Claude Code workflows
- ⚡ **High Performance**: Built with Rust for speed and reliability
- 🔄 **Resilient**: Circuit breakers, automatic retries, and graceful error handling

## Installation
****
### Quick Start (Recommended)

**Download the latest release binary:**

```bash
# macOS (Apple Silicon)
curl -L -o knowledge_accumulator_mcp https://github.com/Ladvien/knowledge_accumulator_mcp/releases/latest/download/knowledge_accumulator_mcp
chmod +x knowledge_accumulator_mcp

# Move to a permanent location
sudo mv knowledge_accumulator_mcp /usr/local/bin/
```

### Building from Source

If you prefer to build from source:

```bash
# Prerequisites: Rust 1.70+ (install from https://rustup.rs/)

# Clone the repository
git clone https://github.com/Ladvien/knowledge_accumulator_mcp.git
cd knowledge_accumulator_mcp

# Build the release binary
cargo build --release

# The binary will be at ./target/release/knowledge_accumulator_mcp
```

### Configuration for Claude Desktop

Add the following to your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
**Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "knowledge_accumulator_mcp": {
      "command": "/opt/homebrew/bin/knowledge_accumulator_mcp",
      "args": [
        "--download-dir", "~/downloads/research_papers",
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
knowledge_accumulator_mcp [OPTIONS]

Options:
  --download-dir <PATH>    Directory for downloaded papers [default: ~/downloads/papers]
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

### Core Research Tools

#### search_papers
Search for academic papers across multiple sources.

**Parameters:**
- `query` (required): Search query (DOI, title, or author)
- `limit` (optional): Maximum results to return (default: 10)

#### download_paper
Download a paper PDF by DOI.

**Parameters:**
- `doi` (required): The DOI of the paper to download
- `filename` (optional): Custom filename for the downloaded PDF

#### download_papers_batch ✨ NEW!
Download multiple papers concurrently for significantly faster batch downloads.

**Parameters:**
- `papers` (required): Array of download requests, each containing:
  - `doi` (optional): DOI of the paper to download
  - `url` (optional): Direct download URL (alternative to DOI)
  - `filename` (optional): Custom filename for this specific paper
  - `category` (optional): Organization category for this paper
- `max_concurrent` (optional): Maximum concurrent downloads (default: 9, max: 20)
- `continue_on_error` (optional): Continue downloading if some papers fail (default: true)
- `shared_settings` (optional): Common settings for all downloads:
  - `directory` (optional): Target directory for all downloads
  - `category` (optional): Default category for organizing downloads
  - `overwrite` (optional): Whether to overwrite existing files (default: false)
  - `verify_integrity` (optional): Verify file integrity after download (default: true)

**Example Usage:**
```
Ask Claude: "Download these papers in parallel: [{'doi': '10.1038/nature12373'}, {'doi': '10.1126/science.1259855'}]"
```

**Performance:** Up to 5-10x faster than downloading papers individually!

#### extract_metadata
Extract bibliographic metadata from a PDF file.

**Parameters:**
- `file_path` (required): Path to the PDF file
- `batch_files` (optional): Array of file paths for batch processing (processes up to 12 files concurrently)

**Example Usage:**
```
Single file: "Extract metadata from paper.pdf"
Batch processing: "Extract metadata from these files: ['paper1.pdf', 'paper2.pdf', 'paper3.pdf']"
```

**Performance:** Up to 4-8x faster when processing multiple PDFs concurrently!

### Claude Code Enhanced Tools

#### search_code
Search for code patterns within downloaded research papers using regex.

**Parameters:**
- `pattern` (required): Regex pattern to search for
- `language` (optional): Programming language filter (python, javascript, rust, etc.)
- `search_dir` (optional): Directory to search in (defaults to download directory)
- `limit` (optional): Maximum results (default: 20)
- `context_lines` (optional): Lines of context around matches (default: 3)

**Example Usage:**
```
Ask Claude: "Search for 'def train_model' in my downloaded papers"
```

#### generate_bibliography
Generate citations and bibliography from paper DOIs in various formats with parallel metadata fetching.

**Parameters:**
- `identifiers` (required): Array of DOIs or paper identifiers
- `format` (optional): Citation format - `bibtex`, `apa`, `mla`, `chicago`, `ieee`, `harvard` (default: bibtex)
- `include_abstract` (optional): Include abstract in citation (default: false)
- `include_keywords` (optional): Include keywords in citation (default: false)

**Example Usage:**
```
Ask Claude: "Generate a BibTeX bibliography for these DOIs: ['10.1038/nature12373', '10.1126/science.1259855']"
```

**Performance:** Up to 10-20x faster for large reference lists with 30 concurrent metadata fetches!

## Claude Code Integration

This MCP server is specifically enhanced for **Claude Code** workflows:

### Research-Driven Development
- **Code Pattern Discovery**: Find algorithm implementations in research papers
- **Citation Management**: Generate properly formatted references for your projects
- **Research Documentation**: Extract and organize findings from academic sources

### Common Workflows

#### 1. Algorithm Research
```bash
# Search for papers on a topic
"Find recent papers on transformer architectures"

# Download relevant papers
"Download the first 3 papers from the search results"

# Search for implementation patterns
"Search for 'class Transformer' in downloaded papers"
```

#### 2. Literature Review
```bash
# Collect papers on a research area
"Search for papers by Yoshua Bengio on deep learning"

# Generate bibliography
"Create a BibTeX bibliography from the downloaded paper DOIs"

# Extract key concepts
"Search for 'attention mechanism' implementations"
```

#### 3. Code Documentation
```bash
# Find reference implementations
"Search for 'def attention' in my research papers"

# Generate proper citations
"Create IEEE format citations for papers containing this algorithm"
```

### Integration Tips

1. **Set up downloads directory**: Configure a dedicated research papers directory
2. **Use regex patterns**: Leverage powerful pattern matching for code discovery  
3. **Batch operations**: Process multiple papers efficiently with the bibliography tool
4. **Context awareness**: Use context lines to understand code snippets better

## Configuration File

Create a configuration file at `~/.config/knowledge_accumulator_mcp/config.toml`:

```toml
[server]
port = 8080
host = "127.0.0.1"

[research_source]
# Timeout per provider when searching across multiple sources (default: 30 seconds)
provider_timeout_secs = 30

[downloads]
directory = "~/downloads/research_papers"
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
cargo nextest run

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

## Changelog

### Version 0.4.0 (Latest)
- **🚀 Major Multi-Provider Enhancement**: Added 8 new academic sources
  - PubMed Central for biomedical papers with NCBI E-utilities API
  - OpenReview for ML conference papers (NeurIPS, ICLR, ICML)
  - MDPI for open access journals with HTML parsing
  - ResearchGate with ethical ToS-compliant access
- **🧠 Intelligent Priority Ordering**: Context-aware provider selection
  - Domain-specific routing (CS/ML → arXiv/OpenReview, biomedical → PubMed)
  - Search type optimization (DOI → CrossRef, author → Semantic Scholar)
  - Content availability boosting (PDF access, recent papers)
  - Temporal relevance adjustments
- **🔧 Enhanced Testing**: Fixed all integration and security tests
- **📊 Improved Coverage**: 12+ providers with specialized capabilities

### Version 0.3.0
- Critical fix for zero-byte file creation on failed downloads
- Multi-provider academic search with comprehensive testing
- Complete repository cleanup and Sci-Hub provider overhaul

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
- **macOS**: `~/Library/Logs/Claude/mcp-server-knowledge_accumulator_mcp.log`
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

For issues, questions, or suggestions, please [open an issue](https://github.com/yourusername/knowledge_accumulator_mcp/issues) on GitHub.

---

Made with ❤️ for the academic community. Please use responsibly.