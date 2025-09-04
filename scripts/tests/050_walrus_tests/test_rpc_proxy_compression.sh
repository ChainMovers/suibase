#!/bin/bash

# Test for RPC proxy binary response issue
# This test reproduces a critical bug where suibase RPC proxy returns compressed binary data
# instead of properly decompressed JSON responses when upstream servers use gzip compression.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the walrus test common utilities (which includes build daemon functionality)
. "$SCRIPT_DIR/__test_common.sh"

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
    echo "--- Setting up test environment"
    
    # Setup clean environment and ensure BUILD version of daemon
    setup_clean_environment
    setup_test_workdir "$WORKDIR"
    backup_config_files "$WORKDIR"
    
    # Ensure BUILD version of suibase-daemon is used (not precompiled)
    ensure_build_daemon
    
    # Start daemon using safe method that ensures BUILD version
    safe_start_daemon
    
    # Wait for daemon to be ready
    wait_for_daemon_running 15 true
    
    # Wait for proxy to be ready on the specific port
    local attempts=0
    while [ $attempts -lt 10 ]; do
        if curl -s --max-time 2 "http://localhost:$PROXY_PORT" >/dev/null 2>&1; then
            echo "✓ RPC proxy is responding"
            return 0
        fi
        ((attempts++))
        sleep 1
    done
    
    echo "✗ RPC proxy failed to start within timeout"
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
    
    echo "Testing explicit compression/decompression verification"
    
    # First, test direct upstream server with compression
    echo "1/3: Testing direct upstream server with compression request"
    local upstream_response
    upstream_response=$(curl -s --max-time 10 \
        -H "Content-Type: application/json" \
        -H "Accept-Encoding: gzip, deflate" \
        -d "$query" \
        "https://fullnode.testnet.sui.io:443" 2>/dev/null)
    
    if [ $? -ne 0 ] || [ -z "$upstream_response" ]; then
        echo "⚠ Could not test upstream server directly, skipping compression verification"
        return 0  # Not a failure, just can't verify
    fi
    
    # Verify upstream gives us valid JSON
    if ! echo "$upstream_response" | jq . >/dev/null 2>&1; then
        echo "⚠ Upstream server not returning valid JSON, skipping compression verification"
        return 0
    fi
    
    echo "✓ Upstream server responding with valid JSON"
    
    # Now test our proxy with explicit compression request
    echo "2/3: Testing suibase proxy with compression request"
    local proxy_response
    proxy_response=$(curl -s --max-time 10 \
        -H "Content-Type: application/json" \
        -H "Accept-Encoding: gzip, deflate" \
        -d "$query" \
        "http://localhost:$PROXY_PORT" 2>/dev/null)
    
    if [ $? -ne 0 ] || [ -z "$proxy_response" ]; then
        echo "✗ Proxy compression test failed - no response"
        return 1
    fi
    
    # Verify proxy response is uncompressed JSON
    if is_binary_response "$proxy_response"; then
        echo "✗ CRITICAL: Proxy returned compressed binary data!"
        echo "This means our decompression fix is not working"
        return 2
    fi
    
    if ! echo "$proxy_response" | jq . >/dev/null 2>&1; then
        echo "✗ Proxy returned invalid JSON"
        return 1
    fi
    
    echo "✓ Proxy correctly returned uncompressed JSON"
    
    # Compare content to ensure proxy is working correctly
    echo "3/3: Verifying proxy and upstream return equivalent data"
    local upstream_id=$(echo "$upstream_response" | jq -r '.result // empty')
    local proxy_id=$(echo "$proxy_response" | jq -r '.result // empty')
    
    if [ "$upstream_id" = "$proxy_id" ] && [ -n "$upstream_id" ]; then
        echo "✓ Proxy and upstream return equivalent data"
        echo "✓ Compression/decompression test PASSED"
        return 0
    else
        echo "⚠ Proxy and upstream returned different results (acceptable for changing blockchain state)"
        echo "✓ Compression/decompression test PASSED"
        return 0
    fi
}

# Test a single JSON-RPC query
test_rpc_query() {
    local query="$1"
    local attempt="$2"
    
    echo "--- Test $attempt/$MAX_ATTEMPTS: Testing RPC query"
    
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
        echo "⚠ Query $attempt failed - curl error $curl_exit_code"
        return 1
    fi
    
    if [ -z "$response" ]; then
        echo "⚠ Query $attempt failed - empty response"
        return 1
    fi
    
    # Check if response is binary/compressed
    if is_binary_response "$response"; then
        echo "✗ Query $attempt FAILED - received binary/compressed response!"
        echo "First 100 chars of response: $(echo "$response" | head -c 100)"
        echo "Response contains binary data - this indicates the compression bug is present"
        return 2  # Special return code for compression bug
    fi
    
    # Try to parse as JSON to verify it's valid
    if ! echo "$response" | jq . >/dev/null 2>&1; then
        echo "⚠ Query $attempt failed - invalid JSON response"
        echo "Response: $response"
        return 1
    fi
    
    echo "✓ Query $attempt passed - received valid JSON response"
    return 0
}

run_compression_tests() {
    echo "--- Running RPC proxy compression tests"
    
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
    
    echo "--- Test Results Summary"
    echo "  Total tests: $total_tests"
    echo "  Passed: $passed_tests"
    echo "  Failed (network/other): $failed_tests"
    echo "  Failed (compression bug): $compression_bugs"
    
    if [ $compression_bugs -gt 0 ]; then
        echo "✗ CRITICAL: RPC proxy compression bug detected!"
        echo "The suibase RPC proxy is returning compressed binary data instead of JSON."
        echo "This breaks SuiClient and other JSON-RPC clients."
        echo "This test will FAIL to ensure the bug gets fixed."
        return 1
    fi
    
    if [ $passed_tests -eq 0 ]; then
        echo "✗ All tests failed - RPC proxy appears to be non-functional"
        return 1
    fi
    
    echo "✓ No compression bugs detected - all responses are proper JSON"
    return 0
}

cleanup_test_environment() {
    echo "--- Cleaning up test environment"
    
    # Restore config files
    restore_config_files "$WORKDIR"
    
    # Clean up any port conflicts (stop processes using our ports)
    cleanup_port_conflicts
    
    return 0
}

main() {
    echo "================================"
    echo "$TEST_NAME"
    echo "================================"
    echo "Description: $TEST_DESCRIPTION"
    echo "Workdir: $WORKDIR"
    echo "Proxy Port: $PROXY_PORT"
    echo "Test Queries: ${#TEST_QUERIES[@]}"
    echo ""
    
    # Set up error handling
    trap cleanup_test_environment EXIT
    
    # Run the test sequence
    if ! setup_test_environment; then
        echo "✗ Test environment setup failed"
        exit 1
    fi
    
    if ! run_compression_tests; then
        echo "✗ $TEST_NAME FAILED"
        exit 1
    fi
    
    echo "✓ $TEST_NAME PASSED"
    exit 0
}

# Run the test if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi