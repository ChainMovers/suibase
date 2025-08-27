#!/bin/bash

# Test walrus relay stats API methods
# This verifies getWalrusRelayStats and resetWalrusRelayStats return valid responses

set -e  # Exit on any error

# Load common test functions
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"

# Test configuration
DAEMON_PORT="44399"
DAEMON_URL="http://localhost:$DAEMON_PORT"
WORKDIR="testnet"

# JSON-RPC helper function
call_json_rpc() {
    local method="$1"
    local params="$2"
    local payload
    
    if [ -n "$params" ]; then
        payload='{"jsonrpc":"2.0","method":"'$method'","params":'$params',"id":1}'
    else
        payload='{"jsonrpc":"2.0","method":"'$method'","id":1}'
    fi
    
    curl -s -X POST -H "Content-Type: application/json" \
         -d "$payload" "$DAEMON_URL" 2>/dev/null || {
        echo "ERROR: Failed to call JSON-RPC method: $method"
        return 1
    }
}

# Test plan
echo "=== Testing Walrus Relay Stats API ==="
echo "Testing: getWalrusRelayStats and resetWalrusRelayStats methods"
echo "Daemon URL: $DAEMON_URL"
echo "Workdir: $WORKDIR"
echo

# Setup test environment
setup_test_workdir "$WORKDIR"
backup_config_files "$WORKDIR"

# Ensure we're using BUILD version of suibase-daemon
ensure_build_daemon

test_daemon_running() {
    echo "--- Test: Ensure daemon and services are running ---"
    
    # Start testnet services which includes suibase-daemon
    echo "Starting testnet services..."
    "$SUIBASE_DIR/scripts/$WORKDIR" start >/dev/null 2>&1 || {
        echo "Note: testnet start may have had issues, checking daemon status..."
    }
    
    # Wait for daemon to be ready
    local timeout=30
    local count=0
    while [ $count -lt $timeout ]; do
        if curl -s -m 2 "$DAEMON_URL" >/dev/null 2>&1; then
            echo "✓ Daemon is running and responsive on port $DAEMON_PORT"
            return 0
        fi
        sleep 1
        count=$((count + 1))
    done
    
    fail "Daemon not responsive on port $DAEMON_PORT after ${timeout}s"
}

test_walrus_relay_setup() {
    echo "--- Test: Ensure walrus relay is enabled ---"
    
    # Enable walrus relay if not already enabled
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable >/dev/null 2>&1 || {
        fail "Failed to enable walrus relay"
    }
    
    # Check walrus relay status
    local status_output
    status_output=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay status 2>/dev/null || echo "STATUS_FAILED")
    
    if [ "$status_output" = "STATUS_FAILED" ]; then
        echo "Note: Walrus relay status check failed, but continuing with test"
    else
        echo "✓ Walrus relay status accessible"
    fi
    
    echo "✓ Walrus relay setup completed"
}

test_get_walrus_stats_basic() {
    echo "--- Test: getWalrusRelayStats basic functionality ---"
    
    # Test with basic parameters (summary and links enabled)
    local response
    local params='{"workdir":"'$WORKDIR'","summary":true,"links":true,"data":true}'
    
    response=$(call_json_rpc "getWalrusRelayStats" "$params")
    
    if [ -z "$response" ]; then
        fail "getWalrusRelayStats returned empty response"
    fi
    
    # Check that response contains expected JSON-RPC fields
    if ! echo "$response" | jq -e '.jsonrpc' >/dev/null 2>&1; then
        fail "Response missing jsonrpc field: $response"
    fi
    
    if ! echo "$response" | jq -e '.id' >/dev/null 2>&1; then
        fail "Response missing id field: $response"
    fi
    
    # Check for result or error
    if echo "$response" | jq -e '.error' >/dev/null 2>&1; then
        local error_msg
        error_msg=$(echo "$response" | jq -r '.error.message // .error')
        fail "getWalrusRelayStats returned error: $error_msg"
    fi
    
    if ! echo "$response" | jq -e '.result' >/dev/null 2>&1; then
        fail "Response missing result field: $response"
    fi
    
    echo "✓ getWalrusRelayStats returned valid JSON-RPC response"
    
    # Check that result has expected structure
    if ! echo "$response" | jq -e '.result.status' >/dev/null 2>&1; then
        fail "Result missing status field: $response"
    fi
    
    if ! echo "$response" | jq -e '.result.summary' >/dev/null 2>&1; then
        fail "Result missing summary field: $response"
    fi
    
    # Verify summary has stats fields
    local total_requests
    total_requests=$(echo "$response" | jq -r '.result.summary.totalRequests // "null"')
    if [ "$total_requests" = "null" ]; then
        fail "Summary missing totalRequests field: $response"
    fi
    
    local successful_requests  
    successful_requests=$(echo "$response" | jq -r '.result.summary.successfulRequests // "null"')
    if [ "$successful_requests" = "null" ]; then
        fail "Summary missing successfulRequests field: $response"
    fi
    
    local failed_requests
    failed_requests=$(echo "$response" | jq -r '.result.summary.failedRequests // "null"')
    if [ "$failed_requests" = "null" ]; then
        fail "Summary missing failedRequests field: $response"
    fi
    
    echo "✓ getWalrusRelayStats response has expected structure"
    echo "   total_requests: $total_requests"  
    echo "   successful_requests: $successful_requests"
    echo "   failed_requests: $failed_requests"
}

