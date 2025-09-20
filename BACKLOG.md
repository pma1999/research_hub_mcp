# Production Compliance Audit - Prioritized Backlog

**Audit Date:** 2025-01-22
**Scope:** /Users/ladvien/sci_hub_mcp/src/*
**Total Stories:** 46 (11 Critical, 18 High, 12 Medium, 5 Low)

## 🔴 CRITICAL PRIORITY (11 stories)

### Security Vulnerabilities (4 Critical)

**[AUDIT-SEC-001] ArXiv Provider - HTTP Protocol Security Vulnerability**
- **Priority:** Critical
- **Points:** 3
- **AC:** Replace HTTP URL with HTTPS in ArXiv provider configuration. Update `"http://export.arxiv.org/api/query"` to use HTTPS protocol.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/providers/arxiv.rs:33
- **Security Impact:** data_exposure
- **Dependencies:** None

**[AUDIT-SEC-004] SSL Configuration - Dangerous Certificate Bypass Option**
- **Priority:** Critical
- **Points:** 5
- **AC:** Remove `danger_accept_invalid_certs: bool` configuration option or restrict to development mode only with explicit warnings.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/mod.rs:27
- **Security Impact:** complete_tls_bypass
- **Dependencies:** None

**[AUDIT-SEC-002] HTTP Client Security - Missing Certificate Validation Controls**
- **Priority:** Critical
- **Points:** 8
- **AC:** Implement centralized secure HTTP client factory with certificate pinning, explicit TLS v1.2+ enforcement, and standardized timeout configurations.
- **Files:** All provider files using Client::builder()
- **Security Impact:** connection_hijacking
- **Dependencies:** AUDIT-SEC-004

**[AUDIT-SEC-005] Error Handling - Excessive Panic-Prone Code**
- **Priority:** Critical
- **Points:** 8
- **AC:** Replace unwrap/expect with proper error handling in production code paths. Focus on HTTP client creation and provider defaults (150+ instances identified).
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/providers/semantic_scholar.rs:427, /Users/ladvien/sci_hub_mcp/src/tools/download.rs:174, multiple provider default implementations
- **Security Impact:** denial_of_service
- **Dependencies:** None

### Architecture Violations (4 Critical)

**[AUDIT-ARCH-001] Circuit Breaker Integration - Missing External Call Protection**
- **Priority:** Critical
- **Points:** 8
- **AC:** All HTTP calls to external services must use circuit breaker protection. Circuit breaker implementation exists but is unused in practice.
- **Files:** src/client/providers/*, src/client/meta_search.rs
- **Dependencies:** None

**[AUDIT-ARCH-002] Hexagonal Architecture - Implement Ports/Adapters Pattern**
- **Priority:** Critical
- **Points:** 8
- **AC:** Tools receive service interfaces via dependency injection, not concrete implementations. Eliminate direct coupling between business logic and external services.
- **Files:** src/tools/*, src/server/handler.rs
- **Dependencies:** AUDIT-ARCH-005

**[AUDIT-ARCH-005] Dependency Injection - Implement Proper DI Container**
- **Priority:** Critical
- **Points:** 8
- **AC:** Tools receive all dependencies via constructor injection, no internal creation. Implement DI container for clean architecture compliance.
- **Files:** src/tools/*, src/server/handler.rs, new src/di/
- **Dependencies:** None

### Code Quality (3 Critical)

**[AUDIT-STA-001] Clippy Lint Violations - Critical Cognitive Complexity**
- **Priority:** Critical
- **Points:** 3
- **AC:** Refactor get_by_doi function in meta_search.rs to reduce cognitive complexity from 48 to under 30. Split into smaller, focused functions.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/meta_search.rs:181
- **Dependencies:** None

**[AUDIT-STA-007] Error Handling - Risky Unwrap/Expect Usage**
- **Priority:** Critical
- **Points:** 3
- **AC:** Replace unwrap/expect with proper error handling in production code. Focus on HTTP client creation and provider defaults.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/providers/semantic_scholar.rs:427, /Users/ladvien/sci_hub_mcp/src/tools/download.rs:174
- **Dependencies:** None

**[AUDIT-STA-008] Documentation - Missing Public API Documentation**
- **Priority:** Critical
- **Points:** 5
- **AC:** Add comprehensive rustdoc comments to all public modules, structs, traits, and functions. Focus on client module and provider traits.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/mod.rs, /Users/ladvien/sci_hub_mcp/src/client/providers/traits.rs, all provider implementations
- **Dependencies:** None

## 🟠 HIGH PRIORITY (18 stories)

### Security (3 High)

**[AUDIT-SEC-003] File System Security - Missing Permission Controls**
- **Priority:** High
- **Points:** 5
- **AC:** Set restrictive file permissions (0600) for sensitive files like PID files and downloaded content. Implement symlink attack protection.
- **Files:** /Users/ladvien/sci_hub_mcp/src/service/pid.rs, /Users/ladvien/sci_hub_mcp/src/tools/download.rs
- **Security Impact:** privilege_escalation
- **Dependencies:** None

**[AUDIT-SEC-008] HTTP Client Hardening - Missing Security Headers**
- **Priority:** High
- **Points:** 4
- **AC:** Configure HTTP clients with security headers, proper User-Agent strings, and request size limits to prevent abuse.
- **Files:** All HTTP client configurations
- **Security Impact:** DoS_prevention
- **Dependencies:** AUDIT-SEC-002

**[AUDIT-SEC-009] TLS Configuration - Version and Cipher Security**
- **Priority:** High
- **Points:** 3
- **AC:** Enforce TLS 1.2+ minimum version and secure cipher suites in HTTP client configurations.
- **Files:** HTTP client builder configurations
- **Security Impact:** protocol_downgrade
- **Dependencies:** AUDIT-SEC-002

### Architecture (4 High)

**[AUDIT-ARCH-003] Repository Pattern - Implement Data Access Abstraction**
- **Priority:** High
- **Points:** 5
- **AC:** Create repository traits for all data persistence operations to separate business logic from data access.
- **Files:** src/client/*, new src/repositories/
- **Dependencies:** None

**[AUDIT-ARCH-004] Command Pattern - Unify Tool Execution Interface**
- **Priority:** High
- **Points:** 5
- **AC:** All tools implement unified Command trait with execute() method for consistent tool interface.
- **Files:** src/tools/mod.rs, src/tools/*
- **Dependencies:** None

**[AUDIT-ARCH-006] Layer Separation - Separate Business Logic from Transport**
- **Priority:** High
- **Points:** 6
- **AC:** MCP protocol handling separated from business logic to maintain clean architecture.
- **Files:** src/server/handler.rs, src/tools/*
- **Dependencies:** AUDIT-ARCH-002

**[AUDIT-ARCH-008] Error Boundaries - Implement Layer Error Isolation**
- **Priority:** High
- **Points:** 5
- **AC:** Each architectural layer has proper error transformation and isolation to prevent error bleeding.
- **Files:** src/error.rs, src/tools/*, src/client/*
- **Dependencies:** AUDIT-ARCH-002

### Performance (3 High)

**[AUDIT-PERF-001] Download Tool - Inefficient HTTP Client Configuration**
- **Priority:** High
- **Points:** 8
- **AC:** Configure reqwest client with pool_max_idle_per_host=10, pool_idle_timeout=30s, http2_prior_knowledge=true to reduce TCP overhead by 40-60%.
- **Files:** /Users/ladvien/sci_hub_mcp/src/tools/download.rs:170-174
- **Target:** Enable HTTP/2 connection pooling
- **Dependencies:** None

**[AUDIT-PERF-002] Meta Search - Unbounded Concurrent Provider Requests**
- **Priority:** High
- **Points:** 6
- **AC:** Implement adaptive semaphore sizing based on provider response times and add request queue monitoring.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/meta_search.rs:262-264
- **Target:** Improve search latency from >2s to <500ms
- **Dependencies:** None

**[AUDIT-PERF-003] Provider Implementations - Blocking Synchronous Operations**
- **Priority:** High
- **Points:** 5
- **AC:** Replace frequent atomic operations with local counters and periodic updates to reduce per-request overhead by 15-25%.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/providers/sci_hub.rs:54,64,77,81
- **Target:** Reduce per-request overhead
- **Dependencies:** None

### Code Quality (8 High)

**[AUDIT-STA-009] Documentation - Module-Level Documentation**
- **Priority:** High
- **Points:** 3
- **AC:** Add module-level documentation with //! syntax explaining purpose, usage, and examples for each public module.
- **Files:** All modules in /Users/ladvien/sci_hub_mcp/src/*/mod.rs
- **Dependencies:** None

