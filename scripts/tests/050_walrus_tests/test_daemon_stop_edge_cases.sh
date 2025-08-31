#!/bin/bash

# Test edge cases when suibase-daemon is stopped
# This ensures CLI handles daemon unavailability gracefully and falls back appropriately

set -e  # Exit on any error

# Load common test functions
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"


echo "=== Testing Walrus Relay with Daemon Stop Edge Cases ==="
echo "Testing: CLI behavior when suibase-daemon is not running"
echo

# Setup clean environment and test workdir
setup_clean_environment
setup_test_workdir "$WORKDIR"
backup_config_files "$WORKDIR"

# Ensure we're using BUILD version of suibase-daemon for walrus relay features
ensure_build_daemon

# Store original config state
ORIGINAL_CONFIG_STATE=""
if grep -q "^walrus_relay_enabled:" "$WORKDIRS/$WORKDIR/suibase.yaml" 2>/dev/null; then
    ORIGINAL_CONFIG_STATE=$(grep "^walrus_relay_enabled:" "$WORKDIRS/$WORKDIR/suibase.yaml")
fi

test_daemon_running_baseline() {
    echo "--- Test: Baseline with daemon running ---"

    # Ensure daemon is running
    if ! "$SUIBASE_DIR/scripts/dev/is-daemon-running" >/dev/null 2>&1; then
        echo "Starting suibase-daemon for baseline test..."
        safe_start_daemon
        wait_for_daemon_running 10 >/dev/null 2>&1 || true
    fi

    # Enable walrus relay
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable

    # Test status with daemon running
    echo "✓ Initial walrus relay status:"
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay status

    # Check that status.yaml exists and is written by daemon
    if [ -f "$WORKDIRS/$WORKDIR/walrus-relay/status.yaml" ]; then
        echo "✓ status.yaml exists when daemon is running"
        local last_check
        last_check=$(grep "last_check:" "$WORKDIRS/$WORKDIR/walrus-relay/status.yaml" | cut -d' ' -f2 || echo "unknown")
        echo "  Last check: $last_check"
    else
        echo "⚠ status.yaml not found when daemon is running"
    fi

    echo "✓ Baseline test completed"
}

test_daemon_stopped() {
    echo "--- Test: Behavior when daemon is stopped ---"

    # Stop the daemon
    echo "Stopping suibase-daemon..."
    "$SUIBASE_DIR/scripts/dev/stop-daemon"

    # Wait for daemon to fully stop
    if ! wait_for_daemon_stopped; then
        fail "Daemon failed to stop within timeout"
    fi
    echo "✓ suibase-daemon stopped"

    # Test status command when daemon is stopped
    local status_output
    status_output=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay status 2>&1)

    # CLI should fall back to "NOT RUNNING" when daemon is stopped (can't get real status)
    if strip_ansi_colors "$status_output" | grep -q "NOT RUNNING"; then
        echo "✓ Shows NOT RUNNING when daemon stopped (correct fallback behavior)"
    elif strip_ansi_colors "$status_output" | grep -q "DISABLED"; then
        echo "✓ Shows DISABLED when daemon stopped (config-based fallback)"
    else
        echo "⚠ Unexpected behavior when daemon stopped: $status_output"
    fi

    # Test main status command too
    local main_status_output
    main_status_output=$("$SUIBASE_DIR/scripts/$WORKDIR" status 2>&1 | grep "Walrus Relay" || true)

    if [ -n "$main_status_output" ]; then
        echo "✓ Main status includes walrus relay even when daemon stopped"
    else
        echo "⚠ Main status missing walrus relay when daemon stopped"
    fi

    echo "✓ Daemon stopped test completed"
}

test_enable_disable_without_daemon() {
    echo "--- Test: Enable/disable commands without daemon ---"

    # Ensure daemon is stopped
    "$SUIBASE_DIR/scripts/dev/stop-daemon" >/dev/null 2>&1
    wait_for_daemon_stopped >/dev/null 2>&1 || true

    # Test disable command
    local disable_output
    disable_output=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable 2>&1)

    if echo "$disable_output" | grep -q "disabled"; then
        echo "✓ Disable command works without daemon"
    else
        echo "⚠ Disable command issue without daemon: $disable_output"
    fi

    # Test enable command
    local enable_output
    enable_output=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable 2>&1)

    if echo "$enable_output" | grep -q "enabled"; then
        echo "✓ Enable command works without daemon"
    else
        echo "⚠ Enable command issue without daemon: $enable_output"
    fi

    echo "✓ Enable/disable without daemon test completed"
}

