# Comprehensive Test Suite - RSH-13

This directory contains the comprehensive test suite for the rust-sci-hub-mcp project, implementing RSH-13 requirements.

## Test Categories

### 1. Unit Tests (`src/*/tests.rs`)
- **Location**: Embedded in source files using `#[cfg(test)]` modules
- **Coverage**: >90% code coverage target
- **Purpose**: Test individual functions and modules in isolation
- **Examples**: Configuration validation, error handling, data structures

### 2. Integration Tests (`tests/*.rs`)
- **Basic Integration**: `integration_test.rs` - Core functionality integration
- **Server Integration**: `server_integration_test.rs` - MCP server lifecycle
- **Service Integration**: `service_integration_test.rs` - Background service functionality

### 3. End-to-End Tests (`tests/end_to_end_test.rs`)
- **Complete Workflows**: Full user scenarios from search to download
- **Server Lifecycle**: Complete server startup, operation, and shutdown
- **Error Recovery**: Resilience testing with service failures
- **Concurrent Operations**: Multi-threaded operation validation

### 4. Property-Based Tests (`tests/property_tests.rs`)
- **DOI Validation**: Property-based testing of DOI format validation
- **Configuration**: Validation of configuration parameters across ranges
- **Error Categorization**: Consistency of error classification
- **Search Algorithms**: Query normalization and validation properties
- **Rate Limiting**: Property testing of rate limiter behavior

### 5. Security Tests (`tests/security_tests.rs`)
- **SQL Injection**: Protection against SQL injection attacks
- **XSS Prevention**: Cross-site scripting prevention
- **Path Traversal**: File system path traversal protection
- **Command Injection**: Protection against command injection
- **Input Validation**: Comprehensive input sanitization testing
- **DoS Protection**: Denial of service protection mechanisms

### 6. Compatibility Tests (`tests/compatibility_tests.rs`)
- **Cross-Platform**: Path handling across different operating systems
- **Network Stack**: IPv4/IPv6 compatibility
- **File Systems**: Different file system behaviors
- **Environment Variables**: Environment configuration handling
- **Memory Management**: Memory usage patterns and limits
- **Platform-Specific**: macOS, Linux, Windows specific functionality

### 7. Performance Tests (`benches/*.rs`)
- **Search Benchmarks**: `search_bench.rs` - Search operation performance
- **Download Benchmarks**: `download_bench.rs` - Download operation performance  
- **Server Benchmarks**: `server_bench.rs` - Server throughput and latency

## Test Execution

### Running All Tests
```bash
# Run all unit and integration tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test file
cargo test --test end_to_end_test

# Run tests with coverage
cargo tarpaulin --out Html
```

### Running Benchmarks
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench search_bench

# Generate HTML reports
cargo bench -- --output-format html
```

### Running Property Tests
```bash
# Run property tests with more cases
PROPTEST_CASES=10000 cargo test property_tests

# Run property tests with verbose output
PROPTEST_VERBOSE=1 cargo test property_tests
```

## Test Configuration

### Environment Variables
- `PROPTEST_CASES`: Number of property test cases (default: 256)
- `PROPTEST_VERBOSE`: Enable verbose property test output
- `RUST_LOG`: Set logging level for tests
- `RUST_SCI_HUB_MCP_*`: Environment-specific configuration

### Test Dependencies
- `tokio-test`: Async testing utilities
- `wiremock`: HTTP mocking for external services
- `tempfile`: Temporary file and directory creation
- `proptest`: Property-based testing framework
- `criterion`: Performance benchmarking
- `tarpaulin`: Code coverage reporting

## Mock Services

### Sci-Hub Mock Server
The test suite includes comprehensive mock servers using `wiremock` to simulate:
- Successful paper searches and downloads
- Various HTTP error conditions (503, 404, 429, etc.)
- Rate limiting scenarios
- Network timeouts and failures
- Invalid responses and edge cases

### Example Mock Setup
```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

let mock_server = MockServer::start().await;

