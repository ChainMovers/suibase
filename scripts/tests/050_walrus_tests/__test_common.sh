#!/bin/bash

# Common code for walrus relay test scripts.
# Tests for walrus-upload-relay binary management and process lifecycle

SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/common/__globals.sh
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="testnet"  # Default to testnet for walrus relay tests

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"

# Source globals
# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"
trap cleanup EXIT

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

# Test configuration
TEMP_TEST_DIR="/tmp/suibase_walrus_relay_test_$$"

# Test helper functions
setup_test_workdir() {
    local workdir="$1"
    local config_dir="$WORKDIRS/$workdir/config-default"
    
    # Proactively clean up any stale walrus locks from previous test runs
    # This prevents the "stale lock" issue that blocks walrus binary downloads
    local _WALRUS_LOCK="/tmp/.suibase/cli-walrus.lock"
    if [ -d "$_WALRUS_LOCK" ]; then
        # Check if any process is actually using the lock
        if ! lsof "$_WALRUS_LOCK" >/dev/null 2>&1; then
            echo "Removing stale walrus lock directory from previous test run: $_WALRUS_LOCK"
            rmdir "$_WALRUS_LOCK" 2>/dev/null || true
        fi
    fi
    
    # Ensure workdir structure exists
    mkdir -p "$config_dir"
    mkdir -p "$WORKDIRS/$workdir/bin"
    
    # Create a basic suibase.yaml if it doesn't exist
    if [ ! -f "$WORKDIRS/$workdir/suibase.yaml" ]; then
        echo "# Test suibase.yaml for walrus relay tests" > "$WORKDIRS/$workdir/suibase.yaml"
    fi
}

backup_config_files() {
    local workdir="$1"
    local config_dir="$WORKDIRS/$workdir/config-default"
    local backup_dir="$TEMP_TEST_DIR/backup_$workdir"
    
    mkdir -p "$backup_dir"
    
    # Backup existing config files
    [ -f "$config_dir/walrus-config.yaml" ] && cp "$config_dir/walrus-config.yaml" "$backup_dir/"
    [ -f "$config_dir/relay-config.yaml" ] && cp "$config_dir/relay-config.yaml" "$backup_dir/"
}

restore_config_files() {
    local workdir="$1"
    local config_dir="$WORKDIRS/$workdir/config-default"
    local backup_dir="$TEMP_TEST_DIR/backup_$workdir"
    
    # Restore backed up config files
    if [ -d "$backup_dir" ]; then
        [ -f "$backup_dir/walrus-config.yaml" ] && cp "$backup_dir/walrus-config.yaml" "$config_dir/"
        [ -f "$backup_dir/relay-config.yaml" ] && cp "$backup_dir/relay-config.yaml" "$config_dir/"
    fi
}

cleanup_test() {
    echo "Cleaning up test environment..."
    
    # Stop any running walrus-upload-relay processes for this workdir
    if [ -n "${WALRUS_RELAY_PROCESS_PID:-}" ]; then
        kill "$WALRUS_RELAY_PROCESS_PID" 2>/dev/null || true
        wait "$WALRUS_RELAY_PROCESS_PID" 2>/dev/null || true
        unset WALRUS_RELAY_PROCESS_PID
    fi
    
    # Kill any remaining processes using the test port
    cleanup_port_conflicts
    
    # Clean up stale walrus locks that may have been left by interrupted processes
    local _WALRUS_LOCK="/tmp/.suibase/cli-walrus.lock"
    if [ -d "$_WALRUS_LOCK" ]; then
        if ! lsof "$_WALRUS_LOCK" >/dev/null 2>&1; then
            echo "Removing stale walrus lock directory: $_WALRUS_LOCK"
            rmdir "$_WALRUS_LOCK" 2>/dev/null || true
        fi
    fi
    
    # Restore any backed up files
    restore_config_files "testnet"
    restore_config_files "mainnet"
    
    # Remove temp directory
    [ -d "$TEMP_TEST_DIR" ] && rm -rf "$TEMP_TEST_DIR"
}

