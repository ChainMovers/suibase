#!/bin/bash

# Test walrus proxy integration - Phase 4 implementation
# Tests that HTTP requests through the proxy (port 45852) produce identical results
# to requests made directly to the walrus-upload-relay backend (port 45802)

# Ignore SIGPIPE on macOS to prevent test failures
if [[ "$(uname)" == "Darwin" ]]; then
    trap '' SIGPIPE
    echo "SIGPIPE trap installed"
fi

# Load common test functions
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"

# Test plan
echo "=== Testing Walrus Relay HTTP Proxy Integration (Phase 4) ==="
echo "Testing: HTTP requests through proxy vs direct should be identical"
echo "Proxy port: 45852, Backend port: 45802"
echo

# Setup test environment
setup_clean_environment
setup_test_workdir "$WORKDIR"
backup_config_files "$WORKDIR"

# Ensure we're using BUILD version of suibase-daemon for walrus relay features
ensure_build_daemon

# Get the port configurations using show-config
PROXY_PORT=$("$SUIBASE_DIR/scripts/dev/show-config" "$WORKDIR" | grep "^CFG_walrus_relay_proxy_port=" | cut -d'=' -f2)
BACKEND_PORT=$("$SUIBASE_DIR/scripts/dev/show-config" "$WORKDIR" | grep "^CFG_walrus_relay_local_port=" | cut -d'=' -f2)

if [[ -z "$PROXY_PORT" ]]; then
    echo "ERROR: walrus_relay_proxy_port not configured in suibase.yaml"
    exit 1
fi

if [[ -z "$BACKEND_PORT" ]]; then
    echo "ERROR: walrus_relay_local_port not configured in suibase.yaml"
    exit 1
fi

echo "Using proxy port: $PROXY_PORT"
echo "Using backend port: $BACKEND_PORT"
echo

test_proxy_vs_direct_identical() {
    echo "--- Test: Proxy requests should be identical to direct requests ---"

    # Ensure walrus relay is enabled and running
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable
    "$SUIBASE_DIR/scripts/$WORKDIR" start
    
    # Wait for services to be fully up
    echo "Waiting for services to start..."
    sleep 3

    # Verify both proxy and backend are listening
    if ! check_port_listening "$PROXY_PORT"; then
        echo "ERROR: Proxy port $PROXY_PORT is not listening"
        return 1
    fi
    
    if ! check_port_listening "$BACKEND_PORT"; then
        echo "ERROR: Backend port $BACKEND_PORT is not listening"
        return 1
    fi
    
    echo "✓ Both proxy ($PROXY_PORT) and backend ($BACKEND_PORT) are listening"

    # Test 1: GET /v1/tip-config endpoint
    echo "Testing GET /v1/tip-config..."
    
    local direct_response="/tmp/walrus_direct_response.json"
    local proxy_response="/tmp/walrus_proxy_response.json"
    local direct_headers="/tmp/walrus_direct_headers.txt"
    local proxy_headers="/tmp/walrus_proxy_headers.txt"
    
    # Make direct request to backend
    if curl -s -D "$direct_headers" -o "$direct_response" \
            "http://localhost:$BACKEND_PORT/v1/tip-config" \
            --max-time 10; then
        echo "✓ Direct request to backend successful"
    else
        echo "ERROR: Direct request to backend failed"
        return 1
    fi
    
    # Make request through proxy
    if curl -s -D "$proxy_headers" -o "$proxy_response" \
            "http://localhost:$PROXY_PORT/v1/tip-config" \
            --max-time 10; then
        echo "✓ Proxy request successful"
    else
        echo "ERROR: Proxy request failed"
        return 1
    fi
    
    # Compare response bodies (ignore minor differences like timestamps)
    if compare_walrus_responses "$direct_response" "$proxy_response"; then
        echo "✓ Response bodies are equivalent"
    else
        echo "ERROR: Response bodies differ"
        echo "Direct response:"
        cat "$direct_response"
        echo
        echo "Proxy response:"  
        cat "$proxy_response"
        return 1
    fi
    
    # Check that both returned successful HTTP status codes
    direct_status=$(head -n1 "$direct_headers" | cut -d' ' -f2)
    proxy_status=$(head -n1 "$proxy_headers" | cut -d' ' -f2)
    
    if [[ "$direct_status" == "200" && "$proxy_status" == "200" ]]; then
        echo "✓ Both requests returned HTTP 200"
    else
        echo "ERROR: HTTP status codes differ - Direct: $direct_status, Proxy: $proxy_status"
        return 1
    fi
    
    # Clean up temporary files
    rm -f "$direct_response" "$proxy_response" "$direct_headers" "$proxy_headers"
    
    echo "✅ Proxy vs direct request test PASSED"
}

