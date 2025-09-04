#!/bin/bash

# Test that '$WORKDIR start' properly starts walrus-upload-relay when walrus_relay_enabled: true
# This test reproduces the bug where $WORKDIR start doesn't start walrus relay despite being enabled

# Ignore SIGPIPE on macOS to prevent test failures
if [[ "$(uname)" == "Darwin" ]]; then
    trap '' SIGPIPE
    echo "SIGPIPE trap installed"
fi

# Load common test functions
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"


# Test plan
echo "=== Testing $WORKDIR Start Walrus Relay Integration ==="
echo "Testing: '$WORKDIR start' should start walrus-upload-relay when enabled"
echo


# Setup test environment with clean state
setup_clean_environment
setup_test_workdir "$WORKDIR"
backup_config_files "$WORKDIR"

# Ensure we're using BUILD version of suibase-daemon for walrus relay features
ensure_build_daemon

# Ensure clean state by stopping any running $WORKDIR services
echo "Ensuring clean state..."
"$SUIBASE_DIR/scripts/$WORKDIR" stop || true

# Store original config state
ORIGINAL_CONFIG_STATE=""
if grep -q "^walrus_relay_enabled:" "$WORKDIRS/$WORKDIR/suibase.yaml" 2>/dev/null; then
    ORIGINAL_CONFIG_STATE=$(grep "^walrus_relay_enabled:" "$WORKDIRS/$WORKDIR/suibase.yaml")
fi

test_start_with_walrus_enabled() {
    echo "--- Test: $WORKDIR start should start walrus relay when enabled ---"

    # Ensure walrus relay is enabled
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable

    # Stop any existing processes first
    "$SUIBASE_DIR/scripts/$WORKDIR" stop

    # Verify walrus relay is enabled in config
    if ! grep -q "^walrus_relay_enabled: true" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "walrus_relay_enabled should be true in suibase.yaml"
    fi

    echo "  Config shows walrus_relay_enabled: true"

    # Start $WORKDIR services
    echo "  Starting $WORKDIR services..."
    "$SUIBASE_DIR/scripts/$WORKDIR" start

    # Wait for walrus relay process to be running or status to be ready
    wait_for_walrus_relay_status "$WORKDIR" "OK|DOWN|INITIALIZING" 10 >/dev/null 2>&1 || true

    # Check if walrus-upload-relay process is running
    if ! check_walrus_process_running "$WORKDIR" >/dev/null 2>&1; then
        # Get status to see what happened
        "$SUIBASE_DIR/scripts/$WORKDIR" status

        # Check status.yaml for debugging
        if [ -f "$WORKDIRS/$WORKDIR/walrus-relay/status.yaml" ]; then
            echo "  status.yaml content:"
            cat "$WORKDIRS/$WORKDIR/walrus-relay/status.yaml"
        fi

        fail "$WORKDIR start should have started walrus-upload-relay process, but no process found"
    fi

    echo "  ✓ walrus-upload-relay process is running"

    # Verify status shows OK
    local status_output
    status_output=$("$SUIBASE_DIR/scripts/$WORKDIR" status 2>&1)

    if ! echo "$status_output" | grep -q "Walrus Relay.*OK"; then
        echo "  Unexpected status output: $status_output"
        fail "$WORKDIR status should show 'Walrus Relay : OK' after successful start"
    fi

    echo "  ✓ Status correctly shows Walrus Relay as OK"
}

test_start_with_walrus_disabled() {
    echo "--- Test: $WORKDIR start should not start walrus relay when disabled ---"

    # Stop services first
    "$SUIBASE_DIR/scripts/$WORKDIR" stop

    # Disable walrus relay
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable

    # Verify walrus relay is disabled in config
    if ! grep -q "^walrus_relay_enabled: false" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "walrus_relay_enabled should be false in suibase.yaml"
    fi

    echo "  Config shows walrus_relay_enabled: false"

    # Start $WORKDIR services
    echo "  Starting $WORKDIR services..."
    "$SUIBASE_DIR/scripts/$WORKDIR" start

    # Wait for services to settle - process should stay stopped when disabled
    wait_for_process_stopped "$WORKDIR" 5 >/dev/null 2>&1 || true

    # Check that walrus-upload-relay process is NOT running
    if check_walrus_process_running "$WORKDIR" >/dev/null 2>&1; then
        fail "$WORKDIR start should NOT have started walrus-upload-relay when disabled, but process is still running"
    fi

    echo "  ✓ No walrus-upload-relay process running (as expected)"

    # Verify status shows DISABLED
    local status_output
    status_output=$("$SUIBASE_DIR/scripts/$WORKDIR" status 2>&1)

    if ! echo "$status_output" | grep -q "Walrus Relay.*DISABLED"; then
        echo "  Unexpected status output: $status_output"
        fail "$WORKDIR status should show 'Walrus Relay : DISABLED' when disabled"
    fi

    echo "  ✓ Status correctly shows Walrus Relay as DISABLED"
}

test_config_variable_loading() {
    echo "--- Test: Configuration variables are properly loaded during start ---"

    # Enable walrus relay
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable

    # Create a test script that sources globals and checks the config variable
    cat > /tmp/test_config_loading.sh << EOF
#!/bin/bash
SUIBASE_DIR="\$HOME/suibase"
SCRIPT_COMMON_CALLER="test"
WORKDIR="$WORKDIR"
source "\$SUIBASE_DIR/scripts/common/__globals.sh" "\$SCRIPT_COMMON_CALLER" "\$WORKDIR"

echo "CFG_walrus_relay_enabled: \${CFG_walrus_relay_enabled:-UNDEFINED}"
echo "WORKDIR: \${WORKDIR:-UNDEFINED}"
echo "_SUPPORT_WALRUS_RELAY would be: \$([ "\${CFG_walrus_relay_enabled:-false}" = "true" ] && [ "\$WORKDIR" = "$WORKDIR" ] && echo "true" || echo "false")"
EOF
    chmod +x /tmp/test_config_loading.sh

    local config_output
    config_output=$(/tmp/test_config_loading.sh 2>&1)
    echo "  Config loading test output: $config_output"

    if echo "$config_output" | grep -q "CFG_walrus_relay_enabled: UNDEFINED"; then
        fail "CFG_walrus_relay_enabled should be loaded from suibase.yaml, not UNDEFINED"
    fi

    if ! echo "$config_output" | grep -q "CFG_walrus_relay_enabled: true"; then
        fail "CFG_walrus_relay_enabled should be 'true' when enabled"
    fi

    if ! echo "$config_output" | grep -q "_SUPPORT_WALRUS_RELAY would be: true"; then
        fail "_SUPPORT_WALRUS_RELAY logic should evaluate to true for enabled $WORKDIR"
    fi

    echo "  ✓ Configuration variables loaded correctly"

    # Cleanup
    rm -f /tmp/test_config_loading.sh
}

# Run tests in order
test_config_variable_loading
test_start_with_walrus_disabled
test_start_with_walrus_enabled

# Stop services to cleanup
"$SUIBASE_DIR/scripts/$WORKDIR" stop

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
echo "=== All $WORKDIR Start Walrus Relay Tests Passed! ==="
echo