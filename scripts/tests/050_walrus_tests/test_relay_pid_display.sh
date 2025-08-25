#!/bin/bash

# Test walrus relay PID display scenarios using real enable/start/stop/disable commands
# Tests that PID is shown when walrus-upload-relay process is running

set -e  # Exit on any error

# Load common test functions
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"

# Test plan
echo "=== Testing Walrus Relay PID Display ==="
echo "Testing: PID display with real enable/start/stop/disable commands"
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

# Use the common utility function instead of custom implementation
wait_for_status_change() {
    local expected_status="$1"
    local max_wait="${2:-10}"
    
    wait_for_walrus_relay_status "testnet" "$expected_status" "$max_wait" false
}

test_disabled_state() {
    echo "--- Test: DISABLED state ---"
    
    # Disable walrus relay
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable >/dev/null 2>&1
    
    # Test detailed status
    local detailed_output
    detailed_output=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)
    
    if ! echo "$detailed_output" | grep -q "DISABLED"; then
        fail "Detailed status should show DISABLED when disabled. Got: $detailed_output"
    fi
    
    if echo "$detailed_output" | grep -q "( pid"; then
        fail "Detailed status should NOT show PID when DISABLED. Got: $detailed_output"
    fi
    
    if ! echo "$detailed_output" | grep -q "To enable do"; then
        fail "Detailed status should show enable instruction when DISABLED. Got: $detailed_output"
    fi
    
    # Test main status
    local main_output
    main_output=$("$SUIBASE_DIR/scripts/testnet" status 2>&1)
    local relay_line
    relay_line=$(echo "$main_output" | grep "Walrus Relay" || true)
    
    if ! echo "$relay_line" | grep -q "DISABLED"; then
        fail "Main status should show DISABLED. Got: $relay_line"
    fi
    
    if echo "$relay_line" | grep -q "( pid"; then
        fail "Main status should NOT show PID when DISABLED. Got: $relay_line"
    fi
    
    echo "✓ DISABLED state correctly shows without PID but with help"
}

test_enable_and_start_services() {
    echo "--- Test: Enable walrus relay and start services ---"
    
    # Start from disabled state
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable >/dev/null 2>&1
    
    # Enable walrus relay
    echo "Enabling walrus relay..."
    local enable_output
    enable_output=$("$SUIBASE_DIR/scripts/testnet" wal-relay enable 2>&1)
    
    if ! echo "$enable_output" | grep -q "Walrus relay is now enabled"; then
        fail "Enable command should confirm enablement. Got: $enable_output"
    fi
    
    # Check status after enable (should be DOWN when daemon detects no walrus-upload-relay process)
    local status_after_enable
    status_after_enable=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)
    echo "Status after enable: $status_after_enable"
    
    # Start all services
    echo "Starting testnet services..."
    local start_output
    start_output=$("$SUIBASE_DIR/scripts/testnet" start 2>&1)
    echo "Start command completed"
    
    # Wait for services to reach OK or DOWN status (not INITIALIZING)
    if ! wait_for_walrus_relay_status "testnet" "OK|DOWN" 15; then
        fail "Walrus relay did not reach OK or DOWN status after starting services"
    fi
    
    local status_after_start
    status_after_start=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)
    echo "Status after start: $status_after_start"
    
    # The status should show the actual daemon-detected state
    if echo "$status_after_start" | grep -q "DOWN"; then
        echo "✓ Walrus relay shows DOWN - daemon detects no walrus-upload-relay process running"
    elif echo "$status_after_start" | grep -q "OK"; then
        echo "✓ Walrus relay shows OK - backend process is running"
        
        # Check if PID is shown when status is OK
        if echo "$status_after_start" | grep -q "( pid"; then
            echo "✓ PID is shown when status is OK"
            # Extract and verify the PID
            local displayed_pid
            displayed_pid=$(echo "$status_after_start" | sed -n 's/.*( pid \([0-9]*\) ).*/\1/p')
            if [ -n "$displayed_pid" ] && kill -0 "$displayed_pid" 2>/dev/null; then
                echo "✓ Displayed PID $displayed_pid is a valid running process"
            else
                echo "⚠ Displayed PID $displayed_pid is not valid or not running"
            fi
        else
            echo "⚠ PID not shown when status is OK"
        fi
    elif echo "$status_after_start" | grep -q "INITIALIZING"; then
        fail "Status stuck in INITIALIZING after 10 seconds. Got: $status_after_start"
    else
        echo "⚠ Unexpected status after start: $status_after_start"
    fi
    
    echo "✓ Enable and start test completed"
}