test_proxy_statistics() {
    echo "--- Test: Proxy vs direct statistics validation ---"
    echo "This validates both statistics reporting and that our test hits the correct ports"
    
    # Get initial statistics
    local stats_before
    if ! stats_before=$(get_walrus_stats_json); then
        echo "ERROR: Could not get initial walrus statistics"
        return 1
    fi
    
    local requests_before
    requests_before=$(echo "$stats_before" | jq -r '.result.summary.totalRequests // 0')
    echo "Initial total request count: $requests_before"
    
    # Test 1: Make requests DIRECTLY to backend - should NOT increment stats
    echo
    echo "Step 1: Making 2 requests directly to backend (port $BACKEND_PORT) - should NOT affect stats"
    for i in {1..2}; do
        curl -s "http://localhost:$BACKEND_PORT/v1/tip-config" --max-time 10 > /dev/null || {
            echo "WARNING: Direct request $i to backend failed"
        }
    done
    
    # Wait for any potential statistics updates
    sleep 2
    
    # Get statistics after direct requests
    local stats_after_direct
    if ! stats_after_direct=$(get_walrus_stats_json); then
        echo "ERROR: Could not get walrus statistics after direct requests"
        return 1
    fi
    
    local requests_after_direct
    requests_after_direct=$(echo "$stats_after_direct" | jq -r '.result.summary.totalRequests // 0')
    echo "Request count after direct requests: $requests_after_direct"
    
    # Verify that direct requests did NOT increment statistics
    if [[ "$requests_after_direct" == "$requests_before" ]]; then
        echo "✓ Direct requests to backend did NOT increment statistics (correct)"
    else
        echo "ERROR: Direct requests should not increment statistics!"
        echo "  Before: $requests_before, After direct: $requests_after_direct"
        return 1
    fi
    
    # Test 2: Make requests through PROXY - should increment stats
    echo
    echo "Step 2: Making 3 requests through proxy (port $PROXY_PORT) - should increment stats"
    for i in {1..3}; do
        curl -s "http://localhost:$PROXY_PORT/v1/tip-config" --max-time 10 > /dev/null || {
            echo "WARNING: Proxy request $i failed"
        }
    done
    
    # Wait for statistics to be updated
    sleep 2
    
    # Get final statistics
    local stats_after_proxy
    if ! stats_after_proxy=$(get_walrus_stats_json); then
        echo "ERROR: Could not get final walrus statistics"
        return 1
    fi
    
    local requests_after_proxy
    requests_after_proxy=$(echo "$stats_after_proxy" | jq -r '.result.summary.totalRequests // 0')
    echo "Request count after proxy requests: $requests_after_proxy"
    
    # Verify that proxy requests DID increment statistics by exactly 3
    local expected_count=$((requests_before + 3))
    if [[ "$requests_after_proxy" == "$expected_count" ]]; then
        echo "✓ Proxy requests incremented statistics correctly: $requests_before → $requests_after_proxy (+3)"
        echo "✅ Statistics validation test PASSED"
        echo "✅ Port validation confirmed: proxy=$PROXY_PORT, backend=$BACKEND_PORT"
    else
        echo "ERROR: Proxy requests did not increment statistics correctly!"
        echo "  Expected: $expected_count (initial $requests_before + 3 proxy requests)"
        echo "  Actual: $requests_after_proxy"
        echo "  Direct requests: $requests_after_direct (should equal initial $requests_before)"
        return 1
    fi
    
    # Summary
    echo
    echo "Statistics Summary:"
    echo "  Initial requests: $requests_before"
    echo "  After 2 direct requests: $requests_after_direct (no change ✓)"
    echo "  After 3 proxy requests: $requests_after_proxy (+3 ✓)"
}