cleanup_port_conflicts() {
    local test_port="${CFG_walrus_relay_local_port:-45802}"
    echo "Checking for port conflicts on port $test_port..."
    
    # Find processes using the port with more robust detection
    local pids
    pids=$(ss -tlnp 2>/dev/null | grep ":$test_port " | grep -o 'pid=[0-9]*' | cut -d= -f2 | sort -u 2>/dev/null || true)
    
    if [ -n "$pids" ]; then
        echo "Found processes using port $test_port: $pids"
        for pid in $pids; do
            if kill -0 "$pid" 2>/dev/null; then
                # Check if it's actually a walrus-upload-relay process
                if ps -p "$pid" -o cmd= 2>/dev/null | grep -q "walrus-upload-relay"; then
                    echo "Stopping walrus-upload-relay process $pid using port $test_port"
                    kill "$pid" 2>/dev/null || true
                    # Wait for graceful shutdown
                    sleep 2
                    # Force kill if still running
                    if kill -0 "$pid" 2>/dev/null; then
                        echo "Force killing process $pid"
                        kill -9 "$pid" 2>/dev/null || true
                    fi
                fi
            fi
        done
        
        # Wait for port to be released
        wait_for_port_available "$test_port" 5
    else
        echo "✓ Port $test_port is available"
    fi
}

wait_for_port_available() {
    local port="$1"
    local timeout="${2:-10}"
    local end=$((SECONDS + timeout))
    
    while [ $SECONDS -lt $end ]; do
        if ! ss -tln 2>/dev/null | grep -q ":$port "; then
            echo "✓ Port $port is now available"
            return 0
        fi
        sleep 0.5
    done
    
    echo "⚠ Port $port still in use after ${timeout}s timeout"
    return 1
}

wait_for_process_ready() {
    local port="$1"
    local endpoint="${2:-/v1/tip-config}"
    local timeout="${3:-30}"
    local end=$((SECONDS + timeout))
    
    echo "Waiting for process to be ready on port $port (endpoint: $endpoint, timeout: ${timeout}s)..."
    
    while [ $SECONDS -lt $end ]; do
        # First check if port is listening
        if ss -tln 2>/dev/null | grep -q ":$port "; then
            # Then check if endpoint responds
            if curl -s -m 2 "http://localhost:$port$endpoint" >/dev/null 2>&1; then
                echo "✓ Process ready and responding on port $port"
                return 0
            fi
        fi
        sleep 0.5
    done
    
    echo "✗ Process not ready after ${timeout}s timeout"
    return 1
}

# Override cleanup function
cleanup() {
    cleanup_test
    # Call original cleanup if it exists
    type original_cleanup >/dev/null 2>&1 && original_cleanup
}

# Test assertion helpers
assert_binary_exists() {
    local workdir="$1"
    local binary_path="$WORKDIRS/$workdir/bin/walrus-upload-relay"
    
    if [ ! -f "$binary_path" ]; then
        fail "walrus-upload-relay binary not found at $binary_path"
    fi
    
    if [ ! -x "$binary_path" ]; then
        fail "walrus-upload-relay binary not executable at $binary_path"
    fi
    
    echo "✓ walrus-upload-relay binary exists and is executable at $binary_path"
}

assert_process_running() {
    local pid="$1"
    local process_name="$2"
    
    if [ -z "$pid" ]; then
        fail "$process_name PID is empty"
    fi
    
    if ! kill -0 "$pid" 2>/dev/null; then
        fail "$process_name process (PID $pid) is not running"
    fi
    
    echo "✓ $process_name process is running with PID $pid"
}

assert_config_file_exists() {
    local workdir="$1"
    local config_file="$2"
    local config_path="$WORKDIRS/$workdir/config-default/$config_file"
    
    if [ ! -f "$config_path" ]; then
        fail "$config_file not found at $config_path"
    fi
    
    echo "✓ $config_file exists at $config_path"
}

export -f setup_test_workdir backup_config_files restore_config_files cleanup_test
export -f cleanup_port_conflicts wait_for_port_available wait_for_process_ready
export -f assert_binary_exists assert_process_running assert_config_file_exists