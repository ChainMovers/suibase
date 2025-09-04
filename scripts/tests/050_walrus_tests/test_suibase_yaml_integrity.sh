#!/bin/bash

# Test that walrus relay enable/disable operations preserve all other config integrity
# This ensures no collateral damage to other settings in suibase.yaml

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
echo "=== Testing Walrus Relay Config Integrity ==="
echo "Testing: Enable/disable preserves other suibase.yaml configurations"
echo

# Setup test environment
setup_test_workdir "$WORKDIR"
backup_config_files "$WORKDIR"

# Ensure we're using BUILD version of suibase-daemon for walrus relay features
ensure_build_daemon

TEMP_CONFIG_FILE="/tmp/suibase_wal_relay_config_test_$$"

create_test_config() {
    local config_file="$1"
    local walrus_setting="$2"  # "true", "false", or "" (omit)

    # Set workdir-specific settings
    local rpc_url proxy_port metrics_port local_port force_tag
    case "$WORKDIR" in
        "testnet")
            rpc_url="https://fullnode.testnet.sui.io:443"
            proxy_port="44342"
            force_tag="testnet-v1.18.0"
            metrics_port="45812"
            local_port="45802"
            ;;
        "mainnet")
            rpc_url="https://fullnode.mainnet.sui.io:443"
            proxy_port="44343"
            force_tag="mainnet-v1.18.0"
            metrics_port="45813"
            local_port="45803"
            ;;
        *)
            rpc_url="https://fullnode.testnet.sui.io:443"
            proxy_port="44342"
            force_tag="testnet-v1.18.0"
            metrics_port="45812"
            local_port="45802"
            ;;
    esac

    cat > "$config_file" << EOF
# Test configuration for walrus relay integrity testing
# This file contains various settings that should remain unchanged

# Examples
# ========
precompiled_bin: false
default_repo_branch: "main"
force_tag: "$force_tag"
enable_local_repo: true

# User addresses and keys
add_private_keys:
  - 0x0cdb9491ab9697379802b188cd3566920cbb095dccca3fd91765bb45b461c30f
autocoins_address: "0x7c3c5899e5443c6bb2c4080b6ca23bdf3856bd50d0dabfc524e1f6b6b84565c2"
autocoins_enabled: false
autocoins_mode: "stage"

# Network configuration
enable_default_links: false
links:
  - alias: "tsuip"
    rpc: "http://0.0.0.0:39000"
    priority: 10
    monitored: true
    selectable: true
  - alias: "sui.io"
    rpc: "$rpc_url"
    priority: 20
    monitored: true
    selectable: true

# Proxy settings
proxy_enabled: true
proxy_host_ip: "localhost"
proxy_port_number: $proxy_port

# Walrus relay settings
walrus_relay_proxy_port: 45852
walrus_relay_local_port: $local_port
walrus_relay_metrics_port: $metrics_port

EOF

    # Add walrus_relay_enabled if specified
    if [ -n "$walrus_setting" ]; then
        echo "walrus_relay_enabled: $walrus_setting" >> "$config_file"
    fi

    # Add some trailing config
    cat >> "$config_file" << EOF

# Additional settings that should be preserved
sui_explorer_enabled: true
sui_explorer_scheme: "https://"
sui_explorer_host_ip: "suiscan.xyz"

# Final comment that should remain
EOF
}

