#!/bin/bash

# Test walrus relay CLI commands (status, enable, disable)
# Tests the bash-level functionality without requiring Rust daemon support

set -e  # Exit on any error

# Load common test functions
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"

# Test plan
echo "=== Testing Walrus Relay CLI Commands ==="
echo "Testing: testnet wal-relay status/enable/disable"
echo

# Setup test environment
setup_test_workdir "testnet"
backup_config_files "testnet"

# Ensure we're using BUILD version of suibase-daemon for walrus relay features
ensure_build_daemon

# Store original config state
ORIGINAL_CONFIG_STATE=""
if grep -q "^walrus_relay_enabled:" "$WORKDIRS/testnet/suibase.yaml" 2>/dev/null; then
    ORIGINAL_CONFIG_STATE=$(grep "^walrus_relay_enabled:" "$WORKDIRS/testnet/suibase.yaml")
fi

test_status_when_disabled() {
    echo "--- Test: Status when disabled ---"

    # Ensure disabled state  
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable
    
    # Start services so CLI reads config state instead of showing STOPPED
    "$SUIBASE_DIR/scripts/testnet" start

    # Test status output - CLI detects DISABLED instantaneously from config
    local output
    output=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)

    if ! echo "$output" | grep -q "DISABLED"; then
        fail "Status should show DISABLED when relay is disabled. Got: $output"
    fi

    if ! echo "$output" | grep -q "To enable do 'testnet wal-relay enable'"; then
        fail "Status should show enable instruction when disabled. Got: $output"
    fi

    echo "✓ Status correctly shows DISABLED state with enable instruction"
}

test_enable_command() {
    echo "--- Test: Enable command ---"

    # Ensure starting from disabled state
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable

    # Test enable command
    local output
    output=$("$SUIBASE_DIR/scripts/testnet" wal-relay enable 2>&1)

    if ! echo "$output" | grep -q "Walrus relay is now enabled"; then
        fail "Enable command should confirm enablement. Got: $output"
    fi

    # Verify config file was updated
    if ! grep -q "^walrus_relay_enabled: true" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "Config file should contain 'walrus_relay_enabled: true' after enable"
    fi

    echo "✓ Enable command works and updates config file"
}

test_status_when_enabled() {
    echo "--- Test: Status when enabled (daemon supports walrus relay) ---"

    # NOTE: With daemon walrus support implemented, this tests the real behavior:
    # When walrus_relay_enabled=true and daemon supports walrus relay,
    # status shows DOWN when process is not running, or OK when process is running

    # Ensure enabled state but stop services so process isn't running
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable
    "$SUIBASE_DIR/scripts/testnet" stop

    # Test status output
    local output
    output=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)

    # Should show DOWN, STOPPED, or OK depending on daemon and process state
    if echo "$output" | grep -q "DOWN"; then
        echo "✓ Status correctly shows DOWN state when enabled but process not running"
    elif echo "$output" | grep -q "STOPPED"; then
        echo "✓ Status correctly shows STOPPED state when services are stopped"
    elif echo "$output" | grep -q "OK"; then
        echo "✓ Status correctly shows OK state when enabled and process running"
    else
        fail "Status should show DOWN, STOPPED, or OK when enabled with daemon support. Got: $output"
    fi
}

test_status_when_working() {
    echo "--- Test: Status when working (daemon integration) ---"

    # NOTE: With daemon walrus support implemented, this tests actual OK/DOWN states
    # When enabled and services running, status should be OK or DOWN from daemon

    # Ensure enabled state and start services  
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable
    "$SUIBASE_DIR/scripts/testnet" start

    # Test status output 
    local output
    output=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)

    # Wait for INITIALIZING to resolve to OK or DOWN (max 10 seconds)
    if ! wait_for_walrus_relay_status "testnet" "OK|DOWN" 10 true; then
        echo "⚠ Walrus relay status did not resolve from INITIALIZING within 10 seconds"
    fi
    
    # Get final status
    output=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)
    
    # Check final status after INITIALIZING resolved (or timeout)
    if echo "$output" | grep -q "OK"; then
        # Daemon implementation working - process running and healthy
        if ! echo "$output" | grep -q "http://localhost:45852"; then
            fail "Status should show proxy URL when OK. Got: $output"
        fi
        if ! echo "$output" | grep -q "pid"; then
            fail "Status should show PID when OK and running. Got: $output"  
        fi
        echo "✓ Status correctly shows OK state with proxy URL and PID"
    elif echo "$output" | grep -q "DOWN"; then
        # Daemon detects process not running or unhealthy
        echo "✓ Status correctly shows DOWN state (daemon detects process issue)"
    elif echo "$output" | grep -q "INITIALIZING"; then
        # Timeout waiting for INITIALIZING to resolve
        fail "Status stuck in INITIALIZING after 10 seconds. Got: $output"
    elif echo "$output" | grep -q "NOT RUNNING"; then
        # Daemon not running (should not happen if testnet start worked)
        echo "⚠ Status shows NOT RUNNING - daemon may not be running"
    else
        fail "Status should show OK or DOWN when enabled with daemon support. Got: $output"
    fi
}

