#!/bin/bash

# Test Walrus SDK upload integration with Suibase upload relay
# This test verifies that the Mysten Labs Walrus SDK can successfully upload blobs
# through the local Suibase upload relay service on testnet.

# Ignore SIGPIPE on macOS to prevent test failures
if [[ "$(uname)" == "Darwin" ]]; then
    trap '' SIGPIPE
    echo "SIGPIPE trap installed"
fi
set -e  # Exit on any error

# Load common test functions (which includes validation and setup)
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Export SCRIPT_DIR globally for reliable cleanup
export SCRIPT_DIR="$script_dir"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"

# Test plan
echo "=== Testing Walrus SDK Upload Integration ==="
echo "Testing: Walrus SDK blob upload through Suibase upload relay"
echo "Workdir: $WORKDIR"
echo "Using active address from Suibase keystore"
echo

# Main test function
run_walrus_sdk_test() {
    echo "--- Starting Walrus SDK Upload Test ---"
    
    # Auto-setup environment (happens once on first call)
    auto_setup_sdk_test_environment "$script_dir"
    
    echo "Using Walrus relay on port: $WALRUS_RELAY_PORT"
    
    # Run the TypeScript test
    echo "Executing TypeScript test..."
    local test_output
    local test_exit_code
    
    # Capture both output and exit code
    if test_output=$(run_typescript_test "$script_dir" 2>&1); then
        test_exit_code=0
    else
        test_exit_code=$?
    fi
    
    # Display the test output
    echo "$test_output"
    
    # Analyze the results
    if [ $test_exit_code -eq 0 ]; then
        echo "✓ Walrus SDK upload test PASSED"
        return 0
    elif [ $test_exit_code -eq 2 ]; then
        echo "○ Walrus SDK upload test SKIPPED (exit code: $test_exit_code)"
        return 2
    else
        echo "✗ Walrus SDK upload test FAILED (exit code: $test_exit_code)"
        return 1
    fi
}

# Cleanup is handled automatically by __test_common.sh

# Main execution
main() {
    local exit_code
    
    # Run the test and capture its exit code
    run_walrus_sdk_test
    exit_code=$?
    
    # Test completed
    if [ $exit_code -eq 0 ]; then
        echo
        echo "=== Walrus SDK Test Summary ==="
        echo "✓ SUCCESS: All tests passed"
        echo "The Walrus SDK successfully uploaded a blob through the Suibase upload relay"
    elif [ $exit_code -eq 2 ]; then
        echo
        echo "=== Walrus SDK Test Summary ==="
        echo "○ SKIPPED: Test was skipped"
        echo "Check the output above for skip reason (insufficient balance, missing active address, etc.)"
    else
        echo
        echo "=== Walrus SDK Test Summary ==="
        echo "✗ FAILURE: Test failed"
        echo "Check the output above for details"
    fi
    
    exit $exit_code
}

# Run the main function
main "$@"