verify_config_integrity() {
    local config_file="$1"
    local expected_walrus_setting="$2"
    local test_name="$3"

    echo "  Verifying config integrity for: $test_name"

    # Check critical fields are preserved
    if ! grep -q "^precompiled_bin: false" "$config_file"; then
        fail "precompiled_bin setting was lost or modified"
    fi

    if ! grep -q "^default_repo_branch: \"main\"" "$config_file"; then
        fail "default_repo_branch setting was lost or modified"
    fi

    if ! grep -q "^autocoins_address: \"0x7c3c5899e5443c6bb2c4080b6ca23bdf3856bd50d0dabfc524e1f6b6b84565c2\"" "$config_file"; then
        fail "autocoins_address setting was lost or modified"
    fi

    # Check array/list structures are preserved
    if ! grep -q "add_private_keys:" "$config_file"; then
        fail "add_private_keys array was lost"
    fi

    if ! grep -q "  - 0x0cdb9491ab9697379802b188cd3566920cbb095dccca3fd91765bb45b461c30f" "$config_file"; then
        fail "add_private_keys content was lost or modified"
    fi

    if ! grep -q "links:" "$config_file"; then
        fail "links array was lost"
    fi

    if ! grep -q "  - alias: \"tsuip\"" "$config_file"; then
        fail "links content was lost or modified"
    fi

    if ! grep -q "    rpc: \"http://0.0.0.0:39000\"" "$config_file"; then
        fail "nested links content was lost or modified"
    fi

    # Check comments are preserved
    if ! grep -q "# Test configuration for walrus relay integrity testing" "$config_file"; then
        fail "Header comments were lost"
    fi

    if ! grep -q "# Final comment that should remain" "$config_file"; then
        fail "Trailing comments were lost"
    fi

    # Check walrus_relay_enabled has correct value
    if [ "$expected_walrus_setting" = "none" ]; then
        if grep -q "^walrus_relay_enabled:" "$config_file"; then
            fail "walrus_relay_enabled should not exist but was found"
        fi
    else
        if ! grep -q "^walrus_relay_enabled: $expected_walrus_setting" "$config_file"; then
            fail "walrus_relay_enabled should be '$expected_walrus_setting' but was: $(grep "^walrus_relay_enabled:" "$config_file" || echo "missing")"
        fi
    fi

    echo "  ✓ All config integrity checks passed for: $test_name"
}

get_walrus_relay_pid() {
    # Extract PID from "testnet wal-relay status" output
    # Returns empty string if no PID found
    local status_output
    status_output=$("$SUIBASE_DIR/scripts/$WORKDIR" wal-relay status 2>/dev/null)
    # Remove ANSI color codes first, then extract PID
    strip_ansi_colors "$status_output" | grep -o "pid [0-9]\+" | grep -o "[0-9]\+" | head -1
}

