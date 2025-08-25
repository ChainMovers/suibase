#!/bin/bash

# Test walrus relay status integration in both 'testnet status' and 'testnet wal-relay status'
# This ensures wal-relay status appears correctly in the main status output

set -e  # Exit on any error

# Load common test functions
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"

# Test plan
echo "=== Testing Walrus Relay Status Integration ==="
echo "Testing: Status appears in both 'testnet status' and 'testnet wal-relay status'"
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

test_status_in_main_status_when_disabled() {
    echo "--- Test: Status appears in 'testnet status' when disabled ---"

    # Ensure disabled state
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable >/dev/null 2>&1

    # Ensure testnet services are started so status shows walrus relay line
    "$SUIBASE_DIR/scripts/testnet" start >/dev/null 2>&1

    # Test main status output
    local output
    output=$("$SUIBASE_DIR/scripts/testnet" status 2>&1)

    if ! echo "$output" | grep -q "Walrus Relay.*DISABLED"; then
        fail "Main status should show 'Walrus Relay : DISABLED' when disabled. Got: $output"
    fi

    # Verify it's a one-liner (no URL or extra info)
    local relay_line
    relay_line=$(echo "$output" | grep "Walrus Relay" || true)
    if echo "$relay_line" | grep -q "http://"; then
        fail "Main status should NOT show URL when disabled. Got: $relay_line"
    fi

    echo "✓ Main status correctly shows DISABLED as one-liner"
}

test_status_in_main_status_when_enabled() {
    echo "--- Test: Status appears in 'testnet status' when enabled ---"

    # Ensure enabled state
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable >/dev/null 2>&1

    # Start testnet services to ensure status reflects actual state
    "$SUIBASE_DIR/scripts/testnet" start >/dev/null 2>&1

    # Wait for INITIALIZING to resolve to OK/DOWN (max 10 seconds)
    local attempts=0
    local max_attempts=20  # 20 attempts * 0.5s = 10 seconds max
    local output

    while [ $attempts -lt $max_attempts ]; do
        output=$("$SUIBASE_DIR/scripts/testnet" status 2>&1)

        if echo "$output" | grep -q "Walrus Relay.*INITIALIZING"; then
            echo "  Main status: INITIALIZING (attempt $((attempts + 1))/$max_attempts)"
            sleep 0.5
            attempts=$((attempts + 1))
        else
            break
        fi
    done

    # Should show OK when enabled and testnet start has started the process
    if echo "$output" | grep -q "Walrus Relay.*OK"; then
        # Daemon detects walrus-upload-relay process is running and responding
        local relay_line
        relay_line=$(echo "$output" | grep "Walrus Relay" || true)
        echo "✓ Main status correctly shows OK when process is running"
    elif echo "$output" | grep -q "Walrus Relay.*DOWN"; then
        # Daemon detects process issue
        echo "✓ Main status correctly shows DOWN when daemon detects process issue"
    elif echo "$output" | grep -q "Walrus Relay.*INITIALIZING"; then
        fail "Main status stuck in INITIALIZING after 10 seconds. Got: $output"
    else
        fail "Main status should show 'Walrus Relay : OK' or 'DOWN' when enabled and services running. Got: $output"
    fi
}

test_detailed_status_still_works() {
    echo "--- Test: Detailed 'wal-relay status' still works ---"

    # Test when disabled
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable >/dev/null 2>&1
    local disabled_output
    disabled_output=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)

    if ! echo "$disabled_output" | grep -q "DISABLED"; then
        fail "Detailed status should show DISABLED when disabled. Got: $disabled_output"
    fi

    if ! echo "$disabled_output" | grep -q "To enable do"; then
        fail "Detailed status should show enable instruction when disabled. Got: $disabled_output"
    fi

    # Test when enabled
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable >/dev/null 2>&1
    "$SUIBASE_DIR/scripts/testnet" start >/dev/null 2>&1

    local enabled_output
    enabled_output=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)

    # Should show OK when enabled and process is running
    if echo "$enabled_output" | grep -q "OK"; then
        echo "  Detailed status shows OK"
    else
        fail "Detailed status should show OK when enabled and process running. Got: $enabled_output"
    fi

    echo "✓ Detailed 'wal-relay status' works correctly"
}