test_daemon_restart_recovery() {
    echo "--- Test: Daemon restart recovery ---"

    # Start with daemon stopped and walrus relay enabled
    "$SUIBASE_DIR/scripts/dev/stop-daemon" >/dev/null 2>&1
    wait_for_daemon_stopped >/dev/null 2>&1 || true
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable

    # Restart daemon
    echo "Restarting suibase-daemon..."
    safe_start_daemon

    # Wait for daemon to start and walrus status to be ready
    wait_for_daemon_running 10 >/dev/null 2>&1
    wait_for_walrus_relay_status "$WORKDIR" "OK|DOWN|INITIALIZING" 8 >/dev/null 2>&1 || true

    # Check if daemon properly recovers walrus relay status
    local recovery_status
    recovery_status=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay status 2>&1)

    if strip_ansi_colors "$recovery_status" | grep -q "DOWN\|OK"; then
        echo "✓ Daemon properly recovers walrus relay status after restart"

        # Verify status.yaml is recreated
        if [ -f "$WORKDIRS/$WORKDIR/walrus-relay/status.yaml" ]; then
            echo "✓ status.yaml recreated after daemon restart"
        else
            echo "⚠ status.yaml not recreated after daemon restart"
        fi
    else
        echo "⚠ Daemon recovery issue: $recovery_status"
    fi

    echo "✓ Daemon restart recovery test completed"
}

test_config_change_without_daemon() {
    echo "--- Test: Config changes without daemon should take effect on restart ---"

    # Stop daemon and make config change
    "$SUIBASE_DIR/scripts/dev/stop-daemon"

    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable

    # Modify suibase.yaml to set walrus_relay_local_port based on workdir
    echo "Checking if walrus_relay_local_port needs to be added..."
    if ! grep -q "walrus_relay_local_port:" "$WORKDIRS/$WORKDIR/suibase.yaml" 2>/dev/null; then
        echo "Adding walrus_relay_local_port to config..."
        # Use different port based on workdir
        local port
        case "$WORKDIR" in
            "testnet") port="45802" ;;
            "mainnet") port="45803" ;;
            *) port="45802" ;;
        esac
        echo "walrus_relay_local_port: $port" >> "$WORKDIRS/$WORKDIR/suibase.yaml"
    else
        echo "walrus_relay_local_port already present in config"
    fi

    # Start daemon

    safe_start_daemon

    # Wait for daemon to start and walrus status to be ready
    wait_for_daemon_running 10 >/dev/null 2>&1
    wait_for_walrus_relay_status "$WORKDIR" "OK|DOWN|DISABLED|INITIALIZING" 8 >/dev/null 2>&1 || true

    # Check that daemon picks up the config

    local status_after_config
    status_after_config=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay status)


    if strip_ansi_colors "$status_after_config" | grep -q "DOWN\|OK"; then
        echo "✓ Daemon picks up config changes made while it was stopped"
    else
        echo "⚠ Daemon may not have picked up config changes: $status_after_config"
        if [ -f "$WORKDIRS/$WORKDIR/suibase.yaml" ]; then
            grep -E "walrus_relay" "$WORKDIRS/$WORKDIR/suibase.yaml" || echo "DEBUG: No walrus_relay settings found in config"
        else
            echo "Config file does not exist at $WORKDIRS/$WORKDIR/suibase.yaml"
        fi
    fi

    echo "✓ Config change without daemon test completed"
}

# Run tests in sequence
test_daemon_running_baseline
test_daemon_stopped
test_enable_disable_without_daemon
test_daemon_restart_recovery
test_config_change_without_daemon

# Restore original config state
echo "--- Restoring original configuration ---"

# Check if suibase.yaml file exists before attempting to modify it
if [ ! -f "$WORKDIRS/$WORKDIR/suibase.yaml" ]; then
    echo "ERROR: suibase.yaml file not found at $WORKDIRS/$WORKDIR/suibase.yaml"
    exit 1
fi

if [ -n "$ORIGINAL_CONFIG_STATE" ]; then
    # Remove any existing walrus_relay_enabled line and add the original
    sed -i.bak '/^walrus_relay_enabled:/d' "$WORKDIRS/$WORKDIR/suibase.yaml" && rm -f "$WORKDIRS/$WORKDIR/suibase.yaml.bak"
    sed -i.bak '/^walrus_relay_local_port:/d' "$WORKDIRS/$WORKDIR/suibase.yaml" && rm -f "$WORKDIRS/$WORKDIR/suibase.yaml.bak"
    echo "$ORIGINAL_CONFIG_STATE" >> "$WORKDIRS/$WORKDIR/suibase.yaml"
    echo "✓ Restored original config: $ORIGINAL_CONFIG_STATE"
else
    # Remove walrus_relay lines if they didn't exist originally
    sed -i.bak '/^walrus_relay_enabled:/d' "$WORKDIRS/$WORKDIR/suibase.yaml" && rm -f "$WORKDIRS/$WORKDIR/suibase.yaml.bak"
    sed -i.bak '/^walrus_relay_local_port:/d' "$WORKDIRS/$WORKDIR/suibase.yaml" && rm -f "$WORKDIRS/$WORKDIR/suibase.yaml.bak"
    echo "✓ Removed walrus_relay config (wasn't present originally)"
fi

# Ensure daemon is running for other tests
if ! "$SUIBASE_DIR/scripts/dev/is-daemon-running" >/dev/null 2>&1; then
    echo "Restarting daemon for other tests..."
    safe_start_daemon
fi

echo
echo "=== All Daemon Stop Edge Case Tests Completed! ==="
echo