test_config_process_discrepancy() {
    echo "--- Test: Config-Process Discrepancy Scenarios ---"
    echo "Testing: Config and process state mismatches should not cause exit code 1"
    echo

    # Scenario 1: Process enabled → Config disabled → wal-relay enable
    echo "=== Scenario 1: enabled->disabled config, then wal-relay enable ==="

    # Start with enabled state - need both config enabled AND services running
    echo "Starting with wal-relay enable..."
    if ! "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable; then
        fail "Initial wal-relay enable failed"
    fi

    echo "Starting $WORKDIR services to actually run the walrus-upload-relay process..."
    if ! "$SUIBASE_DIR/scripts/$WORKDIR" start; then
        fail "Failed to start $WORKDIR services"
    fi

    # Get the PID of running process
    local pid1
    pid1=$(get_walrus_relay_pid)
    echo "Process PID after enable and start: $pid1"

    if [ -z "$pid1" ]; then
        fail "No PID found after wal-relay enable and start - process should be running"
    fi

    # Directly modify config to create discrepancy
    echo "Creating discrepancy: setting config to disabled while process runs..."
    if grep -q "^walrus_relay_enabled:" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        sed -i.bak "s/^walrus_relay_enabled:.*/walrus_relay_enabled: false/" "$WORKDIRS/$WORKDIR/suibase.yaml" && rm "$WORKDIRS/$WORKDIR/suibase.yaml.bak"
    else
        echo "walrus_relay_enabled: false" >> "$WORKDIRS/$WORKDIR/suibase.yaml"
    fi

    # Call wal-relay enable (should handle discrepancy gracefully)
    echo "Calling wal-relay enable to resolve discrepancy..."
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable
    local exit_code=$?

    if [ $exit_code -ne 0 ]; then
        fail "Scenario 1: wal-relay enable failed with exit code $exit_code"
    fi

    # Verify config is now enabled
    if ! grep -q "^walrus_relay_enabled: true" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "Scenario 1: Config should show enabled after wal-relay enable"
    fi

    echo "✓ Scenario 1 passed: discrepancy resolved without exit code 1"
    echo

    # Scenario 2: Process enabled → Config disabled → wal-relay disable
    echo "=== Scenario 2: enabled->disabled config, then wal-relay disable ==="

    # Start with enabled state (should already be enabled from scenario 1)
    local pid2
    pid2=$(get_walrus_relay_pid)
    echo "Process PID before config change: $pid2"

    # Directly modify config to disabled
    echo "Creating discrepancy: setting config to disabled while process runs..."
    
    # Check if suibase.yaml file exists before attempting to modify it
    if [ ! -f "$WORKDIRS/$WORKDIR/suibase.yaml" ]; then
        echo "ERROR: suibase.yaml file not found at $WORKDIRS/$WORKDIR/suibase.yaml"
        exit 1
    fi
    
    sed -i.bak "s/^walrus_relay_enabled:.*/walrus_relay_enabled: false/" "$WORKDIRS/$WORKDIR/suibase.yaml" && rm "$WORKDIRS/$WORKDIR/suibase.yaml.bak"

    # Call wal-relay disable (should handle discrepancy gracefully)
    echo "Calling wal-relay disable to resolve discrepancy..."
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable
    exit_code=$?

    if [ $exit_code -ne 0 ]; then
        fail "Scenario 2: wal-relay disable failed with exit code $exit_code"
    fi

    # Verify config shows disabled and process stopped
    if ! grep -q "^walrus_relay_enabled: false" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "Scenario 2: Config should show disabled after wal-relay disable"
    fi

    local pid3
    pid3=$(get_walrus_relay_pid)
    if [ -n "$pid3" ]; then
        fail "Process still running (PID: $pid3) after wal-relay disable - disable operation failed"
    else
        echo "✓ Process stopped as expected"
    fi

    echo "✓ Scenario 2 passed: discrepancy resolved without exit code 1"
    echo

    # Scenario 3: Process disabled → Config enabled → wal-relay enable
    echo "=== Scenario 3: disabled->enabled config, then wal-relay enable ==="

    # Start with disabled state (should be disabled from scenario 2)
    local pid4
    pid4=$(get_walrus_relay_pid)
    echo "Process PID before config change: $pid4"

    # Directly modify config to enabled
    echo "Creating discrepancy: setting config to enabled while process stopped..."
    
    # Check if suibase.yaml file exists before attempting to modify it
    if [ ! -f "$WORKDIRS/$WORKDIR/suibase.yaml" ]; then
        echo "ERROR: suibase.yaml file not found at $WORKDIRS/$WORKDIR/suibase.yaml"
        exit 1
    fi
    
    sed -i.bak "s/^walrus_relay_enabled:.*/walrus_relay_enabled: true/" "$WORKDIRS/$WORKDIR/suibase.yaml" && rm "$WORKDIRS/$WORKDIR/suibase.yaml.bak"

    # Call wal-relay enable (should handle discrepancy gracefully)
    echo "Calling wal-relay enable to resolve discrepancy..."
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable
    exit_code=$?

    if [ $exit_code -ne 0 ]; then
        fail "Scenario 3: wal-relay enable failed with exit code $exit_code"
    fi

    # Verify config shows enabled and process started
    if ! grep -q "^walrus_relay_enabled: true" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "Scenario 3: Config should show enabled after wal-relay enable"
    fi

    local pid5
    pid5=$(get_walrus_relay_pid)
    if [ -n "$pid5" ]; then
        echo "✓ New process started (PID: $pid5)"
        # Verify this is a new process (different from all previous PIDs)
        if [ "$pid5" = "$pid1" ] || [ "$pid5" = "$pid2" ] || [ "$pid5" = "$pid4" ]; then
            fail "PID $pid5 should be different from previous PIDs (pid1=$pid1, pid2=$pid2, pid4=$pid4) - new process expected"
        fi
        echo "✓ Confirmed new process with unique PID"
    else
        fail "No PID found after wal-relay enable - process should be running"
    fi

    echo "✓ Scenario 3 passed: discrepancy resolved without exit code 1"
    echo

    # Scenario 4: Process disabled → Config enabled → wal-relay disable
    echo "=== Scenario 4: disabled->enabled config, then wal-relay disable ==="

    # First ensure we have disabled state - disable the process
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable > /dev/null 2>&1

    local pid6
    pid6=$(get_walrus_relay_pid)
    echo "Process PID after explicit disable: $pid6"

    # Directly modify config to enabled
    echo "Creating discrepancy: setting config to enabled while process stopped..."
    
    # Check if suibase.yaml file exists before attempting to modify it
    if [ ! -f "$WORKDIRS/$WORKDIR/suibase.yaml" ]; then
        echo "ERROR: suibase.yaml file not found at $WORKDIRS/$WORKDIR/suibase.yaml"
        exit 1
    fi
    
    sed -i.bak "s/^walrus_relay_enabled:.*/walrus_relay_enabled: true/" "$WORKDIRS/$WORKDIR/suibase.yaml" && rm "$WORKDIRS/$WORKDIR/suibase.yaml.bak"

    # Call wal-relay disable (should handle discrepancy gracefully)
    echo "Calling wal-relay disable to resolve discrepancy..."
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable
    exit_code=$?

    if [ $exit_code -ne 0 ]; then
        fail "Scenario 4: wal-relay disable failed with exit code $exit_code"
    fi

    # Verify config shows disabled and no process running
    if ! grep -q "^walrus_relay_enabled: false" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "Scenario 4: Config should show disabled after wal-relay disable"
    fi

    local pid7
    pid7=$(get_walrus_relay_pid)
    if [ -n "$pid7" ]; then
        fail "Process still running (PID: $pid7) after wal-relay disable - disable operation failed"
    else
        echo "✓ No process running as expected"
    fi

    echo "✓ Scenario 4 passed: discrepancy resolved without exit code 1"
    echo

    echo "✓ All config-process discrepancy scenarios handled correctly"
}

