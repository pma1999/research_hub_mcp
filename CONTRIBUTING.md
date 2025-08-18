# Contributing to rust-research-mcp

Thank you for your interest in contributing to rust-research-mcp! This project provides academic paper search and retrieval capabilities through a Model Context Protocol (MCP) server.

## 🚀 Quick Start

1. **Fork and Clone**
   ```bash
   git clone https://github.com/YOUR_USERNAME/research_hub_mcp.git
   cd research_hub_mcp
   ```

2. **Setup Development Environment**
   ```bash
   # Install Rust (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install development tools
   cargo install cargo-tarpaulin cargo-audit
   rustup component add clippy rustfmt
   ```

3. **Verify Setup**
   ```bash
   cargo build
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

## 📋 Development Workflow

### Code Style & Quality

**Before submitting any PR, ensure:**

```bash
# Format code
cargo fmt

# Check for linting issues
cargo clippy -- -D warnings

# Run all tests
cargo test

# Security audit
cargo audit

# Check test coverage (aim for >85%)
cargo tarpaulin --out html
```

## Code of Conduct

Please note that this project adheres to a Code of Conduct. By participating, you are expected to uphold this code.

## Legal and Ethical Guidelines

**Important**: This project is intended for personal academic use only. All contributions must:

- Respect intellectual property rights
- Comply with applicable laws and regulations
- Support legitimate academic research
- Not facilitate or encourage illegal activities

## How to Contribute

### Reporting Issues

1. Check if the issue already exists
2. Create a new issue with a clear title and description
3. Include steps to reproduce (if applicable)
4. Provide system information (OS, Rust version, etc.)

### Submitting Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass (`cargo test`)
6. Format your code (`cargo fmt`)
7. Check for linting issues (`cargo clippy`)
8. Commit with a clear message
9. Push to your fork
10. Open a Pull Request

### Development Setup

```bash
# Clone your fork
git clone https://github.com/yourusername/rust-research-mcp.git
cd rust-research-mcp

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the project
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use meaningful variable and function names
- Add comments for complex logic
- Write unit tests for new functions
- Document public APIs with rustdoc comments

### Testing

- Write tests for all new functionality
- Ensure existing tests pass
- Add integration tests for new tools
- Test error cases and edge conditions

### Documentation

- Update README.md if adding new features
- Add rustdoc comments for public functions
- Include examples in documentation
- Update CHANGELOG.md for notable changes

## Areas for Contribution

### High Priority

- Additional academic source providers
- Improved error handling and recovery
- Performance optimizations
- Cross-platform compatibility improvements

### Feature Ideas

- Batch download capabilities
- Citation format generation
- Advanced search filters
- Paper recommendation system
- Local paper database management

### Documentation

- Usage examples and tutorials
- API documentation improvements
- Configuration guides
- Troubleshooting guides

## Review Process

1. All PRs require at least one review
2. CI tests must pass
3. Code must be formatted and linted
4. Documentation must be updated
5. Changes must align with project goals

## Questions?

Feel free to open an issue for any questions about contributing.

Thank you for helping make rust-research-mcp better!