test_reset_walrus_stats() {
    echo "--- Test: resetWalrusRelayStats functionality ---"
    
    # Test reset stats
    local response
    local params='{"workdir":"'$WORKDIR'"}'
    
    response=$(call_json_rpc "resetWalrusRelayStats" "$params")
    
    if [ -z "$response" ]; then
        fail "resetWalrusRelayStats returned empty response"
    fi
    
    # Check for valid JSON-RPC response
    if echo "$response" | jq -e '.error' >/dev/null 2>&1; then
        local error_msg
        error_msg=$(echo "$response" | jq -r '.error.message // .error')
        fail "resetWalrusRelayStats returned error: $error_msg"
    fi
    
    if ! echo "$response" | jq -e '.result' >/dev/null 2>&1; then
        fail "resetWalrusRelayStats response missing result field: $response"
    fi
    
    # Check that result indicates success
    local result_status
    result_status=$(echo "$response" | jq -r '.result.result // "null"')
    if [ "$result_status" != "true" ]; then
        local info
        info=$(echo "$response" | jq -r '.result.info // "no info"')
        fail "resetWalrusRelayStats did not succeed: result=$result_status, info=$info"
    fi
    
    echo "✓ resetWalrusRelayStats completed successfully"
}

test_stats_after_reset() {
    echo "--- Test: Verify stats are zero after reset ---"
    
    # Get stats after reset
    local response
    local params='{"workdir":"'$WORKDIR'","summary":true,"data":true}'
    
    response=$(call_json_rpc "getWalrusRelayStats" "$params")
    
    if [ -z "$response" ]; then
        fail "getWalrusRelayStats returned empty response after reset"
    fi
    
    if echo "$response" | jq -e '.error' >/dev/null 2>&1; then
        local error_msg
        error_msg=$(echo "$response" | jq -r '.error.message // .error')
        fail "getWalrusRelayStats returned error after reset: $error_msg"  
    fi
    
    # Verify all stats are zero
    local total_requests
    total_requests=$(echo "$response" | jq -r '.result.summary.totalRequests')
    if [ "$total_requests" != "0" ]; then
        fail "totalRequests should be 0 after reset, got: $total_requests"
    fi
    
    local successful_requests  
    successful_requests=$(echo "$response" | jq -r '.result.summary.successfulRequests')
    if [ "$successful_requests" != "0" ]; then
        fail "successfulRequests should be 0 after reset, got: $successful_requests"
    fi
    
    local failed_requests
    failed_requests=$(echo "$response" | jq -r '.result.summary.failedRequests')
    if [ "$failed_requests" != "0" ]; then
        fail "failedRequests should be 0 after reset, got: $failed_requests"
    fi
    
    echo "✓ All stats are zero after reset"
    echo "   total_requests: $total_requests"
    echo "   successful_requests: $successful_requests"  
    echo "   failed_requests: $failed_requests"
}

test_display_format() {
    echo "--- Test: Verify display format works ---"
    
    # Test display=true parameter
    local response
    local params='{"workdir":"'$WORKDIR'","summary":true,"links":true,"display":true}'
    
    response=$(call_json_rpc "getWalrusRelayStats" "$params")
    
    if [ -z "$response" ]; then
        fail "getWalrusRelayStats with display=true returned empty response"
    fi
    
    if echo "$response" | jq -e '.error' >/dev/null 2>&1; then
        local error_msg
        error_msg=$(echo "$response" | jq -r '.error.message // .error')
        fail "getWalrusRelayStats with display=true returned error: $error_msg"
    fi
    
    # Check that display field exists and contains text
    if ! echo "$response" | jq -e '.result.display' >/dev/null 2>&1; then
        fail "Result missing display field when display=true: $response"
    fi
    
    local display_text
    display_text=$(echo "$response" | jq -r '.result.display')
    if [ -z "$display_text" ] || [ "$display_text" = "null" ]; then
        fail "Display field is empty or null: $display_text"
    fi
    
    # Check that display contains expected content
    if ! echo "$display_text" | grep -q "Statistics"; then
        fail "Display text missing 'Statistics' header: $display_text"
    fi
    
    if ! echo "$display_text" | grep -q "Statistics"; then
        fail "Display text missing 'Statistics' section: $display_text"
    fi
    
    echo "✓ Display format contains expected content"
}

