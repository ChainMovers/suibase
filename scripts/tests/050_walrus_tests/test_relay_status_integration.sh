#!/bin/bash

# Test walrus relay status integration in both '$WORKDIR status' and '$WORKDIR wal-relay status'
# This ensures wal-relay status appears correctly in the main status output

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
echo "=== Testing Walrus Relay Status Integration ==="
echo "Testing: Status appears in both '$WORKDIR status' and '$WORKDIR wal-relay status'"
echo

# Setup test environment
setup_test_workdir "$WORKDIR"
backup_config_files "$WORKDIR"

# Ensure we're using BUILD version of suibase-daemon for walrus relay features
ensure_build_daemon

# Store original config state
ORIGINAL_CONFIG_STATE=""
if grep -q "^walrus_relay_enabled:" "$WORKDIRS/$WORKDIR/suibase.yaml" 2>/dev/null; then
    ORIGINAL_CONFIG_STATE=$(grep "^walrus_relay_enabled:" "$WORKDIRS/$WORKDIR/suibase.yaml")
fi

test_status_in_main_status_when_disabled() {
    echo "--- Test: Status appears in '$WORKDIR status' when disabled ---"

    # Ensure disabled state
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable

    # Ensure $WORKDIR services are started so status shows walrus relay line
    "$SUIBASE_DIR/scripts/$WORKDIR" start

    # Test main status output
    local output
    output=$("$SUIBASE_DIR/scripts/$WORKDIR" status 2>&1)

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
    echo "--- Test: Status appears in '$WORKDIR status' when enabled ---"

    # Ensure enabled state
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable

    # Start $WORKDIR services to ensure status reflects actual state
    "$SUIBASE_DIR/scripts/$WORKDIR" start

    # Wait for main status to stabilize to OK or DOWN (max 10 seconds)
    if wait_for_service_status "$WORKDIR status" "OK|DOWN" "Walrus Relay" 10 true; then
        # Final check to confirm the result
        local output
        output=$("$SUIBASE_DIR/scripts/$WORKDIR" status 2>&1)
        
        if echo "$output" | grep -q "Walrus Relay.*OK"; then
            echo "✓ Main status correctly shows OK when process is running"
        elif echo "$output" | grep -q "Walrus Relay.*DOWN"; then
            echo "✓ Main status correctly shows DOWN when daemon detects process issue"
        fi
    else
        local final_output
        final_output=$("$SUIBASE_DIR/scripts/$WORKDIR" status 2>&1)
        fail "Main status failed to reach OK or DOWN within 10 seconds. Got: $final_output"
    fi
}

test_detailed_status_still_works() {
    echo "--- Test: Detailed 'wal-relay status' still works ---"

    # Test when disabled
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable
    local disabled_output
    disabled_output=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay status 2>&1)

    if ! echo "$disabled_output" | grep -q "DISABLED"; then
        fail "Detailed status should show DISABLED when disabled. Got: $disabled_output"
    fi

    if ! echo "$disabled_output" | grep -q "To enable do"; then
        fail "Detailed status should show enable instruction when disabled. Got: $disabled_output"
    fi

    # Test when enabled
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable
    "$SUIBASE_DIR/scripts/$WORKDIR" start

    local enabled_output
    enabled_output=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay status 2>&1)

    # Should show OK when enabled and process is running
    if echo "$enabled_output" | grep -q "OK"; then
        echo "  Detailed status shows OK"
    else
        fail "Detailed status should show OK when enabled and process running. Got: $enabled_output"
    fi

    echo "✓ Detailed 'wal-relay status' works correctly"
}


test_devnet_status_no_walrus_relay() {
    echo "--- Test: Devnet status does NOT show walrus relay ---"

    # Ensure devnet is initialized and running so we test the actual status logic
    "$SUIBASE_DIR/scripts/devnet" start

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
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable

    local main_status
    main_status=$("$SUIBASE_DIR/scripts/$WORKDIR" status 2>&1 | grep "Walrus Relay" || true)

    local detailed_status
    detailed_status=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay status 2>&1)

    # Both should show DISABLED
    if ! echo "$main_status" | grep -q "DISABLED"; then
        fail "Main status should show DISABLED. Got: $main_status"
    fi

    if ! echo "$detailed_status" | grep -q "DISABLED"; then
        fail "Detailed status should show DISABLED. Got: $detailed_status"
    fi

    # Test OK state consistency (daemon running, process started)
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable
    "$SUIBASE_DIR/scripts/$WORKDIR" start

    # Test consistency between main and detailed status (with daemon status.yaml)
    main_status=$("$SUIBASE_DIR/scripts/$WORKDIR" status 2>&1 | grep "Walrus Relay" || true)
    detailed_status=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay status 2>&1)

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
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable
    "$SUIBASE_DIR/scripts/$WORKDIR" start
    local output
    output=$("$SUIBASE_DIR/scripts/$WORKDIR" status 2>&1)

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
test_devnet_status_no_walrus_relay
test_status_consistency
test_status_format_requirements

# Restore original config state
echo "--- Restoring original configuration ---"

# Check if suibase.yaml file exists before attempting to modify it
if [ ! -f "$WORKDIRS/$WORKDIR/suibase.yaml" ]; then
    echo "ERROR: suibase.yaml file not found at $WORKDIRS/$WORKDIR/suibase.yaml"
    exit 1
fi

if [ -n "$ORIGINAL_CONFIG_STATE" ]; then
    # Remove any existing walrus_relay_enabled line and add the original
    sed -i.bak '/^walrus_relay_enabled:/d' "$WORKDIRS/$WORKDIR/suibase.yaml" && rm "$WORKDIRS/$WORKDIR/suibase.yaml.bak"
    echo "$ORIGINAL_CONFIG_STATE" >> "$WORKDIRS/$WORKDIR/suibase.yaml"
    echo "✓ Restored original config: $ORIGINAL_CONFIG_STATE"
else
    # Remove walrus_relay_enabled line if it didn't exist originally
    sed -i.bak '/^walrus_relay_enabled:/d' "$WORKDIRS/$WORKDIR/suibase.yaml" && rm "$WORKDIRS/$WORKDIR/suibase.yaml.bak"
    echo "✓ Removed walrus_relay_enabled (wasn't present originally)"
fi

echo
echo "=== All Walrus Relay Status Integration Tests Passed! ==="
echo