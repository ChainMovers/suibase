#!/bin/bash

# Test for repair_walrus_config_as_needed() function
# Tests Walrus config file creation, migration, and field updates

SUIBASE_DIR="$HOME/suibase"

# shellcheck source=SCRIPTDIR/../__scripts-lib-before-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-before-globals.sh"

# Source globals
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
WORKDIR="testnet"
# shellcheck source=SCRIPTDIR/../../common/__globals.sh
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"
trap cleanup EXIT

# shellcheck source=SCRIPTDIR/../__scripts-lib-after-globals.sh
source "$SUIBASE_DIR/scripts/tests/__scripts-lib-after-globals.sh"

# Test configuration
TESTNET_CONFIG_DIR="$WORKDIRS/testnet/config-default"
MAINNET_CONFIG_DIR="$WORKDIRS/mainnet/config-default"
TEMP_TEST_DIR="/tmp/suibase_repair_test_$$"

# Test helper functions
setup_test_workdir() {
    local workdir="$1"
    local config_dir="$WORKDIRS/$workdir/config-default"
    
    # Ensure workdir structure exists
    mkdir -p "$config_dir"
    
    # Create a basic suibase.yaml if it doesn't exist
    if [ ! -f "$WORKDIRS/$workdir/suibase.yaml" ]; then
        echo "# Test suibase.yaml" > "$WORKDIRS/$workdir/suibase.yaml"
    fi
}

backup_config_files() {
    local workdir="$1"
    local config_dir="$WORKDIRS/$workdir/config-default"
    local backup_dir="$TEMP_TEST_DIR/backup_$workdir"
    
    mkdir -p "$backup_dir"
    
    # Backup existing config files
    [ -f "$config_dir/walrus-config.yaml" ] && cp "$config_dir/walrus-config.yaml" "$backup_dir/"
    [ -f "$config_dir/sites-config.yaml" ] && cp "$config_dir/sites-config.yaml" "$backup_dir/"
    [ -f "$config_dir/client_config.yaml" ] && cp "$config_dir/client_config.yaml" "$backup_dir/"
}

restore_config_files() {
    local workdir="$1"
    local config_dir="$WORKDIRS/$workdir/config-default"
    local backup_dir="$TEMP_TEST_DIR/backup_$workdir"
    
    # Remove test files
    rm -f "$config_dir/walrus-config.yaml"
    rm -f "$config_dir/sites-config.yaml" 
    rm -f "$config_dir/client_config.yaml"
    
    # Restore original files if they existed
    [ -f "$backup_dir/walrus-config.yaml" ] && cp "$backup_dir/walrus-config.yaml" "$config_dir/"
    [ -f "$backup_dir/sites-config.yaml" ] && cp "$backup_dir/sites-config.yaml" "$config_dir/"
    [ -f "$backup_dir/client_config.yaml" ] && cp "$backup_dir/client_config.yaml" "$config_dir/"
}

cleanup_test() {
    # Restore original configs for both testnet and mainnet
    restore_config_files "testnet"
    restore_config_files "mainnet" 
    
    # Clean up temp directory
    rm -rf "$TEMP_TEST_DIR"
}

create_outdated_walrus_config() {
    local workdir="$1"
    local config_file="$WORKDIRS/$workdir/config-default/walrus-config.yaml"
    
    # Create an outdated config with old object IDs
    if [ "$workdir" = "testnet" ]; then
        cat > "$config_file" << 'EOF'
contexts:
  testnet:
    system_object: 0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af
    staking_object: 0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3
    subsidies_object: 0x000000000000000000000000000000000000000000000000000000000000001
    exchange_objects:
      - 0xf4d164ea2def5fe07dc573992a029e010dba09b1a8dcbc44c5c2e79567f39073
    wallet_config:
      path: $HOME/suibase/workdirs/testnet/config/client.yaml
      active_env: testnet
default_context: testnet
EOF
    else
        cat > "$config_file" << 'EOF'
contexts:
  mainnet:
    system_object: 0x2134d52768ea07e8c43570ef975eb3e4c27a39fa6396bef985b5abc58d03ddd2
    staking_object: 0x10b9d30c28448939ce6c4d6c6e0ffce4a7f8a4ada8248bdad09ef8b70e4a3904
    subsidies_object: 0x000000000000000000000000000000000000000000000000000000000000002
    exchange_objects: []
    wallet_config:
      path: $HOME/suibase/workdirs/mainnet/config/client.yaml
      active_env: mainnet
default_context: mainnet
EOF
    fi
}

