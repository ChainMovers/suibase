#!/bin/bash

# Test that 'testnet start' properly starts walrus-upload-relay when walrus_relay_enabled: true
# This test reproduces the bug where testnet start doesn't start walrus relay despite being enabled

set -e  # Exit on any error

# Load common test functions
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"

# Test plan
echo "=== Testing Testnet Start Walrus Relay Integration ==="
echo "Testing: 'testnet start' should start walrus-upload-relay when enabled"
echo

# Setup test environment with clean state
setup_clean_environment
setup_test_workdir "testnet"
backup_config_files "testnet"

# Ensure we're using BUILD version of suibase-daemon for walrus relay features
ensure_build_daemon

# Ensure clean state by stopping any running testnet services
echo "Ensuring clean state..."
"$SUIBASE_DIR/scripts/testnet" stop || true

# Store original config state
ORIGINAL_CONFIG_STATE=""
if grep -q "^walrus_relay_enabled:" "$WORKDIRS/testnet/suibase.yaml" 2>/dev/null; then
    ORIGINAL_CONFIG_STATE=$(grep "^walrus_relay_enabled:" "$WORKDIRS/testnet/suibase.yaml")
fi

test_testnet_start_with_walrus_enabled() {
    echo "--- Test: testnet start should start walrus relay when enabled ---"
    
    # Ensure walrus relay is enabled
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable
    
    # Stop any existing processes first
    "$SUIBASE_DIR/scripts/testnet" stop
    
    # Verify walrus relay is enabled in config
    if ! grep -q "^walrus_relay_enabled: true" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "walrus_relay_enabled should be true in suibase.yaml"
    fi
    
    echo "  Config shows walrus_relay_enabled: true"
    
    # Start testnet services
    local start_output
    start_output=$("$SUIBASE_DIR/scripts/testnet" start 2>&1)
    echo "  Start output: $start_output"
    
    # Wait for walrus relay process to be running or status to be ready
    wait_for_walrus_relay_status "testnet" "OK|DOWN|INITIALIZING" 10 >/dev/null 2>&1 || true
    
    # Check if walrus-upload-relay process is running
    local walrus_pid
    walrus_pid=$(pgrep -f "walrus-upload-relay" || true)
    
    if [ -z "$walrus_pid" ]; then
        # Get status to see what happened
        local status_output
        status_output=$("$SUIBASE_DIR/scripts/testnet" status 2>&1)
        echo "  Status output: $status_output"
        
        # Check status.yaml for debugging
        if [ -f "$WORKDIRS/testnet/walrus-relay/status.yaml" ]; then
            echo "  status.yaml content:"
            cat "$WORKDIRS/testnet/walrus-relay/status.yaml"
        fi
        
        fail "testnet start should have started walrus-upload-relay process, but no process found"
    fi
    
    echo "  ✓ walrus-upload-relay process running with PID: $walrus_pid"
    
    # Verify status shows OK
    local status_output
    status_output=$("$SUIBASE_DIR/scripts/testnet" status 2>&1)
    
    if ! echo "$status_output" | grep -q "Walrus Relay.*OK"; then
        echo "  Unexpected status output: $status_output"
        fail "testnet status should show 'Walrus Relay : OK' after successful start"
    fi
    
    echo "  ✓ Status correctly shows Walrus Relay as OK"
}

test_testnet_start_with_walrus_disabled() {
    echo "--- Test: testnet start should not start walrus relay when disabled ---"
    
    # Stop services first
    "$SUIBASE_DIR/scripts/testnet" stop
    
    # Disable walrus relay
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable
    
    # Verify walrus relay is disabled in config
    if ! grep -q "^walrus_relay_enabled: false" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "walrus_relay_enabled should be false in suibase.yaml"
    fi
    
    echo "  Config shows walrus_relay_enabled: false"
    
    # Start testnet services
    local start_output
    start_output=$("$SUIBASE_DIR/scripts/testnet" start 2>&1)
    echo "  Start output: $start_output"
    
    # Wait for services to settle - process should stay stopped when disabled
    wait_for_process_stopped "testnet" 5 >/dev/null 2>&1 || true
    
    # Check that walrus-upload-relay process is NOT running
    local walrus_pid
    walrus_pid=$(pgrep -f "walrus-upload-relay" || true)
    
    if [ -n "$walrus_pid" ]; then
        fail "testnet start should NOT have started walrus-upload-relay when disabled, but found PID: $walrus_pid"
    fi
    
    echo "  ✓ No walrus-upload-relay process running (as expected)"
    
    # Verify status shows DISABLED
    local status_output
    status_output=$("$SUIBASE_DIR/scripts/testnet" status 2>&1)
    
    if ! echo "$status_output" | grep -q "Walrus Relay.*DISABLED"; then
        echo "  Unexpected status output: $status_output"
        fail "testnet status should show 'Walrus Relay : DISABLED' when disabled"
    fi
    
    echo "  ✓ Status correctly shows Walrus Relay as DISABLED"
}

test_config_variable_loading() {
    echo "--- Test: Configuration variables are properly loaded during start ---"
    
    # Enable walrus relay
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable
    
    # Create a test script that sources globals and checks the config variable
    cat > /tmp/test_config_loading.sh << 'EOF'
#!/bin/bash
SUIBASE_DIR="$HOME/suibase"
SCRIPT_COMMON_CALLER="test"
WORKDIR="testnet"
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"

echo "CFG_walrus_relay_enabled: ${CFG_walrus_relay_enabled:-UNDEFINED}"
echo "WORKDIR: ${WORKDIR:-UNDEFINED}"
echo "_SUPPORT_WALRUS_RELAY would be: $([ "${CFG_walrus_relay_enabled:-false}" = "true" ] && [ "$WORKDIR" = "testnet" ] && echo "true" || echo "false")"
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
        fail "_SUPPORT_WALRUS_RELAY logic should evaluate to true for enabled testnet"
    fi
    
    echo "  ✓ Configuration variables loaded correctly"
    
    # Cleanup
    rm -f /tmp/test_config_loading.sh
}

# Run tests in order
test_config_variable_loading
test_testnet_start_with_walrus_disabled
test_testnet_start_with_walrus_enabled

# Stop services to cleanup
"$SUIBASE_DIR/scripts/testnet" stop

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
echo "=== All Testnet Start Walrus Relay Tests Passed! ==="
echo