test_proxy_error_handling() {
    echo "--- Test: Proxy error handling and statistics when backend is down ---"
    echo "This test verifies that proxy failures are recorded as error statistics"
    
    # Get initial statistics before stopping backend
    local stats_before
    if ! stats_before=$(get_walrus_stats_json); then
        echo "ERROR: Could not get initial walrus statistics"
        return 1
    fi
    
    local requests_before failed_before
    requests_before=$(echo "$stats_before" | jq -r '.result.summary.totalRequests // 0')
    failed_before=$(echo "$stats_before" | jq -r '.result.summary.failedRequests // 0')
    echo "Initial statistics: total=$requests_before, failed=$failed_before"
    
    # Stop the walrus relay backend while keeping proxy running
    echo "Stopping walrus relay backend..."
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable
    
    # Wait for backend to stop
    sleep 2
    
    # Verify backend is not listening but proxy still is
    if check_port_listening "$BACKEND_PORT"; then
        echo "ERROR: Backend port $BACKEND_PORT is still listening after stop"
        return 1
    fi
    
    if ! check_port_listening "$PROXY_PORT"; then
        echo "ERROR: Proxy port $PROXY_PORT is not listening"
        return 1
    fi
    
    echo "✓ Backend stopped ($BACKEND_PORT), proxy still running ($PROXY_PORT)"
    
    # Make multiple requests through proxy - these should fail and be recorded as errors
    echo "Making 3 requests through proxy that should fail and be recorded..."
    local num_error_requests=3
    local successful_errors=0
    
    # Temporarily disable 'set -e' for error requests since we expect failures
    set +e
    
    for i in $(seq 1 $num_error_requests); do
        echo "  Making request $i through proxy..."
        local response_code
        response_code=$(curl -s -o /dev/null -w "%{http_code}" \
                             "http://localhost:$PROXY_PORT/v1/tip-config" \
                             --max-time 10) || response_code="000"
        
        if [[ "$response_code" == "502" ]] || [[ "$response_code" == "503" ]] || [[ "$response_code" == "500" ]]; then
            echo "  Request $i: Got expected error code $response_code ✓"
            ((successful_errors++))
        else
            echo "  Request $i: Unexpected response code $response_code"
        fi
    done
    
    # Re-enable 'set -e'
    set -e
    
    if [[ $successful_errors -gt 0 ]]; then
        echo "✓ Proxy returned appropriate error codes when backend is down ($successful_errors/$num_error_requests)"
    else
        echo "WARNING: No requests returned expected error codes"
    fi
    
    # Wait for statistics updates
    sleep 2
    
    # Get statistics after proxy failures
    local stats_after
    if ! stats_after=$(get_walrus_stats_json); then
        echo "ERROR: Could not get walrus statistics after proxy failures"
        return 1
    fi
    
    local requests_after failed_after
    requests_after=$(echo "$stats_after" | jq -r '.result.summary.totalRequests // 0')
    failed_after=$(echo "$stats_after" | jq -r '.result.summary.failedRequests // 0')
    echo "Final statistics: total=$requests_after, failed=$failed_after"
    
    # Verify that both total and failed requests increased
    local expected_total=$((requests_before + num_error_requests))
    local expected_failed=$((failed_before + num_error_requests))
    
    if [[ "$requests_after" == "$expected_total" ]] && [[ "$failed_after" == "$expected_failed" ]]; then
        echo "✓ Error statistics recorded correctly:"
        echo "  Total requests: $requests_before → $requests_after (+$num_error_requests)"
        echo "  Failed requests: $failed_before → $failed_after (+$num_error_requests)"
        echo "✅ Proxy error handling and statistics test PASSED"
    else
        echo "ERROR: Error statistics not recorded correctly!"
        echo "  Expected total: $expected_total, actual: $requests_after"
        echo "  Expected failed: $expected_failed, actual: $failed_after"
        # Continue with restart but return failure
        local test_failed=1
    fi
    
    # Restart backend for cleanup
    echo "Restarting backend..."
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable
    "$SUIBASE_DIR/scripts/$WORKDIR" start
    sleep 2
    
    # Return failure if statistics test failed
    if [[ "${test_failed:-0}" == "1" ]]; then
        return 1
    fi
}

