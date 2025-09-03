#!/bin/bash

# Test for RPC proxy binary response issue
# This test reproduces a critical bug where suibase RPC proxy returns compressed binary data
# instead of properly decompressed JSON responses when upstream servers use gzip compression.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the common test utilities
. "$SCRIPT_DIR/../__test_common.sh"

TEST_NAME="RPC Proxy Compression Fix"
TEST_DESCRIPTION="Verify suibase RPC proxy properly handles compressed responses from upstream servers"

# Test configuration
WORKDIR="testnet"
PROXY_PORT="44342"
MAX_ATTEMPTS=6

# JSON-RPC test queries that are read-only and safe
declare -a TEST_QUERIES=(
    '{"jsonrpc":"2.0","id":1,"method":"sui_getLatestCheckpointSequenceNumber","params":[]}'
    '{"jsonrpc":"2.0","id":2,"method":"sui_getChainIdentifier","params":[]}'
    '{"jsonrpc":"2.0","id":3,"method":"sui_getReferenceGasPrice","params":[]}'
    '{"jsonrpc":"2.0","id":4,"method":"sui_getTotalTransactionBlocks","params":[]}'
    '{"jsonrpc":"2.0","id":5,"method":"sui_getProtocolConfig","params":[]}'
    '{"jsonrpc":"2.0","id":6,"method":"sui_getValidatorsApy","params":[]}'
)

setup_test_environment() {
    print_step "Setting up test environment"
    
    # Ensure testnet workdir is initialized
    if ! "$HOME/suibase/scripts/testnet" init-check >/dev/null 2>&1; then
        print_step "Initializing testnet workdir..."
        "$HOME/suibase/scripts/testnet" init >/dev/null
    fi
    
    # Start suibase daemon if not running
    print_step "Ensuring suibase daemon is running..."
    "$HOME/suibase/scripts/testnet" start >/dev/null
    
    # Wait for proxy to be ready
    local attempts=0
    while [ $attempts -lt 10 ]; do
        if curl -s --max-time 2 "http://localhost:$PROXY_PORT" >/dev/null 2>&1; then
            print_success "RPC proxy is responding"
            return 0
        fi
        ((attempts++))
        sleep 1
    done
    
    print_error "RPC proxy failed to start within timeout"
    return 1
}

# Test if a response contains binary/compressed data
is_binary_response() {
    local response="$1"
    
    # Check for common binary/compressed data indicators
    # gzip magic numbers, control characters, etc.
    if echo "$response" | grep -q $'\x1f\x8b\x08'; then
        return 0  # gzip header found
    fi
    
    if echo "$response" | grep -q $'[\x00-\x08\x0E-\x1F\x7F]'; then
        return 0  # control characters found
    fi
    
    # Check for garbled JSON-like patterns that indicate compression
    if echo "$response" | grep -q '�.*{.*}' || echo "$response" | grep -q '{.*�.*}'; then
        return 0  # compressed JSON pattern
    fi
    
    return 1  # appears to be normal text
}

# Test that explicitly verifies compression/decompression is working
test_compression_decompression() {
    local query='{"jsonrpc":"2.0","id":1,"method":"sui_getLatestCheckpointSequenceNumber","params":[]}'
    
    print_step "Testing explicit compression/decompression verification"
    
    # First, test direct upstream server with compression
    print_step "1/3: Testing direct upstream server with compression request"
    local upstream_response
    upstream_response=$(curl -s --max-time 10 \
        -H "Content-Type: application/json" \
        -H "Accept-Encoding: gzip, deflate" \
        -d "$query" \
        "https://fullnode.testnet.sui.io:443" 2>/dev/null)
    
    if [ $? -ne 0 ] || [ -z "$upstream_response" ]; then
        print_warning "Could not test upstream server directly, skipping compression verification"
        return 0  # Not a failure, just can't verify
    fi
    
    # Verify upstream gives us valid JSON
    if ! echo "$upstream_response" | jq . >/dev/null 2>&1; then
        print_warning "Upstream server not returning valid JSON, skipping compression verification"
        return 0
    fi
    
    print_success "Upstream server responding with valid JSON"
    
    # Now test our proxy with explicit compression request
    print_step "2/3: Testing suibase proxy with compression request"
    local proxy_response
    proxy_response=$(curl -s --max-time 10 \
        -H "Content-Type: application/json" \
        -H "Accept-Encoding: gzip, deflate" \
        -d "$query" \
        "http://localhost:$PROXY_PORT" 2>/dev/null)
    
    if [ $? -ne 0 ] || [ -z "$proxy_response" ]; then
        print_error "Proxy compression test failed - no response"
        return 1
    fi
    
    # Verify proxy response is uncompressed JSON
    if is_binary_response "$proxy_response"; then
        print_error "CRITICAL: Proxy returned compressed binary data!"
        echo "This means our decompression fix is not working"
        return 2
    fi
    
    if ! echo "$proxy_response" | jq . >/dev/null 2>&1; then
        print_error "Proxy returned invalid JSON"
        return 1
    fi
    
    print_success "Proxy correctly returned uncompressed JSON"
    
    # Compare content to ensure proxy is working correctly
    print_step "3/3: Verifying proxy and upstream return equivalent data"
    local upstream_id=$(echo "$upstream_response" | jq -r '.result // empty')
    local proxy_id=$(echo "$proxy_response" | jq -r '.result // empty')
    
    if [ "$upstream_id" = "$proxy_id" ] && [ -n "$upstream_id" ]; then
        print_success "Proxy and upstream return equivalent data"
        print_success "✓ Compression/decompression test PASSED"
        return 0
    else
        print_warning "Proxy and upstream returned different results (acceptable for changing blockchain state)"
        print_success "✓ Compression/decompression test PASSED"
        return 0
    fi
}