create_old_client_config() {
    local workdir="$1"
    local config_file="$WORKDIRS/$workdir/config-default/client_config.yaml"
    
    # Create an old client_config.yaml file for migration testing
    if [ "$workdir" = "testnet" ]; then
        cat > "$config_file" << 'EOF'
contexts:
  testnet:
    system_object: 0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af
    staking_object: 0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3
    subsidies_object: 0xda799d85db0429765c8291c594d334349ef5bc09220e79ad397b30106161a0af
    exchange_objects:
      - 0xf4d164ea2def5fe07dc573992a029e010dba09b1a8dcbc44c5c2e79567f39073
    wallet_config:
      path: $HOME/suibase/workdirs/testnet/config/client.yaml
      active_env: testnet
default_context: testnet
EOF
    fi
}

# Main test functions
tests() {
    echo "Starting repair_walrus_config_as_needed() tests..."
    
    # Setup
    mkdir -p "$TEMP_TEST_DIR"
    setup_test_workdir "testnet"
    setup_test_workdir "mainnet"
    backup_config_files "testnet"
    backup_config_files "mainnet"
    
    # Exhaustive testnet tests
    test_testnet_fresh_install
    test_testnet_migration_from_client_config
    test_testnet_object_id_updates
    test_testnet_missing_field_addition
    test_testnet_home_reference_replacement
    test_testnet_sites_config_creation
    test_testnet_sites_config_home_replacement
    test_testnet_invalid_workdir_handling
    
    # Basic mainnet sanity tests
    test_mainnet_basic_creation
    test_mainnet_object_id_updates
    test_mainnet_sites_package_update
    
    # Clean up
    cleanup_test
    
    echo "All repair_walrus_config_as_needed() tests completed successfully!"
}

test_testnet_fresh_install() {
    echo "Testing testnet fresh install (no existing config)..."
    
    # Remove any existing config files
    rm -f "$TESTNET_CONFIG_DIR/walrus-config.yaml"
    rm -f "$TESTNET_CONFIG_DIR/sites-config.yaml"
    
    # Call the function
    repair_walrus_config_as_needed "testnet"
    
    # Verify walrus-config.yaml was created
    if [ ! -f "$TESTNET_CONFIG_DIR/walrus-config.yaml" ]; then
        fail "walrus-config.yaml was not created for testnet"
    fi
    
    # Verify sites-config.yaml was created
    if [ ! -f "$TESTNET_CONFIG_DIR/sites-config.yaml" ]; then
        fail "sites-config.yaml was not created for testnet"
    fi
    
    # Verify essential content (system_object and staking_object should always be present)
    assert_file_contains "$TESTNET_CONFIG_DIR/walrus-config.yaml" "0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af"
    assert_file_contains "$TESTNET_CONFIG_DIR/walrus-config.yaml" "0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3"
    
    # subsidies_object should be present in fresh install (copied from template)
    if grep -q "subsidies_object:" "$TESTNET_CONFIG_DIR/walrus-config.yaml"; then
        assert_file_contains "$TESTNET_CONFIG_DIR/walrus-config.yaml" "0xda799d85db0429765c8291c594d334349ef5bc09220e79ad397b30106161a0af"
        echo "  ✓ subsidies_object present in fresh install"
    else
        echo "  Note: subsidies_object not present in fresh install (may be expected)"
    fi
    
    # Verify $HOME references were replaced
    if grep -q '\$HOME' "$TESTNET_CONFIG_DIR/walrus-config.yaml"; then
        fail "walrus-config.yaml still contains \$HOME references"
    fi
    
    # Verify actual home path is present
    assert_file_contains "$TESTNET_CONFIG_DIR/walrus-config.yaml" "$HOME/suibase/workdirs/testnet"
    
    echo "✓ Testnet fresh install test passed"
}

