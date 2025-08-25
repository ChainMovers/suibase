#!/bin/bash

# Test that walrus relay enable/disable operations preserve all other config integrity
# This ensures no collateral damage to other settings in suibase.yaml

set -e  # Exit on any error

# Load common test functions
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$script_dir/__test_common.sh"

# Test plan
echo "=== Testing Walrus Relay Config Integrity ==="
echo "Testing: Enable/disable preserves other suibase.yaml configurations"
echo

# Setup test environment
setup_test_workdir "testnet"
backup_config_files "testnet"

TEMP_CONFIG_FILE="/tmp/suibase_wal_relay_config_test_$$"

create_test_config() {
    local config_file="$1"
    local walrus_setting="$2"  # "true", "false", or "" (omit)

    cat > "$config_file" << EOF
# Test configuration for walrus relay integrity testing
# This file contains various settings that should remain unchanged

# Examples
# ========
precompiled_bin: false
default_repo_branch: "main"
force_tag: "mainnet-v1.18.0"
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
    rpc: "https://fullnode.testnet.sui.io:443"
    priority: 20
    monitored: true
    selectable: true

# Proxy settings
proxy_enabled: true
proxy_host_ip: "localhost"
proxy_port_number: 44342

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

test_enable_from_missing() {
    echo "--- Test: Enable when walrus_relay_enabled is missing ---"

    # Create config without walrus_relay_enabled
    create_test_config "$TEMP_CONFIG_FILE" ""
    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/testnet/suibase.yaml"

    # Enable walrus relay
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable >/dev/null 2>&1

    # Verify integrity
    verify_config_integrity "$WORKDIRS/testnet/suibase.yaml" "true" "enable from missing"

    echo "✓ Enable from missing preserves config integrity"
}

test_enable_from_false() {
    echo "--- Test: Enable when walrus_relay_enabled is false ---"

    # Create config with walrus_relay_enabled: false
    create_test_config "$TEMP_CONFIG_FILE" "false"
    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/testnet/suibase.yaml"

    # Enable walrus relay
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable >/dev/null 2>&1

    # Verify integrity
    verify_config_integrity "$WORKDIRS/testnet/suibase.yaml" "true" "enable from false"

    echo "✓ Enable from false preserves config integrity"
}

test_disable_from_true() {
    echo "--- Test: Disable when walrus_relay_enabled is true ---"

    # Create config with walrus_relay_enabled: true
    create_test_config "$TEMP_CONFIG_FILE" "true"
    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/testnet/suibase.yaml"

    # Disable walrus relay
    "$SUIBASE_DIR/scripts/testnet" wal-relay disable >/dev/null 2>&1

    # Verify integrity
    verify_config_integrity "$WORKDIRS/testnet/suibase.yaml" "false" "disable from true"

    echo "✓ Disable from true preserves config integrity"
}

test_config_with_surrounding_walrus_settings() {
    echo "--- Test: Config with other walrus-related settings ---"

    # Create config with walrus-related settings around walrus_relay_enabled
    cat > "$TEMP_CONFIG_FILE" << EOF
# Config with various walrus settings
walrus_bin_url: "https://github.com/MystenLabs/walrus"
walrus_network: "testnet"
walrus_relay_enabled: false
walrus_relay_proxy_port: 45852
walrus_relay_local_port: 45802
walrus_config_file: "config/walrus-config.yaml"
EOF

    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/testnet/suibase.yaml"

    # Enable walrus relay
    "$SUIBASE_DIR/scripts/testnet" wal-relay enable >/dev/null 2>&1

    # Verify all walrus settings are preserved
    if ! grep -q "^walrus_bin_url: \"https://github.com/MystenLabs/walrus\"" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "walrus_bin_url was lost or modified"
    fi

    if ! grep -q "^walrus_network: \"testnet\"" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "walrus_network was lost or modified"
    fi

    if ! grep -q "^walrus_relay_enabled: true" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "walrus_relay_enabled was not updated correctly"
    fi

    if ! grep -q "^walrus_relay_proxy_port: 45852" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "walrus_relay_proxy_port was lost or modified"
    fi

    if ! grep -q "^walrus_relay_local_port: 45802" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "walrus_relay_local_port was lost or modified"
    fi

    if ! grep -q "^walrus_config_file: \"config/walrus-config.yaml\"" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "walrus_config_file was lost or modified"
    fi

    echo "✓ Surrounding walrus settings preserved correctly"
}

test_edge_case_line_positions() {
    echo "--- Test: Edge case line positions ---"

    # Test when walrus_relay_enabled is first line
    echo "walrus_relay_enabled: false" > "$TEMP_CONFIG_FILE"
    echo "other_setting: value" >> "$TEMP_CONFIG_FILE"
    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/testnet/suibase.yaml"

    "$SUIBASE_DIR/scripts/testnet" wal-relay enable >/dev/null 2>&1

    if ! grep -q "^walrus_relay_enabled: true" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "Failed to update walrus_relay_enabled when it's the first line"
    fi

    if ! grep -q "^other_setting: value" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "other_setting was lost when walrus_relay_enabled was first line"
    fi

    # Test when walrus_relay_enabled is last line
    echo "other_setting: value" > "$TEMP_CONFIG_FILE"
    echo "walrus_relay_enabled: true" >> "$TEMP_CONFIG_FILE"
    cp "$TEMP_CONFIG_FILE" "$WORKDIRS/testnet/suibase.yaml"

    "$SUIBASE_DIR/scripts/testnet" wal-relay disable >/dev/null 2>&1

    if ! grep -q "^walrus_relay_enabled: false" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "Failed to update walrus_relay_enabled when it's the last line"
    fi

    if ! grep -q "^other_setting: value" "$WORKDIRS/testnet/suibase.yaml"; then
        fail "other_setting was lost when walrus_relay_enabled was last line"
    fi

    echo "✓ Edge case line positions handled correctly"
}

# Store original configs
ORIGINAL_TESTNET_CONFIG=""
if [ -f "$WORKDIRS/testnet/suibase.yaml" ]; then
    ORIGINAL_TESTNET_CONFIG=$(cat "$WORKDIRS/testnet/suibase.yaml")
fi

# Run tests
test_enable_from_missing
test_enable_from_false
test_disable_from_true
test_config_with_surrounding_walrus_settings
test_edge_case_line_positions

# Restore original configs
echo "--- Restoring original configurations ---"
if [ -n "$ORIGINAL_TESTNET_CONFIG" ]; then
    echo "$ORIGINAL_TESTNET_CONFIG" > "$WORKDIRS/testnet/suibase.yaml"
    echo "✓ Restored original testnet config"
fi

# Cleanup
rm -f "$TEMP_CONFIG_FILE"

echo
echo "=== All Walrus Relay Config Integrity Tests Passed! ==="
echo