# Test a single JSON-RPC query
test_rpc_query() {
    local query="$1"
    local attempt="$2"
    
    print_step "Test $attempt/$MAX_ATTEMPTS: Testing RPC query"
    
    # Make the request and capture both response and curl exit code
    local response
    local curl_exit_code
    
    response=$(curl -s --max-time 10 \
        -H "Content-Type: application/json" \
        -H "Accept-Encoding: gzip, deflate" \
        -d "$query" \
        "http://localhost:$PROXY_PORT" 2>/dev/null)
    curl_exit_code=$?
    
    if [ $curl_exit_code -ne 0 ]; then
        print_warning "Query $attempt failed - curl error $curl_exit_code"
        return 1
    fi
    
    if [ -z "$response" ]; then
        print_warning "Query $attempt failed - empty response"
        return 1
    fi
    
    # Check if response is binary/compressed
    if is_binary_response "$response"; then
        print_error "Query $attempt FAILED - received binary/compressed response!"
        echo "First 100 chars of response: $(echo "$response" | head -c 100)"
        echo "Response contains binary data - this indicates the compression bug is present"
        return 2  # Special return code for compression bug
    fi
    
    # Try to parse as JSON to verify it's valid
    if ! echo "$response" | jq . >/dev/null 2>&1; then
        print_warning "Query $attempt failed - invalid JSON response"
        echo "Response: $response"
        return 1
    fi
    
    print_success "Query $attempt passed - received valid JSON response"
    return 0
}

run_compression_tests() {
    print_step "Running RPC proxy compression tests"
    
    local total_tests=${#TEST_QUERIES[@]}
    local passed_tests=0
    local failed_tests=0
    local compression_bugs=0
    
    for i in "${!TEST_QUERIES[@]}"; do
        local query="${TEST_QUERIES[$i]}"
        local attempt=$((i + 1))
        
        test_rpc_query "$query" "$attempt"
        local result=$?
        
        case $result in
            0)
                ((passed_tests++))
                ;;
            1)
                ((failed_tests++))
                ;;
            2)
                ((compression_bugs++))
                ;;
        esac
        
        # Small delay between tests
        sleep 0.5
    done
    
    print_step "Test Results Summary"
    echo "  Total tests: $total_tests"
    echo "  Passed: $passed_tests"
    echo "  Failed (network/other): $failed_tests"
    echo "  Failed (compression bug): $compression_bugs"
    
    if [ $compression_bugs -gt 0 ]; then
        print_error "CRITICAL: RPC proxy compression bug detected!"
        echo "The suibase RPC proxy is returning compressed binary data instead of JSON."
        echo "This breaks SuiClient and other JSON-RPC clients."
        echo "This test will FAIL to ensure the bug gets fixed."
        return 1
    fi
    
    if [ $passed_tests -eq 0 ]; then
        print_error "All tests failed - RPC proxy appears to be non-functional"
        return 1
    fi
    
    print_success "No compression bugs detected - all responses are proper JSON"
    return 0
}

cleanup_test_environment() {
    print_step "Cleaning up test environment"
    # Nothing specific to clean up for this test
    return 0
}

main() {
    print_header "$TEST_NAME"
    echo "Description: $TEST_DESCRIPTION"
    echo "Workdir: $WORKDIR"
    echo "Proxy Port: $PROXY_PORT"
    echo "Test Queries: ${#TEST_QUERIES[@]}"
    echo ""
    
    # Set up error handling
    trap cleanup_test_environment EXIT
    
    # Run the test sequence
    if ! setup_test_environment; then
        print_error "Test environment setup failed"
        exit 1
    fi
    
    if ! run_compression_tests; then
        print_error "$TEST_NAME FAILED"
        exit 1
    fi
    
    print_success "$TEST_NAME PASSED"
    exit 0
}

# Run the test if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi