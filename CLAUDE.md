# CLAUDE.md - AI Assistant Project Guide

## Project Overview

**rust-sci-hub-mcp** is a Rust-based Model Context Protocol (MCP) server that provides Claude and other AI assistants with the ability to search, download, and extract metadata from academic papers through Sci-Hub integration. This project is designed for personal research use in computer science architecture and research.

## Project Goals

- **Primary**: Enable AI assistants to autonomously search and retrieve academic papers for research
- **Secondary**: Provide structured metadata extraction for citation and reference management
- **Tertiary**: Run as a background service on macOS with minimal user intervention

## Architecture Decisions

### Technology Stack
- **Language**: Rust (stable, 1.70+ minimum)
- **MCP Framework**: `rmcp` (official Anthropic SDK) - chosen for production stability and official support
- **Async Runtime**: Tokio - industry standard for Rust async operations
- **HTTP Client**: reqwest - robust HTTP client with connection pooling
- **Configuration**: layered config (file → env → CLI) using serde and clap
- **Logging**: tracing with structured logging for debugging and monitoring

### Key Design Principles
1. **Resilience First**: Circuit breakers, retries, and graceful degradation
2. **Security by Default**: Input validation, secure HTTP, minimal permissions
3. **Observability**: Comprehensive logging and health checks
4. **macOS Native**: LaunchAgent integration, Homebrew distribution
5. **Developer Experience**: Comprehensive tooling (Clippy, rustfmt, tests)

## Directory Structure

```
rust-sci-hub-mcp/
├── src/
│   ├── main.rs              # Entry point and CLI handling
│   ├── lib.rs               # Library exports
│   ├── server/              # MCP server implementation
│   ├── tools/               # MCP tools (search, download, metadata)
│   ├── client/              # Sci-Hub client with mirror management
│   ├── config/              # Configuration management
│   ├── service/             # Background service and daemon logic
│   └── error.rs             # Error types and handling
├── tests/                   # Integration tests
├── benches/                 # Performance benchmarks
├── docs/                    # Documentation
├── scripts/                 # Installation and management scripts
├── homebrew/                # Homebrew formula
└── launchd/                # macOS LaunchAgent configuration
```

## Core Components

### 1. MCP Server (`src/server/`)
- Implements rmcp ServerHandler trait
- Manages tool registration and lifecycle
- Handles stdio transport for Claude Desktop integration
- Provides health checks and graceful shutdown

### 2. Tools (`src/tools/`)
- **Search Tool**: Query papers by DOI, title, author
- **Download Tool**: Retrieve papers with progress tracking
- **Metadata Tool**: Extract bibliographic information

### 3. Sci-Hub Client (`src/client/`)
- Mirror discovery and health checking
- Automatic failover and circuit breaking
- Rate limiting and respectful scraping
- HTTP client with retry logic

### 4. Service Management (`src/service/`)
- Background daemon functionality
- Process supervision and restart
- Signal handling for graceful shutdown
- Integration with system logging

## Development Guidelines for AI Assistants

### Code Quality Standards
- **Always run Clippy**: `cargo clippy -- -D warnings`
- **Format before commit**: `cargo fmt`
- **Test coverage**: Aim for >90% with `cargo tarpaulin`
- **Security audit**: Run `cargo audit` regularly
- **Documentation**: All public APIs must have rustdoc comments

### Error Handling Philosophy
- Use `thiserror` for structured error types
- Chain errors to preserve context
- Log errors at appropriate levels (ERROR for actionable, WARN for recoverable)
- Implement retries with exponential backoff for transient failures
- Use circuit breakers for external service calls

### Async Programming Guidelines
- Prefer async/await over manual Future implementations
- Use `tokio::timeout` for all external calls
- Handle cancellation gracefully with `tokio::select!`
- Use channels for inter-task communication
- Avoid blocking operations in async contexts

### Testing Strategy
- **Unit Tests**: Fast, isolated, test single functions
- **Integration Tests**: Test component interactions
- **Property Tests**: Use `proptest` for algorithm verification
- **Performance Tests**: Use `criterion` for benchmarking
- **Security Tests**: Validate input handling and access controls

## Working with Jira Stories

### Story Dependencies
Stories must be completed in dependency order:
1. **RSH-2**: Project setup (foundation for everything)
2. **RSH-3, RSH-4**: Core server and configuration
3. **RSH-5**: Sci-Hub client (required for tools)
4. **RSH-6, RSH-7, RSH-8**: Tools implementation
5. **RSH-9, RSH-10, RSH-11**: Service and distribution
6. **RSH-12, RSH-13, RSH-14, RSH-15**: Quality and security

### Definition of Done Checklist
For every story, ensure:
- [ ] Code compiles without warnings
- [ ] All tests pass (`cargo test`)
- [ ] Clippy passes with zero warnings
- [ ] Documentation is updated
- [ ] Security considerations are addressed
- [ ] Performance impact is measured

### Code Review Focus Areas
1. **Error Handling**: Are all error cases properly handled?
2. **Resource Management**: No memory leaks or resource exhaustion?
3. **Security**: Input validation, secure defaults, minimal permissions?
4. **Performance**: Appropriate async usage, efficient algorithms?
5. **Maintainability**: Clear code, good abstractions, documented decisions?