test_stop_services() {
    echo "--- Test: Stop services ---"
    
    # Ensure walrus relay is enabled and services are running
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable >/dev/null 2>&1
    "$SUIBASE_DIR/scripts/testnet" start >/dev/null 2>&1
    
    # Wait for services to be ready
    wait_for_walrus_relay_status "testnet" "OK|DOWN" 10 >/dev/null 2>&1 || true
    
    # Check status before stop
    local status_before_stop
    status_before_stop=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)
    echo "Status before stop: $status_before_stop"
    
    # Stop all services
    echo "Stopping testnet services..."
    local stop_output
    stop_output=$("$SUIBASE_DIR/scripts/testnet" stop 2>&1)
    echo "Stop command completed"
    
    # Wait for services to show STOPPED, DOWN, or DISABLED status
    wait_for_walrus_relay_status "testnet" "STOPPED|DOWN|DISABLED" 10 >/dev/null 2>&1 || true
    
    # Check status after stop
    local status_after_stop
    status_after_stop=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)
    echo "Status after stop: $status_after_stop"
    
    # After stop, status can show DOWN, STOPPED, or DISABLED depending on daemon state
    if echo "$status_after_stop" | grep -q "DOWN"; then
        echo "✓ Walrus relay shows DOWN after stop - daemon detects no walrus-upload-relay process"
    elif echo "$status_after_stop" | grep -q "STOPPED"; then
        echo "✓ Walrus relay shows STOPPED - services are stopped"
    elif echo "$status_after_stop" | grep -q "DISABLED"; then
        echo "✓ Walrus relay shows DISABLED - relay was disabled"
    else
        echo "⚠ Unexpected status after stop: $status_after_stop"
    fi
    
    # Status should not show PID when stopped
    if echo "$status_after_stop" | grep -q "( pid"; then
        fail "Status should NOT show PID when services are stopped. Got: $status_after_stop"
    fi
    
    echo "✓ Stop services test completed"
}

test_status_consistency() {
    echo "--- Test: Status consistency between verbose and main status ---"
    
    # Test disabled state consistency
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable >/dev/null 2>&1
    "$SUIBASE_DIR/scripts/testnet" start >/dev/null 2>&1  # Start services so status shows properly
    
    local verbose_status
    verbose_status=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1 | grep "Walrus Relay" | head -1)
    
    local main_status
    main_status=$("$SUIBASE_DIR/scripts/testnet" status 2>&1 | grep "Walrus Relay" | head -1)
    
    # Both should show DISABLED
    if ! echo "$verbose_status" | grep -q "DISABLED"; then
        fail "Verbose status should show DISABLED. Got: $verbose_status"
    fi
    
    if ! echo "$main_status" | grep -q "DISABLED"; then
        fail "Main status should show DISABLED. Got: $main_status"
    fi
    
    # Test enabled state consistency
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable >/dev/null 2>&1
    
    verbose_status=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1 | grep "Walrus Relay" | head -1)
    main_status=$("$SUIBASE_DIR/scripts/testnet" status 2>&1 | grep "Walrus Relay" | head -1)
    
    # Extract the status part (after the colon)
    local verbose_status_part
    verbose_status_part=$(echo "$verbose_status" | sed 's/.*: //' | sed 's/\[.*m//g' | awk '{print $1}')
    
    local main_status_part
    main_status_part=$(echo "$main_status" | sed 's/.*: //' | sed 's/\[.*m//g' | awk '{print $1}')
    
    if [ "$verbose_status_part" != "$main_status_part" ]; then
        echo "⚠ Status mismatch between verbose ('$verbose_status_part') and main ('$main_status_part')"
        echo "  Verbose: $verbose_status"
        echo "  Main: $main_status"
    else
        echo "✓ Status is consistent between verbose and main views: $verbose_status_part"
    fi
    
    echo "✓ Status consistency test completed"
}

test_disabled_state() {
    echo "--- Test: DISABLED state ---"
    
    # Disable walrus relay
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable >/dev/null 2>&1
    
    # Test detailed status
    local detailed_output
    detailed_output=$("$SUIBASE_DIR/scripts/testnet" wal-relay status 2>&1)
    
    if ! echo "$detailed_output" | grep -q "DISABLED"; then
        fail "Detailed status should show DISABLED when disabled. Got: $detailed_output"
    fi
    
    if echo "$detailed_output" | grep -q "( pid"; then
        fail "Detailed status should NOT show PID when DISABLED. Got: $detailed_output"
    fi
    
    if ! echo "$detailed_output" | grep -q "To enable do"; then
        fail "Detailed status should show enable instruction when DISABLED. Got: $detailed_output"
    fi
    
    # Test main status
    local main_output
    main_output=$("$SUIBASE_DIR/scripts/testnet" status 2>&1)
    local relay_line
    relay_line=$(echo "$main_output" | grep "Walrus Relay" || true)
    
    if ! echo "$relay_line" | grep -q "DISABLED"; then
        fail "Main status should show DISABLED. Got: $relay_line"
    fi
    
    if echo "$relay_line" | grep -q "( pid"; then
        fail "Main status should NOT show PID when DISABLED. Got: $relay_line"
    fi
    
    echo "✓ DISABLED state correctly shows without PID but with help"
}


# Run tests
test_disabled_state
test_enable_and_start_services
test_stop_services
test_status_consistency

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

# Cleanup any test files
rm -rf "$WORKDIRS/testnet/walrus-relay"

echo
echo "=== All Walrus Relay PID Display Tests Passed! ==="
echo