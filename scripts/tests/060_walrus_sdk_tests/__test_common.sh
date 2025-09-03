#!/bin/bash

# Common code for Walrus SDK TypeScript test scripts.
# Extends 050_walrus_tests/__test_common.sh with Node.js/TypeScript specific functionality

# Load the base walrus test common functions first
WALRUS_TESTS_COMMON_DIR="$HOME/suibase/scripts/tests/050_walrus_tests"
# shellcheck source=SCRIPTDIR/../050_walrus_tests/__test_common.sh
source "$WALRUS_TESTS_COMMON_DIR/__test_common.sh"

# SDK-specific validation - only testnet is supported
validate_sdk_workdir_support() {
    case "$WORKDIR" in
        "testnet")
            # Only testnet is supported for SDK tests
            return 0
            ;;
        *)
            echo "SKIP: Walrus SDK tests only support testnet workdir, got: $WORKDIR"
            exit 2
            ;;
    esac
}


# Get the walrus relay port for the current workdir
get_walrus_relay_port() {
    local workdir="$1"
    local port
    port=$("$WALRUS_TESTS_COMMON_DIR/utils/__get-walrus-relay-local-port.sh" "$workdir" 2>&1 || echo "UNKNOWN")

    if [ "$port" = "UNKNOWN" ]; then
        echo "ERROR: Failed to get walrus relay port for workdir '$workdir'"
        echo "Please ensure walrus_relay_local_port is set in $HOME/suibase/workdirs/$workdir/suibase.yaml"
        exit 1
    fi

    echo "$port"
}

# Get the walrus relay proxy port (suibase-daemon) for the current workdir
get_walrus_relay_proxy_port() {
    local workdir="$1"
    local port
    port=$("$SUIBASE_DIR/scripts/dev/show-config" "$workdir" 2>/dev/null | grep "^CFG_walrus_relay_proxy_port=" | cut -d= -f2)

    if [ -z "$port" ] || [ "$port" = "~" ]; then
        echo "ERROR: Failed to get walrus relay proxy port for workdir '$workdir'"
        echo "Please ensure walrus_relay_proxy_port is set in the configuration"
        exit 1
    fi

    echo "$port"
}

# Check if Node.js is available and meets minimum version requirement
check_node_version() {
    if ! command -v node >/dev/null 2>&1; then
        echo "SKIP: Node.js is not installed or not in PATH"
        echo "Please install Node.js version 18.0.0 or higher to run SDK tests"
        exit 2
    fi

    local node_version
    node_version=$(node --version 2>/dev/null | sed 's/^v//')
    local major_version
    major_version=$(echo "$node_version" | cut -d. -f1)

    if [ "$major_version" -lt 18 ]; then
        echo "SKIP: Node.js version $node_version is too old"
        echo "Please install Node.js version 18.0.0 or higher to run SDK tests"
        exit 2
    fi

    echo "✓ Node.js version $node_version is available"
}

# Check if npm is available
check_npm() {
    if ! command -v npm >/dev/null 2>&1; then
        echo "SKIP: npm is not installed or not in PATH"
        echo "Please install npm (usually comes with Node.js) to run SDK tests"
        exit 2
    fi
    echo "✓ npm is available"
}

# Install Node.js dependencies if needed
install_node_dependencies() {
    local test_dir="$1"

    if [ ! -f "$test_dir/package.json" ]; then
        echo "ERROR: package.json not found in $test_dir"
        exit 1
    fi

    # Change to test directory for npm operations
    cd "$test_dir" || exit 1

    # Check if node_modules exists and is up to date
    if [ ! -d "node_modules" ] || [ "package.json" -nt "node_modules" ]; then
        echo "Installing Node.js dependencies..."
        if ! npm install --silent >/dev/null 2>&1; then
            echo "ERROR: Failed to install Node.js dependencies"
            exit 1
        fi
        echo "✓ Node.js dependencies installed"
    else
        echo "✓ Node.js dependencies are up to date"
    fi
}

# Build TypeScript code
build_typescript() {
    local test_dir="$1"

    cd "$test_dir" || exit 1

    echo "Building TypeScript code..."
    if ! npm run build >/dev/null 2>&1; then
        echo "ERROR: Failed to build TypeScript code"
        exit 1
    fi
    echo "✓ TypeScript code built successfully"
}