test_mainnet_status_integration() {
    echo "--- Test: Mainnet status integration ---"

    # Ensure mainnet is set up with binaries
    if [ ! -d "$WORKDIRS/mainnet" ] || ! "$SUIBASE_DIR/scripts/mainnet" status >/dev/null 2>&1; then
        echo "Setting up mainnet workdir..."
        "$SUIBASE_DIR/scripts/mainnet" start >/dev/null 2>&1
    fi

    # Ensure mainnet is disabled
    "$SUIBASE_DIR/scripts/mainnet" wal-relay disable >/dev/null 2>&1

    # Test main status output
    local output
    output=$("$SUIBASE_DIR/scripts/mainnet" status 2>&1)

    # The test should work even if sui binary isn't installed
    # We just need to check that walrus relay status appears
    if echo "$output" | grep -q "The sui binary.*not found"; then
        echo "✓ Mainnet status integration skipped (sui binary not installed)"
        return 0
    fi

    if ! echo "$output" | grep -q "Walrus Relay.*DISABLED"; then
        fail "Mainnet status should show 'Walrus Relay : DISABLED'. Got: $output"
    fi

    echo "✓ Mainnet status integration works"
}

test_devnet_status_no_walrus_relay() {
    echo "--- Test: Devnet status does NOT show walrus relay ---"

    # Ensure devnet is initialized and running so we test the actual status logic
    "$SUIBASE_DIR/scripts/devnet" start >/dev/null 2>&1

    # Test devnet status output
    local output
    output=$("$SUIBASE_DIR/scripts/devnet" status 2>&1)

    if echo "$output" | grep -q "Walrus Relay"; then
        fail "Devnet status should NOT show Walrus Relay. Got: $output"
    fi

    echo "✓ Devnet correctly excludes walrus relay from status"
}

test_status_consistency() {
    echo "--- Test: Status consistency between main and detailed views ---"

    # Test disabled state consistency
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable >/dev/null 2>&1

    local main_status
    main_status=$("$SUIBASE_DIR/scripts/testnet" status 2>&1 | grep "Walrus Relay" || true)

    local detailed_status
    detailed_status=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)

    # Both should show DISABLED
    if ! echo "$main_status" | grep -q "DISABLED"; then
        fail "Main status should show DISABLED. Got: $main_status"
    fi

    if ! echo "$detailed_status" | grep -q "DISABLED"; then
        fail "Detailed status should show DISABLED. Got: $detailed_status"
    fi

    # Test OK state consistency (daemon running, process started)
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable >/dev/null 2>&1
    "$SUIBASE_DIR/scripts/testnet" start >/dev/null 2>&1

    # Test consistency between main and detailed status (with daemon status.yaml)
    main_status=$("$SUIBASE_DIR/scripts/testnet" status 2>&1 | grep "Walrus Relay" || true)
    detailed_status=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)

    # Both should show OK when enabled and walrus-upload-relay process is running
    local main_state detailed_state
    if echo "$main_status" | grep -q "OK"; then
        main_state="OK"
    else
        fail "Main status should show OK when enabled and process running. Got: $main_status"
    fi

    if echo "$detailed_status" | grep -q "OK"; then
        detailed_state="OK"
    else
        fail "Detailed status should show OK when enabled and process running. Got: $detailed_status"
    fi

    if [ "$main_state" != "$detailed_state" ]; then
        fail "Main and detailed status should show same state. Main: $main_state, Detailed: $detailed_state"
    fi

    # OK state shows URL
    if ! echo "$main_status" | grep -q "http://"; then
        fail "Main status should show URL when OK. Got: $main_status"
    fi
    if ! echo "$detailed_status" | grep -q "http://"; then
        fail "Detailed status should show URL when OK. Got: $detailed_status"
    fi
    echo "  Both show OK with URL"

    echo "✓ Status is consistent between main and detailed views in all states"
}

test_status_format_requirements() {
    echo "--- Test: Status format requirements ---"

    # Test that main status is always one-liner
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable >/dev/null 2>&1
    "$SUIBASE_DIR/scripts/testnet" start >/dev/null 2>&1
    local output
    output=$("$SUIBASE_DIR/scripts/testnet" status 2>&1)

    # Count lines containing "Walrus Relay"
    local relay_lines
    relay_lines=$(echo "$output" | grep -c "Walrus Relay" || true)

    if [ "$relay_lines" -ne 1 ]; then
        fail "Main status should have exactly 1 line with 'Walrus Relay', got $relay_lines lines"
    fi

    # Verify proper spacing (should align with other services)
    local relay_line
    relay_line=$(echo "$output" | grep "Walrus Relay" || true)
    if ! echo "$relay_line" | grep -q "^Walrus Relay     : "; then
        fail "Walrus Relay line should have proper spacing alignment. Got: '$relay_line'"
    fi

    echo "✓ Status format meets requirements"
}

# Run tests
test_status_in_main_status_when_disabled
test_status_in_main_status_when_enabled
test_detailed_status_still_works
test_mainnet_status_integration
test_devnet_status_no_walrus_relay
test_status_consistency
test_status_format_requirements

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
echo "=== All Walrus Relay Status Integration Tests Passed! ==="
echo