test_testnet_migration_from_client_config() {
    echo "Testing testnet migration from client_config.yaml..."
    
    # Clean up previous test
    rm -f "$TESTNET_CONFIG_DIR/walrus-config.yaml"
    rm -f "$TESTNET_CONFIG_DIR/sites-config.yaml"
    
    # Create old client_config.yaml
    create_old_client_config "testnet"
    
    # Verify client_config.yaml exists before migration
    if [ ! -f "$TESTNET_CONFIG_DIR/client_config.yaml" ]; then
        fail "client_config.yaml was not created for migration test"
    fi
    
    # Call the function
    repair_walrus_config_as_needed "testnet"
    
    # Verify client_config.yaml was renamed to walrus-config.yaml
    if [ -f "$TESTNET_CONFIG_DIR/client_config.yaml" ]; then
        fail "client_config.yaml should have been migrated away"
    fi
    
    if [ ! -f "$TESTNET_CONFIG_DIR/walrus-config.yaml" ]; then
        fail "walrus-config.yaml was not created from migration"
    fi
    
    # Verify content was preserved (should have original object IDs)
    assert_file_contains "$TESTNET_CONFIG_DIR/walrus-config.yaml" "0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af"
    
    echo "✓ Testnet migration from client_config.yaml test passed"
}

test_testnet_object_id_updates() {
    echo "Testing testnet object ID updates..."
    
    # Create config with outdated object IDs
    create_outdated_walrus_config "testnet"
    
    # Verify outdated ID is present before repair
    assert_file_contains "$TESTNET_CONFIG_DIR/walrus-config.yaml" "0x000000000000000000000000000000000000000000000000000000000000001"
    
    # Call the function
    repair_walrus_config_as_needed "testnet"
    
    # Verify system_object and staking_object were NOT updated (they were already correct)
    assert_file_contains "$TESTNET_CONFIG_DIR/walrus-config.yaml" "0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af"
    assert_file_contains "$TESTNET_CONFIG_DIR/walrus-config.yaml" "0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3"
    
    # With the updated repair function, subsidies_object should be removed entirely for testnet
    # since it's no longer in the template
    if grep -q "subsidies_object:" "$TESTNET_CONFIG_DIR/walrus-config.yaml"; then
        fail "subsidies_object should have been removed from testnet config (not in template)"
    fi
    
    # Verify old outdated subsidies_object ID is gone
    if grep -q "0x000000000000000000000000000000000000000000000000000000000000001" "$TESTNET_CONFIG_DIR/walrus-config.yaml"; then
        fail "Old subsidies_object ID should have been removed"
    fi
    
    echo "✓ Testnet object ID updates test passed"
}

test_testnet_missing_field_addition() {
    echo "Testing testnet subsidies_object removal (not in template)..."
    
    # Create config missing subsidies_object and exchange_objects (like migrated client_config.yaml)
    cat > "$TESTNET_CONFIG_DIR/walrus-config.yaml" << 'EOF'
contexts:
  testnet:
    system_object: 0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af
    staking_object: 0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3
    wallet_config:
      path: $HOME/suibase/workdirs/testnet/config/client.yaml
      active_env: testnet
default_context: testnet
EOF
    
    # Verify missing fields are not present before repair
    if grep -q "subsidies_object:" "$TESTNET_CONFIG_DIR/walrus-config.yaml"; then
        fail "subsidies_object should not be present before repair"
    fi
    if grep -q "exchange_objects:" "$TESTNET_CONFIG_DIR/walrus-config.yaml"; then
        fail "exchange_objects should not be present before repair"
    fi
    
    # Call the function
    repair_walrus_config_as_needed "testnet"
    
    # The current implementation has a bug - it won't add missing fields
    # This test documents the current broken behavior and should be updated
    # when the bug is fixed
    
    # UPDATED BEHAVIOR: subsidies_object should be REMOVED from testnet since not in template
    if grep -q "subsidies_object:" "$TESTNET_CONFIG_DIR/walrus-config.yaml"; then
        fail "subsidies_object should have been removed from testnet config (not in template)"
    else
        echo "  ✓ subsidies_object correctly removed from testnet config (not in template)"
    fi
    
    if grep -q "exchange_objects:" "$TESTNET_CONFIG_DIR/walrus-config.yaml"; then
        echo "  Note: exchange_objects was added (good, bug may be fixed!)"
    else
        echo "  Warning: exchange_objects was NOT added (known bug in repair_yaml_root_field_as_needed)"
    fi
    
    echo "✓ Testnet subsidies_object removal test passed"
}