# Run the TypeScript test
run_typescript_test() {
    local test_dir="$1"

    cd "$test_dir" || exit 1

    echo "Running TypeScript test..."

    # Export environment variables needed by the TypeScript test
    export WORKDIR
    # WALRUS_RELAY_PORT and WALRUS_RELAY_PROXY_PORT are already exported globally

    # Run the test and capture output
    npm run test 2>&1
    local exit_code=$?

    return $exit_code
}

# Setup complete test environment for SDK tests (idempotent)
setup_sdk_test_environment() {
    local test_dir="$1"

    # Skip if already set up
    if [ "$SDK_TEST_ENVIRONMENT_READY" = "true" ]; then
        echo "✓ SDK test environment already ready"
        return 0
    fi

    # Run all validation checks
    validate_sdk_workdir_support
    check_node_version
    check_npm

    # Setup environment but skip port conflicts cleanup to avoid killing walrus relay
    # Do the same cleanup as setup_clean_environment but without cleanup_port_conflicts

    # Clean up stale walrus locks that may have been left by interrupted processes
    local _WALRUS_LOCK="/tmp/.suibase/cli-walrus.lock"
    if [ -d "$_WALRUS_LOCK" ]; then
        if ! lsof "$_WALRUS_LOCK" >/dev/null 2>&1; then
            echo "Removing stale walrus lock directory: $_WALRUS_LOCK"
            rmdir "$_WALRUS_LOCK" 2>/dev/null || true
        fi
    fi

    # Remove any previous test temp directories
    rm -rf /tmp/suibase_walrus_relay_test_* 2>/dev/null || true

    # Setup workdir and backup configs
    setup_test_workdir "$WORKDIR"
    backup_config_files "$WORKDIR"

    # Start daemon and all services (idempotent command)
    echo "Starting suibase services (idempotent)..."
    "$SUIBASE_DIR/scripts/$WORKDIR" start >/dev/null 2>&1
    
    # Wait for daemon to be ready
    wait_for_daemon_running 15 true

    # Setup Node.js environment
    install_node_dependencies "$test_dir"
    build_typescript "$test_dir"

    # Mark as ready
    SDK_TEST_ENVIRONMENT_READY=true
    echo "✓ SDK test environment setup complete"
}

# Cleanup SDK test environment
cleanup_sdk_test_environment() {
    local test_dir="$1"

    # Clean up TypeScript build artifacts
    if [ -d "$test_dir/dist" ]; then
        rm -rf "$test_dir/dist" 2>/dev/null || true
    fi

    # Call base cleanup but skip port conflicts cleanup to avoid stopping walrus relay
    restore_config_files "$WORKDIR"
    # NOTE: Not calling cleanup_port_conflicts to avoid stopping walrus relay between tests

    echo "✓ SDK test environment cleaned up"
}

# Note: Cleanup is handled automatically via EXIT trap (cleanup_sdk_test)

# Call SDK validation early
validate_sdk_workdir_support

# Get and export both walrus relay ports for all tests to use
WALRUS_RELAY_PORT=$(get_walrus_relay_port "$WORKDIR")
export WALRUS_RELAY_PORT
echo "✓ Walrus relay local port: $WALRUS_RELAY_PORT"

WALRUS_RELAY_PROXY_PORT=$(get_walrus_relay_proxy_port "$WORKDIR")
export WALRUS_RELAY_PROXY_PORT
echo "✓ Walrus relay proxy port: $WALRUS_RELAY_PROXY_PORT"

# Flag to track if setup has been done (will be set to true after first setup)
SDK_TEST_ENVIRONMENT_READY=false

# Automatically call setup on first test function call
auto_setup_sdk_test_environment() {
    local test_dir="$1"
    if [ "$SDK_TEST_ENVIRONMENT_READY" = "false" ]; then
        setup_sdk_test_environment "$test_dir"
    fi
}

# Generic cleanup function that can be used as EXIT trap  
cleanup_sdk_test() {
    echo "Cleaning up test environment..."
    # Use the global SCRIPT_DIR variable set by the main test script
    local script_dir="${SCRIPT_DIR:-$(pwd)}"
    cleanup_sdk_test_environment "$script_dir"
}

# Automatically set up EXIT trap for all SDK tests
trap cleanup_sdk_test EXIT

# Export new functions
export -f validate_sdk_workdir_support get_walrus_relay_port get_walrus_relay_proxy_port
export -f check_node_version check_npm install_node_dependencies build_typescript run_typescript_test
export -f auto_setup_sdk_test_environment cleanup_sdk_test
export -f setup_sdk_test_environment cleanup_sdk_test_environment