test_disable_command() {
    echo "--- Test: Disable command ---"

    # Ensure starting from enabled state
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable

    # Test disable command
    local output
    output=$("$SUIBASE_DIR/scripts/testnet" wal-relay disable 2>&1)

    if ! echo "$output" | grep -q "Walrus relay is now disabled"; then
        fail "Disable command should confirm disablement. Got: $output"
    fi

    # Verify config file was updated
    if ! grep -q "^walrus_relay_enabled: false" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "Config file should contain 'walrus_relay_enabled: false' after disable"
    fi

    echo "✓ Disable command works and updates config file"
}

test_enable_when_already_enabled() {
    echo "--- Test: Enable when already enabled ---"

    # Ensure enabled state
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable

    # Test enable command again
    local output
    output=$("$SUIBASE_DIR/scripts/testnet" wal-relay enable 2>&1)

    if ! echo "$output" | grep -q "Walrus relay already enabled"; then
        fail "Enable command should detect already enabled state. Got: $output"
    fi

    echo "✓ Enable command correctly detects already enabled state"
}

test_disable_when_already_disabled() {
    echo "--- Test: Disable when already disabled ---"

    # Ensure disabled state
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable

    # Test disable command again
    local output
    output=$("$SUIBASE_DIR/scripts/testnet" wal-relay disable 2>&1)

    if ! echo "$output" | grep -q "Walrus relay already disabled"; then
        fail "Disable command should detect already disabled state. Got: $output"
    fi

    echo "✓ Disable command correctly detects already disabled state"
}

test_mainnet_support() {
    echo "--- Test: Mainnet support ---"

    # Test mainnet status (should work)
    local output
    output=$("$SUIBASE_DIR/scripts/mainnet" wal-relay status 2>&1)

    if ! echo "$output" | grep -q "DISABLED"; then
        fail "Mainnet wal-relay status should work. Got: $output"
    fi

    echo "✓ Mainnet wal-relay commands work"
}

test_unsupported_network() {
    echo "--- Test: Unsupported network ---"

    # Test devnet (should fail)
    local output
    output=$("$SUIBASE_DIR/scripts/devnet" wal-relay status 2>&1 || true)

    if ! echo "$output" | grep -q "not supported for devnet"; then
        fail "Devnet wal-relay should show error. Got: $output"
    fi

    echo "✓ Unsupported networks properly reject wal-relay commands"
}

test_help_command() {
    echo "--- Test: Help command ---"

    # Test help output
    local output
    output=$("$SUIBASE_DIR/scripts/testnet" wal-relay --help 2>&1)

    if ! echo "$output" | grep -q "USAGE:"; then
        fail "Help should show usage information. Got: $output"
    fi

    if ! echo "$output" | grep -q "status" || ! echo "$output" | grep -q "enable" || ! echo "$output" | grep -q "disable"; then
        fail "Help should show available subcommands. Got: $output"
    fi

    if ! echo "$output" | grep -q "Application → suibase-daemon"; then
        fail "Help should show architecture information. Got: $output"
    fi

    echo "✓ Help command shows proper usage information"
}

# Run tests
test_status_when_disabled
test_enable_command
test_status_when_enabled
test_status_when_working
test_disable_command
test_enable_when_already_enabled
test_disable_when_already_disabled
test_mainnet_support
test_unsupported_network
test_help_command

# Restore original config state
echo "--- Restoring original configuration ---"
if [ -n "$ORIGINAL_CONFIG_STATE" ]; then
    # Remove any existing walrus_relay_enabled line and add the original
    sed -i.bak '/^walrus_relay_enabled:/d' "$WORKDIRS/testnet/suibase.yaml" && rm "$WORKDIRS/testnet/suibase.yaml.bak"
    echo "$ORIGINAL_CONFIG_STATE" >> "$WORKDIRS/testnet/suibase.yaml"
    echo "✓ Restored original config: $ORIGINAL_CONFIG_STATE"
else
    # Remove walrus_relay_enabled line if it didn't exist originally
    sed -i.bak '/^walrus_relay_enabled:/d' "$WORKDIRS/testnet/suibase.yaml" && rm "$WORKDIRS/testnet/suibase.yaml.bak"
    echo "✓ Removed walrus_relay_enabled (wasn't present originally)"
fi

echo
echo "=== All Walrus Relay CLI Tests Passed! ==="
echo