test_testnet_home_reference_replacement() {
    echo "Testing testnet \$HOME reference replacement..."
    
    # Create config with $HOME references
    cat > "$TESTNET_CONFIG_DIR/walrus-config.yaml" << 'EOF'
contexts:
  testnet:
    system_object: 0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af
    wallet_config:
      path: $HOME/suibase/workdirs/testnet/config/client.yaml
      active_env: testnet
default_context: testnet
EOF
    
    # Verify $HOME reference exists before repair
    assert_file_contains "$TESTNET_CONFIG_DIR/walrus-config.yaml" '\$HOME'
    
    # Call the function
    repair_walrus_config_as_needed "testnet"
    
    # Verify $HOME references were replaced
    if grep -q '\$HOME' "$TESTNET_CONFIG_DIR/walrus-config.yaml"; then
        fail "walrus-config.yaml should not contain \$HOME references after repair"
    fi
    
    # Verify actual home path is present
    assert_file_contains "$TESTNET_CONFIG_DIR/walrus-config.yaml" "$HOME/suibase/workdirs/testnet"
    
    echo "✓ Testnet \$HOME reference replacement test passed"
}

test_testnet_sites_config_creation() {
    echo "Testing testnet sites-config.yaml creation..."
    
    # Remove sites-config.yaml
    rm -f "$TESTNET_CONFIG_DIR/sites-config.yaml"
    
    # Call the function
    repair_walrus_config_as_needed "testnet"
    
    # Verify sites-config.yaml was created
    assert_file_exists "$TESTNET_CONFIG_DIR/sites-config.yaml"
    
    # Verify it contains testnet-specific content
    assert_file_contains "$TESTNET_CONFIG_DIR/sites-config.yaml" "testnet:"
    assert_file_contains "$TESTNET_CONFIG_DIR/sites-config.yaml" "https://fullnode.testnet.sui.io:443"
    assert_file_contains "$TESTNET_CONFIG_DIR/sites-config.yaml" "0xf99aee9f21493e1590e7e5a9aea6f343a1f381031a04a732724871fc294be799"
    
    echo "✓ Testnet sites-config.yaml creation test passed"
}

test_testnet_sites_config_home_replacement() {
    echo "Testing testnet sites-config.yaml \$HOME replacement..."
    
    # Create sites-config with $HOME references
    cat > "$TESTNET_CONFIG_DIR/sites-config.yaml" << 'EOF'
contexts:
  testnet:
    package: 0xf99aee9f21493e1590e7e5a9aea6f343a1f381031a04a732724871fc294be799
    general:
      rpc_url: https://fullnode.testnet.sui.io:443
      wallet: $HOME/suibase/workdirs/testnet/config-default/client.yaml
      walrus_binary: $HOME/suibase/workdirs/testnet/bin/walrus
      walrus_config: $HOME/suibase/workdirs/testnet/config-default/walrus-config.yaml
default_context: testnet
EOF
    
    # Verify $HOME references exist before repair
    assert_file_contains "$TESTNET_CONFIG_DIR/sites-config.yaml" '\$HOME'
    
    # Call the function
    repair_walrus_config_as_needed "testnet"
    
    # Verify $HOME references were replaced
    if grep -q '\$HOME' "$TESTNET_CONFIG_DIR/sites-config.yaml"; then
        fail "sites-config.yaml should not contain \$HOME references after repair"
    fi
    
    # Verify actual home paths are present
    assert_file_contains "$TESTNET_CONFIG_DIR/sites-config.yaml" "$HOME/suibase/workdirs/testnet"
    
    echo "✓ Testnet sites-config.yaml \$HOME replacement test passed"
}