Mock::given(method("GET"))
    .and(path("/10.1000/test.doi"))
    .respond_with(ResponseTemplate::new(200)
        .set_body_string("<html>Mock response</html>"))
    .mount(&mock_server)
    .await;
```

## Code Coverage

### Target: >90% Coverage
- **Unit Tests**: Cover all public APIs and critical logic paths
- **Integration Tests**: Cover component interactions
- **Error Paths**: Ensure all error conditions are tested
- **Edge Cases**: Test boundary conditions and unusual inputs

### Coverage Reporting
```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# View coverage report
open coverage/tarpaulin-report.html
```

### Coverage Exclusions
- Build scripts (`build.rs`)
- Test code itself
- Unreachable panic conditions
- Platform-specific code not available on test platform

## Continuous Integration

### Automated Test Execution
The test suite is designed for CI/CD pipeline execution:
- Fast unit tests run on every commit
- Integration tests run on pull requests
- Full test suite runs on main branch
- Performance regression detection
- Security vulnerability scanning

### CI Configuration Requirements
```yaml
# Example CI steps
- name: Run Tests
  run: cargo test --all-features

- name: Run Security Tests  
  run: cargo test --test security_tests

- name: Check Coverage
  run: cargo tarpaulin --fail-under 90

- name: Run Benchmarks
  run: cargo bench --no-run
```

## Test Fixtures and Utilities

### Shared Test Utilities
- `TestConfig`: Standard test configuration factory
- `MockSciHub`: Reusable mock server setup
- `TempEnvironment`: Isolated test environment creation
- `AssertionHelpers`: Custom assertion macros

### Test Data Management
- Sample DOIs and papers for testing
- Mock PDF content for download tests
- Error response templates
- Performance baseline data

## Security Testing

### Vulnerability Categories Tested
1. **Input Validation**: SQL injection, XSS, command injection
2. **Path Security**: Directory traversal, null byte injection
3. **Resource Limits**: DoS protection, memory exhaustion
4. **Authentication**: Access control validation
5. **Network Security**: SSL/TLS configuration, secure defaults

### Security Test Guidelines
- Test with real-world attack vectors
- Validate all user inputs are sanitized
- Ensure error messages don't leak sensitive information
- Test resource limits and denial-of-service protection
- Verify secure defaults in configuration

## Performance Testing

### Benchmark Categories
1. **Throughput**: Requests per second under load
2. **Latency**: Response time percentiles
3. **Memory Usage**: Memory consumption patterns
4. **Concurrent Operations**: Performance under concurrency
5. **Resource Utilization**: CPU, memory, network usage

### Performance Baselines
- Search operations: < 500ms average response time
- Download operations: > 1MB/s throughput
- Server startup: < 2 seconds
- Memory usage: < 100MB baseline, < 500MB under load
- Concurrent requests: Support 100+ concurrent operations

## Test Maintenance

### Adding New Tests
1. Follow existing test patterns and naming conventions
2. Include both positive and negative test cases
3. Add property tests for algorithms with invariants
4. Update this documentation when adding new test categories
5. Ensure tests are deterministic and not flaky

### Test Review Checklist
- [ ] Tests cover all acceptance criteria
- [ ] Error conditions are tested
- [ ] Performance implications are considered
- [ ] Security aspects are validated
- [ ] Cross-platform compatibility is addressed
- [ ] Mock services cover realistic scenarios
- [ ] Tests are maintainable and well-documented

## Troubleshooting

### Common Test Issues
1. **Network Tests Failing**: Check internet connectivity and mock server setup
2. **File System Tests**: Verify permissions and temporary directory cleanup
3. **Property Test Failures**: Increase test cases or adjust generators
4. **Performance Test Variability**: Run on consistent hardware with stable load
5. **Platform-Specific Failures**: Use conditional compilation for platform differences

### Debug Helpers
```bash
# Run tests with debug output
RUST_LOG=debug cargo test

# Run single test with backtrace
RUST_BACKTRACE=1 cargo test test_name -- --exact

# Run tests with timing
cargo test -- --report-time
```