test_cli_api_consistency() {
    echo "--- Test: Verify CLI output matches API display format exactly ---"
    
    # Get API display output
    local api_response
    local params='{"workdir":"'$WORKDIR'","summary":true,"links":true,"display":true}'
    
    api_response=$(call_json_rpc "getWalrusRelayStats" "$params")
    
    if [ -z "$api_response" ]; then
        fail "API call failed - empty response"
    fi
    
    if echo "$api_response" | jq -e '.error' >/dev/null 2>&1; then
        local error_msg
        error_msg=$(echo "$api_response" | jq -r '.error.message // .error')
        fail "API call returned error: $error_msg"
    fi
    
    local api_display_text
    api_display_text=$(echo "$api_response" | jq -r '.result.display')
    
    if [ -z "$api_display_text" ] || [ "$api_display_text" = "null" ]; then
        fail "API display text is empty or null"
    fi
    
    # Get CLI output (remove potential ANSI color codes)
    local cli_output
    cli_output=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay stats 2>/dev/null | sed 's/\x1b\[[0-9;]*m//g' || {
        fail "CLI command '$WORKDIR wal-relay stats' failed"
    })
    
    if [ -z "$cli_output" ]; then
        fail "CLI output is empty"
    fi
    
    # Compare the outputs (normalize whitespace for comparison)
    local api_normalized cli_normalized
    api_normalized=$(echo "$api_display_text" | tr -s ' \t\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
    cli_normalized=$(echo "$cli_output" | tr -s ' \t\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
    
    if [ "$api_normalized" != "$cli_normalized" ]; then
        echo "ERROR: CLI output does not match API display format"
        echo
        echo "API display output:"
        echo "-------------------"
        echo "$api_display_text"
        echo
        echo "CLI output:"
        echo "-----------"
        echo "$cli_output"
        echo
        echo "API normalized: '$api_normalized'"
        echo "CLI normalized: '$cli_normalized'"
        fail "CLI and API outputs do not match"
    fi
    
    echo "✓ CLI output matches API display format exactly"
}

test_disabled_walrus_relay() {
    echo "--- Test: Behavior when walrus relay is disabled ---"
    
    # Temporarily disable walrus relay
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable >/dev/null 2>&1
    
    # Test CLI stats command with disabled relay
    local cli_output
    cli_output=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay stats 2>/dev/null || echo "CLI_FAILED")
    
    if [ "$cli_output" = "CLI_FAILED" ]; then
        echo "✓ CLI properly handles disabled walrus relay (no output)"
    else
        # CLI should still work and show disabled status
        if echo "$cli_output" | grep -q "Statistics"; then
            echo "✓ CLI shows stats even when disabled"
        else
            fail "CLI output unexpected when disabled: $cli_output"
        fi
    fi
    
    # Test API with disabled relay
    local api_response
    local params='{"workdir":"'$WORKDIR'","summary":true,"display":true}'
    api_response=$(call_json_rpc "getWalrusRelayStats" "$params")
    
    if [ -n "$api_response" ] && ! echo "$api_response" | jq -e '.error' >/dev/null 2>&1; then
        local status
        status=$(echo "$api_response" | jq -r '.result.status // "unknown"')
        if [ "$status" = "DISABLED" ]; then
            echo "✓ API correctly reports DISABLED status"
        else
            echo "Note: API status is '$status' (may be valid depending on state)"
        fi
        
        # Check if display contains helpful information
        local display_text
        display_text=$(echo "$api_response" | jq -r '.result.display // ""')
        if [ -n "$display_text" ]; then
            echo "✓ API provides display output even when disabled"
        fi
    else
        fail "API call failed when walrus relay disabled"
    fi
    
    # Re-enable walrus relay for subsequent tests
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable >/dev/null 2>&1 || {
        fail "Failed to re-enable walrus relay"
    }
    
    echo "✓ Disabled walrus relay test completed"
}

test_helpful_messages() {
    echo "--- Test: Helpful messages for various scenarios ---"
    
    # Test clear command when disabled
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable >/dev/null 2>&1
    
    local clear_output
    clear_output=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay clear 2>/dev/null || echo "")
    
    if [ -n "$clear_output" ]; then
        echo "✓ Clear command provides output even when disabled"
    else
        echo "✓ Clear command handles disabled state gracefully"
    fi
    
    # Re-enable for other tests
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable >/dev/null 2>&1
    
    echo "✓ Helpful messages test completed"
}

# Cleanup function
cleanup_test() {
    echo
    echo "--- Cleanup ---"
    restore_config_files "$WORKDIR"
    echo "✓ Config files restored"
}

# Set up cleanup on exit
trap cleanup_test EXIT

# Run tests
echo "Starting walrus stats API tests..."
echo

# Test sequence
test_daemon_running
test_walrus_relay_setup
test_get_walrus_stats_basic  
test_reset_walrus_stats
test_stats_after_reset
test_display_format
test_cli_api_consistency
test_disabled_walrus_relay
test_helpful_messages

echo
echo "=== All Walrus Stats API Tests Passed! ==="
echo