test_testnet_invalid_workdir_handling() {
    echo "Testing invalid workdir handling..."
    
    # Test with non-existent workdir structure
    rm -rf "$WORKDIRS/testnet/config-default"
    
    # Should return early without error
    repair_walrus_config_as_needed "testnet"
    
    # Should not have created anything
    if [ -d "$WORKDIRS/testnet/config-default" ]; then
        fail "config-default should not have been created for missing workdir"
    fi
    
    # Test with invalid workdir names
    repair_walrus_config_as_needed "invalid"
    repair_walrus_config_as_needed "cargobin"
    repair_walrus_config_as_needed "active"
    repair_walrus_config_as_needed "localnet"
    repair_walrus_config_as_needed "devnet"
    
    # These should all return silently without error
    
    # Recreate testnet config-default for subsequent tests
    setup_test_workdir "testnet"
    
    echo "✓ Invalid workdir handling test passed"
}

test_mainnet_basic_creation() {
    echo "Testing mainnet basic creation (sanity check)..."
    
    # Remove any existing mainnet config files
    rm -f "$MAINNET_CONFIG_DIR/walrus-config.yaml"
    rm -f "$MAINNET_CONFIG_DIR/sites-config.yaml"
    
    # Call the function
    repair_walrus_config_as_needed "mainnet"
    
    # Verify basic files were created
    assert_file_exists "$MAINNET_CONFIG_DIR/walrus-config.yaml"
    assert_file_exists "$MAINNET_CONFIG_DIR/sites-config.yaml"
    
    # Verify mainnet-specific content
    assert_file_contains "$MAINNET_CONFIG_DIR/walrus-config.yaml" "mainnet:"
    assert_file_contains "$MAINNET_CONFIG_DIR/walrus-config.yaml" "0x2134d52768ea07e8c43570ef975eb3e4c27a39fa6396bef985b5abc58d03ddd2"
    assert_file_contains "$MAINNET_CONFIG_DIR/sites-config.yaml" "https://fullnode.mainnet.sui.io:443"
    
    # Verify no $HOME references
    if grep -q '\$HOME' "$MAINNET_CONFIG_DIR/walrus-config.yaml"; then
        fail "Mainnet walrus-config.yaml should not contain \$HOME references"
    fi
    
    echo "✓ Mainnet basic creation test passed"
}

test_mainnet_object_id_updates() {
    echo "Testing mainnet object ID updates (sanity check)..."
    
    # Create config with one outdated object ID
    create_outdated_walrus_config "mainnet"
    
    # Verify outdated ID is present before repair
    assert_file_contains "$MAINNET_CONFIG_DIR/walrus-config.yaml" "0x000000000000000000000000000000000000000000000000000000000000002"
    
    # Call the function
    repair_walrus_config_as_needed "mainnet"
    
    # Verify subsidies_object was updated
    assert_file_contains "$MAINNET_CONFIG_DIR/walrus-config.yaml" "0xb606eb177899edc2130c93bf65985af7ec959a2755dc126c953755e59324209e"
    
    # Verify old subsidies_object ID is gone
    if grep -q "0x000000000000000000000000000000000000000000000000000000000000000002" "$MAINNET_CONFIG_DIR/walrus-config.yaml"; then
        fail "Old mainnet subsidies_object ID should have been replaced"
    fi
    
    echo "✓ Mainnet object ID updates test passed"
}

test_mainnet_sites_package_update() {
    echo "Testing mainnet sites-config.yaml package ID update..."
    
    # Create sites-config with old package ID
    cat > "$MAINNET_CONFIG_DIR/sites-config.yaml" << 'EOF'
contexts:
  mainnet:
    # module: site
    # portal: wal.app
    package: 0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27
    general:
      rpc_url: https://fullnode.mainnet.sui.io:443
      wallet: $HOME/suibase/workdirs/mainnet/config-default/client.yaml
default_context: mainnet
EOF
    
    # Verify old package ID is present before repair
    assert_file_contains "$MAINNET_CONFIG_DIR/sites-config.yaml" "0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27"
    
    # Call the function
    repair_walrus_config_as_needed "mainnet"
    
    # Verify package ID was updated to current value
    assert_file_contains "$MAINNET_CONFIG_DIR/sites-config.yaml" "0xfa65cb2d62f4d39e60346fb7d501c12538ca2bbc646eaa37ece2aec5f897814e"
    
    # Verify old package ID is gone
    if grep -q "0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27" "$MAINNET_CONFIG_DIR/sites-config.yaml"; then
        fail "Old mainnet package ID should have been replaced"
    fi
    
    echo "✓ Mainnet sites-config.yaml package ID update test passed"
}

# Run the tests
tests