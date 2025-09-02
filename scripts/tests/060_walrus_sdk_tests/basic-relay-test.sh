#!/bin/bash

# Test basic Walrus relay connection
# This test verifies that the Suibase upload relay service is accessible
# and responding correctly on testnet.

# Ignore SIGPIPE on macOS to prevent test failures
if [[ "$(uname)" == "Darwin" ]]; then
    trap '' SIGPIPE
    echo "SIGPIPE trap installed"
fi
set -e  # Exit on any error

# Load common test functions (which includes validation and setup)
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"

# Test plan
echo "=== Testing Basic Walrus Relay Connection ==="
echo "Testing: Basic connectivity to Suibase upload relay service"
echo "Workdir: $WORKDIR"
echo

# Main test function
run_basic_relay_test() {
    echo "--- Starting Basic Relay Connection Test ---"

    echo "Using Walrus relay on port: $WALRUS_RELAY_PORT"
    echo "Using Walrus relay proxy on port: $WALRUS_RELAY_PROXY_PORT"

    # Run the TypeScript basic test
    echo "Executing basic relay connection test..."
    local test_output
    local test_exit_code

    # Change to test directory for npm operations
    cd "$script_dir" || exit 1

    # Capture both output and exit code
    if test_output=$(npm run test:basic 2>&1); then
        test_exit_code=0
    else
        test_exit_code=$?
    fi

    # Display the test output
    echo "$test_output"

    # Analyze the results
    if [ $test_exit_code -eq 0 ]; then
        echo "✓ Basic relay connection test PASSED"
        return 0
    else
        echo "✗ Basic relay connection test FAILED (exit code: $test_exit_code)"
        return 1
    fi
}

# Main execution
main() {
    local exit_code=0

    auto_setup_sdk_test_environment "$script_dir"

    # Run the test
    if ! run_basic_relay_test; then
        exit_code=1
    fi

    # Test completed
    if [ $exit_code -eq 0 ]; then
        echo
        echo "=== Basic Relay Test Summary ==="
        echo "✓ SUCCESS: Both relay connection tests passed"
        echo "Both Walrus relay services are accessible and responding correctly"
    else
        echo
        echo "=== Basic Relay Test Summary ==="
        echo "✗ FAILURE: Relay connection tests failed"
        echo "Check the output above for details"
    fi

    exit $exit_code
}

# Run the main function
main "$@"