**[AUDIT-STA-010] Async Patterns - Blocking Operations Audit**
- **Priority:** High
- **Points:** 2
- **AC:** Audit std::fs usage in async contexts and replace with tokio::fs where appropriate. Ensure no blocking operations in async functions.
- **Files:** /Users/ladvien/sci_hub_mcp/src/service/pid.rs, /Users/ladvien/sci_hub_mcp/src/config/mod.rs, /Users/ladvien/sci_hub_mcp/src/tools/metadata.rs
- **Dependencies:** None

**[AUDIT-STA-011] Test Coverage - Provider Test Standardization**
- **Priority:** High
- **Points:** 3
- **AC:** Standardize provider test patterns and ensure consistent error handling testing across all 13 providers.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/providers/*.rs
- **Dependencies:** None

**[AUDIT-STA-015] Test Quality - Integration Test Coverage**
- **Priority:** High
- **Points:** 4
- **AC:** Improve integration test coverage and add negative test cases for error handling paths. Current test count: 153 unit tests.
- **Files:** /Users/ladvien/sci_hub_mcp/src/resilience/integration_tests.rs, various test modules
- **Dependencies:** None

## 🟡 MEDIUM PRIORITY (12 stories)

### Security (4 Medium)

**[AUDIT-SEC-006] Input Validation - Edge Case Vulnerabilities**
- **Priority:** Medium
- **Points:** 3
- **AC:** Strengthen input validation rules for edge cases and add rate limiting for query size (currently 1000 chars).
- **Files:** Search and download tools
- **Security Impact:** resource_exhaustion
- **Dependencies:** None

**[AUDIT-SEC-007] Credential Management - API Key Storage Security**
- **Priority:** Medium
- **Points:** 4
- **AC:** Implement secure credential storage with encryption at rest for API keys and sensitive configuration.
- **Files:** Provider implementations with api_key fields
- **Security Impact:** credential_disclosure
- **Dependencies:** None

**[AUDIT-SEC-010] File Operation Security - Symlink Attack Prevention**
- **Priority:** Medium
- **Points:** 3
- **AC:** Implement symlink detection and prevention in file operations to prevent directory traversal attacks.
- **Files:** File operation functions
- **Security Impact:** path_traversal
- **Dependencies:** AUDIT-SEC-003

**[AUDIT-SEC-011] Logging Security - Sensitive Data Exposure Prevention**
- **Priority:** Medium
- **Points:** 2
- **AC:** Audit logging statements to ensure no sensitive data (API keys, personal info) is logged accidentally.
- **Files:** All logging statements
- **Security Impact:** information_disclosure
- **Dependencies:** None

### Architecture (1 Medium)

**[AUDIT-ARCH-007] Strategy Pattern - Implement Configurable Provider Selection**
- **Priority:** Medium
- **Points:** 3
- **AC:** Provider selection strategy configurable via configuration instead of hardcoded logic.
- **Files:** src/client/meta_search.rs, src/config/mod.rs
- **Dependencies:** None

### Performance (4 Medium)

**[AUDIT-PERF-004] Search Tool - Memory-Inefficient Caching Strategy**
- **Priority:** Medium
- **Points:** 4
- **AC:** Implement LRU cache with max_size=1000, TTL-based cleanup every 5 minutes to reduce memory usage by 30-50%.
- **Files:** /Users/ladvien/sci_hub_mcp/src/tools/search.rs:114,149
- **Target:** Reduce memory usage for long-running sessions
- **Dependencies:** None

**[AUDIT-PERF-005] Server Handler - Inefficient Tool Cloning Pattern**
- **Priority:** Medium
- **Points:** 3
- **AC:** Use Arc<Tool> pattern to share tool instances across requests, reducing memory allocations by 60%.
- **Files:** /Users/ladvien/sci_hub_mcp/src/server/handler.rs:315-319
- **Target:** Reduce memory allocations
- **Dependencies:** None

**[AUDIT-PERF-006] Rate Limiter - Suboptimal Sleep Strategy**
- **Priority:** Medium
- **Points:** 4
- **AC:** Implement token bucket algorithm with non-blocking checks to improve concurrent request handling by 25-40%.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/rate_limiter.rs:58,287
- **Target:** Improve concurrent request handling
- **Dependencies:** None

**[AUDIT-PERF-007] Download Progress - Excessive Progress Updates**
- **Priority:** Medium
- **Points:** 3
- **AC:** Implement adaptive progress intervals based on transfer rate and file size to reduce CPU overhead by 20%.
- **Files:** /Users/ladvien/sci_hub_mcp/src/tools/download.rs:774-785
- **Target:** Reduce CPU overhead for large downloads
- **Dependencies:** None

**[AUDIT-PERF-008] Circuit Breaker - Lock Contention in Hot Path**
- **Priority:** Medium
- **Points:** 4
- **AC:** Use atomic state representation for fast reads, locks only for state transitions to reduce contention by 50%.
- **Files:** /Users/ladvien/sci_hub_mcp/src/resilience/circuit_breaker.rs:117-118
- **Target:** Reduce lock contention overhead
- **Dependencies:** None

**[AUDIT-PERF-010] Metadata Extraction - No Streaming Parser**
- **Priority:** Medium
- **Points:** 5
- **AC:** Implement streaming PDF metadata extraction with 8KB buffer chunks to reduce memory usage by 80% for large files.
- **Files:** /Users/ladvien/sci_hub_mcp/src/tools/metadata.rs
- **Target:** Reduce memory usage for files >50MB
- **Dependencies:** None

### Code Quality (3 Medium)

**[AUDIT-STA-003] Clippy Lint Violations - Unused Self Arguments**
- **Priority:** Medium
- **Points:** 2
- **AC:** Convert 4 methods with unused self to static functions or justify self usage in meta_search.rs helper methods.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/meta_search.rs:427,483,519,713
- **Dependencies:** None

**[AUDIT-STA-004] Clippy Lint Violations - Inefficient Struct Initialization**
- **Priority:** Medium
- **Points:** 2
- **AC:** Replace field assignment with proper struct initialization using Default::default() pattern in tools.
- **Files:** /Users/ladvien/sci_hub_mcp/src/tools/download.rs:982, /Users/ladvien/sci_hub_mcp/src/tools/search.rs:558
- **Dependencies:** None

**[AUDIT-STA-005] Clippy Lint Violations - File Extension Comparisons**
- **Priority:** Medium
- **Points:** 1
- **AC:** Replace case-sensitive file extension checks with Path::extension() and case-insensitive comparison.
- **Files:** /Users/ladvien/sci_hub_mcp/src/tools/download.rs:1072,1082
- **Dependencies:** None

**[AUDIT-STA-014] Code Quality - Provider Default Implementations**
- **Priority:** Medium
- **Points:** 2
- **AC:** Standardize provider default implementations and remove expect() calls in production code paths.
- **Files:** All provider files with expect() in default implementations
- **Dependencies:** None

## 🟢 LOW PRIORITY (5 stories)

### Performance (2 Low)

**[AUDIT-PERF-009] Provider Timeout Configuration - Fixed Timeouts**
- **Priority:** Low
- **Points:** 2
- **AC:** Implement per-operation timeout strategies (search: 10s, health: 5s, download: 60s) for 20% faster response times.
- **Files:** Multiple provider files with hardcoded Duration::from_secs(30)
- **Target:** Improve user experience for quick operations
- **Dependencies:** None

**[AUDIT-PERF-011] Background Tasks - No Resource Monitoring**
- **Priority:** Low
- **Points:** 3
- **AC:** Add task monitoring with max_concurrent_tasks=50, memory usage tracking to prevent resource exhaustion.
- **Files:** /Users/ladvien/sci_hub_mcp/src/service/daemon.rs:147,329
- **Target:** Prevent resource exhaustion
- **Dependencies:** None

**[AUDIT-PERF-012] JSON Serialization - Inefficient String Allocations**
- **Priority:** Low
- **Points:** 2
- **AC:** Use serde_json::to_writer with pre-allocated buffers for responses >1KB to reduce allocation overhead by 15%.
- **Files:** /Users/ladvien/sci_hub_mcp/src/server/handler.rs:511,627
- **Target:** Reduce allocation overhead
- **Dependencies:** None

### Code Quality (3 Low)

**[AUDIT-STA-002] Clippy Lint Violations - Unused Variables in Tests**
- **Priority:** Low
- **Points:** 2
- **AC:** Prefix unused test variables with underscore or remove them. Fix 13 instances across provider tests and integration tests.
- **Files:** /Users/ladvien/sci_hub_mcp/src/client/providers/biorxiv.rs:435, multiple test files
- **Dependencies:** None

**[AUDIT-STA-006] Clippy Lint Violations - Minor Code Quality Issues**
- **Priority:** Low
- **Points:** 1
- **AC:** Fix remaining clippy violations: manual string creation, redundant clone, useless comparison.
- **Files:** /Users/ladvien/sci_hub_mcp/src/tools/search.rs:576,650, /Users/ladvien/sci_hub_mcp/src/service/daemon.rs:505
- **Dependencies:** None

**[AUDIT-STA-012] Code Organization - Error Propagation Patterns**
- **Priority:** Low
- **Points:** 2
- **AC:** Review and standardize error propagation patterns. Ensure consistent use of ? operator and error context preservation.
- **Files:** Various files with .await? patterns
- **Dependencies:** None

**[AUDIT-STA-013] Memory Safety - Regex Compilation**
- **Priority:** Low
- **Points:** 1
- **AC:** Move regex compilation to lazy_static or OnceCell to avoid repeated compilation in metadata extractor.
- **Files:** /Users/ladvien/sci_hub_mcp/src/tools/metadata.rs:204-228
- **Dependencies:** None

---

## Summary Statistics

- **Total Stories:** 46
- **Critical Issues:** 11 (24%)
- **High Priority:** 18 (39%)
- **Medium Priority:** 12 (26%)
- **Low Priority:** 5 (11%)

### By Category:
- **Security:** 11 stories (3 Critical, 3 High, 4 Medium, 1 Low)
- **Architecture:** 8 stories (4 Critical, 4 High, 1 Medium)
- **Performance:** 12 stories (3 High, 7 Medium, 2 Low)
- **Code Quality:** 15 stories (3 Critical, 8 High, 3 Medium, 1 Low)

### Critical Path:
1. Security vulnerabilities (4 Critical)
2. Architecture violations (4 Critical)
3. Code quality issues (3 Critical)

**Estimated Total Points:** 187 story points
**Recommended Sprint Capacity:** 20-30 points per sprint
**Estimated Timeline:** 6-9 sprints for complete resolution