test_enable_from_missing() {
    echo "--- Test: Enable when walrus_relay_enabled is missing ---"

    # Clean up any running processes first
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable >/dev/null 2>&1 || true

    # Create config without walrus_relay_enabled
    create_test_config "$TEMP_CONFIG_FILE" ""
    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/$WORKDIR/suibase.yaml"

    # Enable walrus relay
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable

    # Verify integrity
    verify_config_integrity "$WORKDIRS/$WORKDIR/suibase.yaml" "true" "enable from missing"

    echo "✓ Enable from missing preserves config integrity"
}

test_enable_from_false() {
    echo "--- Test: Enable when walrus_relay_enabled is false ---"

    # Clean up any running processes first
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable >/dev/null 2>&1 || true

    # Create config with walrus_relay_enabled: false
    create_test_config "$TEMP_CONFIG_FILE" "false"
    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/$WORKDIR/suibase.yaml"

    # Enable walrus relay
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable

    # Verify integrity
    verify_config_integrity "$WORKDIRS/$WORKDIR/suibase.yaml" "true" "enable from false"

    echo "✓ Enable from false preserves config integrity"
}

test_disable_from_true() {
    echo "--- Test: Disable when walrus_relay_enabled is true ---"

    # Clean up any running processes first
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable >/dev/null 2>&1 || true

    # Create config with walrus_relay_enabled: true
    create_test_config "$TEMP_CONFIG_FILE" "true"
    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/$WORKDIR/suibase.yaml"

    # Disable walrus relay
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable

    # Verify integrity
    verify_config_integrity "$WORKDIRS/$WORKDIR/suibase.yaml" "false" "disable from true"

    echo "✓ Disable from true preserves config integrity"
}

