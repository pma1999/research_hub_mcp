# Rust Research MCP Installation Guide

## Quick Installation

### Option 1: Automated Installation (Recommended)
```bash
curl -sSL https://raw.githubusercontent.com/Ladvien/research_hub_mcp/main/install.sh | bash
```

### Option 2: Manual Installation
```bash
# 1. Install from GitHub
cargo install --git https://github.com/Ladvien/research_hub_mcp.git

# 2. Create directories
mkdir -p ~/Documents/Research-Papers
mkdir -p ~/.cache/rust-research-mcp

# 3. Add to Claude Desktop config (see below)
```

## Claude Desktop Configuration

Add this to your `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "rust-research-mcp": {
      "command": "/Users/[YOUR_USERNAME]/.cargo/bin/rust-research-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info",
        "RSH_DOWNLOAD_DIRECTORY": "/Users/[YOUR_USERNAME]/Documents/Research-Papers",
        "RSH_CACHE_DIRECTORY": "/Users/[YOUR_USERNAME]/.cache/rust-research-mcp"
      }
    }
  }
}
```

Replace `[YOUR_USERNAME]` with your actual username.

## What's Included

- **rust-research-mcp.dxt**: Distribution metadata file
- **install.sh**: Automated installation script
- **INSTALL.md**: This installation guide

## Features

✅ **Multi-Provider Search**: Search across 12+ academic databases
✅ **Automated Downloads**: PDF download with integrity verification
✅ **Metadata Extraction**: Bibliographic data and citations
✅ **Circuit Breaker Pattern**: Resilient API calls with automatic retry
✅ **Rate Limiting**: Respectful scraping with automatic backoff
✅ **Mirror Failover**: Automatic Sci-Hub mirror switching
✅ **Organization**: Categorization of downloaded papers
✅ **Progress Tracking**: Real-time download progress
✅ **Hash Verification**: SHA256 integrity checking
✅ **Configurable**: Customizable download directories

## Supported Providers

- **Sci-Hub** - Primary paper access
- **ArXiv** - Preprints and open access
- **CrossRef** - DOI resolution and metadata
- **PubMed Central** - Biomedical papers
- **Semantic Scholar** - AI-powered search
- **Unpaywall** - Open access discovery
- **CORE** - Academic repository aggregator
- **SSRN** - Social sciences research
- **OpenReview** - Peer review platform
- **bioRxiv** - Biology preprints
- **MDPI** - Open access publisher
- **ResearchGate** - Academic social network

## Usage Examples

### Search for Papers
```
"Search for papers about machine learning neural networks"
```

### Download by DOI
```
"Download paper with DOI 10.1038/nature12373"
```

### Get Metadata
```
"Get metadata for DOI 10.1038/nature12373"
```

## System Requirements

- **Rust**: 1.70.0 or higher
- **Platform**: macOS, Linux, or Windows
- **Claude Desktop**: Latest version
- **Network**: Internet connection for paper access

## Troubleshooting

### Common Issues

**Installation fails:**
```bash
# Update Rust
rustup update

# Clear cargo cache
cargo clean
```

**Downloads fail with HTTP errors:**
- Fixed in v0.6.6 - automatic protocol negotiation
- Wait and retry - temporary server issues

**Rate limiting:**
- Wait a few minutes between requests
- System has automatic retry with backoff

**Papers not found:**
- Try different search terms
- Check DOI format (e.g., "10.1038/nature12373")
- Try alternative providers

### Logs and Debugging

Check logs at:
```
~/Library/Logs/Claude/mcp-server-rust-research-mcp.log
```

Enable debug logging:
```json
"env": {
  "RUST_LOG": "debug"
}
```

## Support

- **Repository**: https://github.com/Ladvien/research_hub_mcp
- **Issues**: https://github.com/Ladvien/research_hub_mcp/issues
- **License**: GPL-3.0

## Version History

### v0.6.6 (Latest)
- ✅ Fixed HTTP2 connection errors
- ✅ Enhanced provider reliability
- ✅ Improved error handling
- ✅ Better user agent handling

### v0.6.5
- Added circuit breaker pattern
- Multi-provider support
- Enhanced metadata extraction
- Improved error handling

---

**Happy researching! 🔬📚**