## Security Considerations

### Critical Security Requirements
- **Input Validation**: All user inputs must be validated and sanitized
- **HTTP Security**: Use HTTPS, verify certificates, secure headers
- **File Permissions**: Configuration files should be 0600 (owner read/write only)
- **Credential Management**: Never log sensitive data, use secure storage
- **Rate Limiting**: Prevent abuse and respect external services
- **Memory Safety**: Leverage Rust's ownership system, avoid unsafe code

### Sci-Hub Integration Ethics
- **Personal Use Only**: This tool is designed for personal research
- **Rate Limiting**: Implement respectful request patterns
- **Mirror Rotation**: Distribute load across available mirrors
- **Error Handling**: Graceful degradation when services are unavailable
- **Documentation**: Clear usage guidelines and limitations

## Configuration Management

### Configuration Sources (in precedence order)
1. Command-line arguments (highest priority)
2. Environment variables
3. Configuration file
4. Built-in defaults (lowest priority)

### Key Configuration Categories
- **Server**: Port, transport, logging level
- **Sci-Hub**: Mirror URLs, timeouts, rate limits
- **Downloads**: Directory, concurrent limits, file organization
- **Service**: Daemon mode, PID file location, restart policy

## Testing Strategy

### Test Categories
- **Unit Tests**: Fast, isolated component tests
- **Integration Tests**: Cross-component workflow tests
- **Performance Tests**: Latency and throughput benchmarks
- **Security Tests**: Input validation and access control verification
- **End-to-End Tests**: Complete user scenario validation

### Mock Strategy
- Use `wiremock` for HTTP service mocking
- Create test fixtures for common scenarios
- Implement deterministic test data
- Use property-based testing for edge cases

## Deployment and Distribution

### macOS Integration
- **LaunchAgent**: Automatically starts on user login
- **Homebrew**: Simple installation via `brew install`
- **Logs**: Accessible via Console.app
- **Service Management**: Use `brew services` commands

### Installation Flow
1. User runs `brew install rust-sci-hub-mcp`
2. Homebrew builds from source and installs binary
3. Post-install script sets up LaunchAgent
4. Service starts automatically
5. Claude Desktop can connect via stdio transport

## Performance Targets

### Response Time Goals
- **Search**: < 500ms for simple queries
- **Download**: Stream large files with progress reporting
- **Metadata**: < 200ms for cached data, < 2s for extraction
- **Health Check**: < 50ms for service status

### Resource Limits
- **Memory**: < 100MB baseline, < 500MB under load
- **CPU**: < 5% idle, burst to 50% during operations
- **Disk**: Configurable download storage limits
- **Network**: Respectful rate limiting (1 req/sec default)

## Monitoring and Observability

### Logging Strategy
- **Structured Logging**: Use tracing with JSON format
- **Log Levels**: DEBUG for development, INFO for operations, WARN/ERROR for issues
- **Context**: Include request IDs, operation types, timing information
- **Security**: Never log sensitive data (credentials, personal info)

### Health Checks
- **Service Health**: Basic liveness check
- **Dependencies**: Sci-Hub mirror availability
- **Resources**: Memory, disk, network connectivity
- **Performance**: Response time percentiles

## Common Development Tasks

### Setting Up Development Environment
```bash
# Clone and setup
git clone <repository>
cd rust-sci-hub-mcp
cargo build

# Install development tools
cargo install cargo-tarpaulin cargo-audit
rustup component add clippy rustfmt

# Run full test suite
cargo test
cargo clippy -- -D warnings
cargo audit
```

### Adding a New Tool
1. Create module in `src/tools/`
2. Implement tool using `#[tool]` macro
3. Add input validation with schemars
4. Register tool in server handler
5. Add comprehensive tests
6. Update documentation

### Debugging Common Issues
- **Connection Issues**: Check LaunchAgent status and logs
- **Mirror Failures**: Verify circuit breaker status and retry logic
- **Performance**: Use `cargo flamegraph` for profiling
- **Memory Leaks**: Use `valgrind` or `heaptrack` for analysis

## Resources and References

### Documentation
- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [rmcp SDK Documentation](https://docs.rs/rmcp/)
- [Tokio Guide](https://tokio.rs/tokio/tutorial)
- [Rust Async Book](https://rust-lang.github.io/async-book/)

### Tools and Libraries
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
- [tracing Documentation](https://docs.rs/tracing/)
- [reqwest Guide](https://docs.rs/reqwest/)
- [serde Tutorial](https://serde.rs/)

## Contributing Guidelines

### For AI Assistants
- Follow the Jira story structure and acceptance criteria
- Implement comprehensive error handling from the start
- Write tests before implementing features (TDD approach)
- Use structured logging with appropriate context
- Ask clarifying questions about requirements before implementation
- Focus on security and performance from day one

### Code Style
- Use `cargo fmt` for consistent formatting
- Follow Rust naming conventions (snake_case, CamelCase)
- Prefer explicit error handling over unwrap/expect
- Use meaningful variable and function names
- Keep functions small and focused on single responsibilities

Remember: This project handles academic research data and interfaces with external services. Security, reliability, and ethical usage are paramount considerations in all development decisions.