test_config_with_surrounding_walrus_settings() {
    echo "--- Test: Config with other walrus-related settings ---"

    # Set workdir-specific port numbers
    local proxy_port local_port
    case "$WORKDIR" in
        "testnet") proxy_port="45852"; local_port="45802" ;;
        "mainnet") proxy_port="45853"; local_port="45803" ;;
        *) proxy_port="45852"; local_port="45802" ;;
    esac

    # Create config with walrus-related settings around walrus_relay_enabled
    cat > "$TEMP_CONFIG_FILE" << EOF
# Config with various walrus settings
walrus_bin_url: "https://github.com/MystenLabs/walrus"
walrus_network: "$WORKDIR"
walrus_relay_enabled: false
walrus_relay_proxy_port: $proxy_port
walrus_relay_local_port: $local_port
walrus_config_file: "config/walrus-config.yaml"
EOF

    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/$WORKDIR/suibase.yaml"

    # Enable walrus relay
    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable

    # Verify all walrus settings are preserved
    if ! grep -q "^walrus_bin_url: \"https://github.com/MystenLabs/walrus\"" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "walrus_bin_url was lost or modified"
    fi

    if ! grep -q "^walrus_network: \"$WORKDIR\"" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "walrus_network was lost or modified"
    fi

    if ! grep -q "^walrus_relay_enabled: true" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "walrus_relay_enabled was not updated correctly"
    fi

    if ! grep -q "^walrus_relay_proxy_port: $proxy_port" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "walrus_relay_proxy_port was lost or modified"
    fi

    if ! grep -q "^walrus_relay_local_port: $local_port" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "walrus_relay_local_port was lost or modified"
    fi

    if ! grep -q "^walrus_config_file: \"config/walrus-config.yaml\"" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "walrus_config_file was lost or modified"
    fi

    echo "✓ Surrounding walrus settings preserved correctly"
}

test_edge_case_line_positions() {
    echo "--- Test: Edge case line positions ---"

    # Just to help knowing the initial state before the enable/disable tests.
    "$SUIBASE_DIR/scripts/$WORKDIR" status

    # Test when walrus_relay_enabled is first line
    echo "walrus_relay_enabled: false" > "$TEMP_CONFIG_FILE"
    echo "other_setting: value" >> "$TEMP_CONFIG_FILE"
    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/$WORKDIR/suibase.yaml"

    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay enable

    if ! grep -q "^walrus_relay_enabled: true" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "Failed to update walrus_relay_enabled when it's the first line"
    fi

    if ! grep -q "^other_setting: value" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "other_setting was lost when walrus_relay_enabled was first line"
    fi
    echo "✓ First line test passed"

    # Test when walrus_relay_enabled is last line
    echo "Testing: walrus_relay_enabled as last line"
    echo "other_setting: value" > "$TEMP_CONFIG_FILE"
    echo "walrus_relay_enabled: true" >> "$TEMP_CONFIG_FILE"
    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/$WORKDIR/suibase.yaml"

    "$SUIBASE_DIR/scripts/$WORKDIR" wal-relay disable

    if ! grep -q "^walrus_relay_enabled: false" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "Failed to update walrus_relay_enabled when it's the last line"
    fi

    if ! grep -q "^other_setting: value" "$WORKDIRS/$WORKDIR/suibase.yaml"; then
        fail "other_setting was lost when walrus_relay_enabled was last line"
    fi

    echo "✓ Edge case line positions handled correctly"
}

# Run tests. Stop suibase-daemon while doing these fist set of tests that
# modify suibase.yaml "wildly".

"$SUIBASE_DIR/scripts/dev/stop-daemon"

test_enable_from_missing
test_enable_from_false
test_disable_from_true
test_config_with_surrounding_walrus_settings
test_edge_case_line_positions

# Restore suibase.yaml from default template (handles both testnet and mainnet)
restore_suibase_yaml_from_default "$WORKDIR"
"$SUIBASE_DIR/scripts/dev/update-daemon"

test_config_process_discrepancy

restore_suibase_yaml_from_default "$WORKDIR"

# Cleanup
rm -f "$TEMP_CONFIG_FILE"

echo
echo "=== All Walrus Relay Config Integrity Tests Passed! ==="
echo