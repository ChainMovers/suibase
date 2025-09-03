#!/bin/bash

# Test that Walrus RPC operations use the suibase-daemon proxy
# This test verifies that twalrus commands increment the RPC proxy statistics,
# proving that they go through the suibase-daemon proxy rather than direct to RPC endpoints.

# Ignore SIGPIPE on macOS to prevent test failures
if [[ "$(uname)" == "Darwin" ]]; then
    trap '' SIGPIPE
    echo "SIGPIPE trap installed"
fi
set -e  # Exit on any error

# Load common test functions
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"

# Test plan
echo "=== Testing Walrus RPC Proxy Usage ==="
echo "Testing: twalrus commands use suibase-daemon RPC proxy"
echo "Workdir: $WORKDIR"
echo

# Only run this test on testnet workdir
if [[ "$WORKDIR" != "testnet" ]]; then
    echo "SKIP: This test only runs on testnet workdir, current: $WORKDIR"
    exit 2
fi

# Skip if CI_WORKDIR is not testnet (for CI environments)
if [[ -n "$CI_WORKDIR" && "$CI_WORKDIR" != "testnet" ]]; then
    echo "SKIP: CI_WORKDIR is '$CI_WORKDIR', this test only supports testnet"
    exit 2
fi

# Setup test environment
setup_clean_environment
setup_test_workdir "$WORKDIR"
backup_config_files "$WORKDIR"

# Main test function
run_walrus_rpc_proxy_test() {
    echo "--- Starting Walrus RPC Proxy Test ---"

    # Start testnet workdir
    echo "Starting testnet workdir..."
    ~/suibase/scripts/testnet start >/dev/null 2>&1

    # Wait for daemon to be fully ready
    wait_for_daemon_running 15 true

    # Get initial RPC stats
    echo "Getting initial RPC statistics..."
    local initial_stats_json
    initial_stats_json=$(~/suibase/scripts/testnet links --json 2>/dev/null)

    if [[ -z "$initial_stats_json" ]]; then
        echo "ERROR: Failed to get initial RPC statistics"
        return 1
    fi

    # Extract initial successOnFirstAttempt count
    local initial_success_count
    initial_success_count=$(echo "$initial_stats_json" | jq -r '.result.summary.successOnFirstAttempt // 0' 2>/dev/null)

    if [[ -z "$initial_success_count" || "$initial_success_count" == "null" ]]; then
        echo "ERROR: Failed to parse initial successOnFirstAttempt count"
        echo "Stats JSON: $initial_stats_json"
        return 1
    fi

    echo "Initial successOnFirstAttempt count: $initial_success_count"

    # Execute twalrus info command
    echo "Executing 'twalrus info' command..."
    local twalrus_output
    local twalrus_exit_code

    # Capture twalrus output and exit code
    if twalrus_output=$(~/suibase/scripts/twalrus info 2>&1); then
        twalrus_exit_code=0
    else
        twalrus_exit_code=$?
    fi

    echo "twalrus command exit code: $twalrus_exit_code"

    # Check if twalrus succeeded (it should work even if we're just getting info)
    if [[ $twalrus_exit_code -ne 0 ]]; then
        echo "ERROR: twalrus info command failed (exit code: $twalrus_exit_code)"
        echo "Output: $twalrus_output"
        return 1
    else
        echo "✓ twalrus info command completed successfully"
    fi

    # Check the output has a line starting with "Current epoch"
    if ! echo "$twalrus_output" | grep -q "^Current epoch:"; then
        echo "ERROR: twalrus info command output is missing 'Current epoch' line"
        echo "Expected output should contain walrus system information"
        return 1
    fi

    # Wait a brief moment for stats to update
    sleep 2

    # Get final RPC stats
    echo "Getting final RPC statistics..."
    local final_stats_json
    final_stats_json=$(~/suibase/scripts/testnet links --json 2>/dev/null)

    if [[ -z "$final_stats_json" ]]; then
        echo "ERROR: Failed to get final RPC statistics"
        return 1
    fi

    # Extract final successOnFirstAttempt count
    local final_success_count
    final_success_count=$(echo "$final_stats_json" | jq -r '.result.summary.successOnFirstAttempt // 0' 2>/dev/null)

    if [[ -z "$final_success_count" || "$final_success_count" == "null" ]]; then
        echo "ERROR: Failed to parse final successOnFirstAttempt count"
        echo "Stats JSON: $final_stats_json"
        return 1
    fi

    echo "Final successOnFirstAttempt count: $final_success_count"

    # Calculate the difference
    local success_increment=$((final_success_count - initial_success_count))
    echo "successOnFirstAttempt increment: $success_increment"

    # Verify that the count incremented (proving RPC calls went through proxy)
    if [[ $success_increment -gt 0 ]]; then
        echo "✓ SUCCESS: RPC proxy usage verified (increment: $success_increment)"
        echo "  twalrus operations are using the suibase-daemon RPC proxy"
        return 0
    else
        echo "✗ FAILURE: No RPC proxy increment detected"
        echo "  This suggests twalrus is not using the suibase-daemon RPC proxy"
        echo "  Initial count: $initial_success_count"
        echo "  Final count: $final_success_count"
        return 1
    fi
}

# Cleanup is handled automatically by __test_common.sh

# Main execution
main() {
    local exit_code

    # Run the test and capture its exit code
    run_walrus_rpc_proxy_test
    exit_code=$?

    # Test completed
    if [ $exit_code -eq 0 ]; then
        echo
        echo "=== Walrus RPC Proxy Test Summary ==="
        echo "✓ SUCCESS: Test passed"
        echo "The twalrus command is correctly using the suibase-daemon RPC proxy"
    else
        echo
        echo "=== Walrus RPC Proxy Test Summary ==="
        echo "✗ FAILURE: Test failed"
        echo "Check the output above for details"
    fi

    exit $exit_code
}

# Run the main function
main "$@"