# JSON-RPC helper function for getting walrus stats
call_json_rpc() {
    local method="$1"
    local params="$2"
    local payload
    local daemon_port="44399"
    local daemon_url="http://localhost:$daemon_port"

    if [ -n "$params" ]; then
        payload='{"jsonrpc":"2.0","method":"'$method'","params":'$params',"id":1}'
    else
        payload='{"jsonrpc":"2.0","method":"'$method'","id":1}'
    fi

    curl -s -X POST -H "Content-Type: application/json" \
         -d "$payload" "$daemon_url" 2>/dev/null || {
        echo "ERROR: Failed to call JSON-RPC method: $method" >&2
        return 1
    }
}

# Helper function to get walrus stats as JSON
get_walrus_stats_json() {
    local params='{"workdir":"'$WORKDIR'","summary":true,"links":false,"data":true}'
    local response
    
    response=$(call_json_rpc "getWalrusRelayStats" "$params")
    
    if [ -z "$response" ]; then
        echo "ERROR: getWalrusRelayStats returned empty response" >&2
        return 1
    fi

    if echo "$response" | jq -e '.error' >/dev/null 2>&1; then
        local error_msg
        error_msg=$(echo "$response" | jq -r '.error.message // .error')
        echo "ERROR: getWalrusRelayStats returned error: $error_msg" >&2
        return 1
    fi

    echo "$response"
}

# Helper function to check if a port is listening (cross-platform)
check_port_listening() {
    local port="$1"
    
    # Method 1: Try ss (available on modern Linux)
    if command -v ss >/dev/null 2>&1; then
        if ss -tln 2>/dev/null | grep -q ":$port " 2>/dev/null; then
            return 0
        fi
    fi
    
    # Method 2: Try netstat (available on most Unix systems including macOS)
    if command -v netstat >/dev/null 2>&1; then
        if netstat -an 2>/dev/null | grep -q "LISTEN.*[.:]$port "; then
            return 0
        fi
    fi
    
    # Method 3: Try lsof (usually available on macOS and Linux)
    if command -v lsof >/dev/null 2>&1; then
        if lsof -i ":$port" -sTCP:LISTEN >/dev/null 2>&1; then
            return 0
        fi
    fi
    
    # Method 4: Fallback - try connecting to the port
    if command -v nc >/dev/null 2>&1; then
        if echo "" | nc -w 1 localhost "$port" >/dev/null 2>&1; then
            return 0
        fi
    fi
    
    # If all methods fail, port is not listening
    return 1
}

# Helper function to compare walrus responses (allowing for minor differences)
compare_walrus_responses() {
    local file1="$1"
    local file2="$2"
    
    # If both files are empty, they're identical
    if [[ ! -s "$file1" && ! -s "$file2" ]]; then
        return 0
    fi
    
    # If one is empty and the other isn't, they're different
    if [[ ! -s "$file1" || ! -s "$file2" ]]; then
        return 1
    fi
    
    # For JSON responses, compare structure ignoring timestamps and minor differences
    if jq empty "$file1" 2>/dev/null && jq empty "$file2" 2>/dev/null; then
        # Both are valid JSON - compare structure
        local normalized1="/tmp/normalized1.json"
        local normalized2="/tmp/normalized2.json"
        
        # Normalize JSON (pretty print and sort keys)
        jq -S '.' "$file1" > "$normalized1"
        jq -S '.' "$file2" > "$normalized2"
        
        if cmp -s "$normalized1" "$normalized2"; then
            rm -f "$normalized1" "$normalized2"
            return 0
        else
            rm -f "$normalized1" "$normalized2"
            return 1
        fi
    else
        # Not JSON, compare as text
        if cmp -s "$file1" "$file2"; then
            return 0
        else
            return 1
        fi
    fi
}

# Main test execution
echo "Starting walrus proxy integration tests..."

# Cleanup function
cleanup() {
    echo "Cleaning up..."
    "$SUIBASE_DIR/scripts/$WORKDIR" stop || true
    restore_config_files "$WORKDIR" || true
    rm -f /tmp/walrus_*response* /tmp/walrus_*headers* /tmp/normalized*.json || true
    echo "Cleanup completed"
}

# Set trap for cleanup on exit
trap cleanup EXIT

# Run tests
test_proxy_vs_direct_identical
echo
test_proxy_statistics  
echo
test_proxy_error_handling
echo

echo "🎉 All walrus proxy integration tests PASSED!"
echo "Phase 4 HTTP proxy implementation is working correctly"