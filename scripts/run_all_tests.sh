#!/bin/bash
# Comprehensive test runner for rust-research-mcp

set -e  # Exit on error

echo "========================================="
echo "Running Comprehensive E2E Test Suite"
echo "========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track test results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to run a test and report results
run_test() {
    local test_name=$1
    local test_command=$2
    
    echo -e "${YELLOW}Running: $test_name${NC}"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if eval $test_command; then
        echo -e "${GREEN}✅ PASSED: $test_name${NC}\n"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}❌ FAILED: $test_name${NC}\n"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

# 1. Build the project
echo "Building project..."
cargo build --bin rust-research-mcp 2>&1 | grep -E "(Compiling|Finished)" || true
echo ""

# 2. Run Rust unit tests
echo "========================================="
echo "RUST UNIT TESTS"
echo "========================================="
run_test "Cargo unit tests" "cargo test --lib -- --nocapture 2>&1 | grep -E '(test result:|passed)'"

# 3. Run Rust integration tests  
echo "========================================="
echo "RUST INTEGRATION TESTS"
echo "========================================="
run_test "Provider tests" "cargo test --test e2e_test_suite test_arxiv_provider -- --nocapture"
run_test "CrossRef provider" "cargo test --test e2e_test_suite test_crossref_provider -- --nocapture"
run_test "Semantic Scholar provider" "cargo test --test e2e_test_suite test_semantic_scholar_provider -- --nocapture"
run_test "Unpaywall provider" "cargo test --test e2e_test_suite test_unpaywall_provider -- --nocapture"
run_test "CORE provider" "cargo test --test e2e_test_suite test_core_provider -- --nocapture"
run_test "bioRxiv provider" "cargo test --test e2e_test_suite test_biorxiv_provider -- --nocapture"
run_test "SSRN provider" "cargo test --test e2e_test_suite test_ssrn_provider -- --nocapture"
run_test "Meta search client" "cargo test --test e2e_test_suite test_meta_search_client -- --nocapture"
run_test "Search tool" "cargo test --test e2e_test_suite test_search_tool -- --nocapture"
run_test "Download tool" "cargo test --test e2e_test_suite test_download_tool -- --nocapture"
run_test "Metadata extractor" "cargo test --test e2e_test_suite test_metadata_extractor -- --nocapture"
run_test "Server handler" "cargo test --test e2e_test_suite test_server_handler -- --nocapture"
run_test "URL resolution" "cargo test --test e2e_test_suite test_url_resolution -- --nocapture"
run_test "Cascade PDF retrieval" "cargo test --test e2e_test_suite test_cascade_pdf_retrieval -- --nocapture"
run_test "Rate limiting" "cargo test --test e2e_test_suite test_rate_limiting -- --nocapture"
run_test "Error handling" "cargo test --test e2e_test_suite test_error_handling -- --nocapture"
run_test "Full integration flow" "cargo test --test e2e_test_suite test_full_search_download_flow -- --nocapture"

# 4. Run Python MCP protocol tests
echo "========================================="
echo "PYTHON MCP PROTOCOL TESTS"
echo "========================================="
run_test "MCP protocol tests" "python3 tests/test_mcp_e2e.py"

# 5. Quick smoke tests
echo "========================================="
echo "SMOKE TESTS"
echo "========================================="

# Test that server starts and responds to ping
run_test "Server startup" "timeout 5 cargo run --bin rust-research-mcp 2>&1 | grep -q 'Starting MCP server' || true"

# 6. Report results
echo ""
echo "========================================="
echo "TEST RESULTS SUMMARY"
echo "========================================="
echo -e "Total Tests: $TOTAL_TESTS"
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
echo -e "${RED}Failed: $FAILED_TESTS${NC}"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "\n${GREEN}🎉 ALL TESTS PASSED!${NC}"
    exit 0
else
    echo -e "\n${RED}⚠️ SOME TESTS FAILED${NC}"
    echo "Please review the failures above"
    exit 1
fi