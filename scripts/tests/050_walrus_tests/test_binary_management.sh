#!/bin/bash

# Test for walrus-upload-relay binary management
# Tests binary download, installation, and basic functionality

# shellcheck source=SCRIPTDIR/__test_common.sh
source "$(dirname "$0")/__test_common.sh"

test_binary_installation() {
    echo "Testing walrus binary installation..."
    
    setup_test_workdir "testnet"
    backup_config_files "testnet"
    
    # Test updating via proper script (should download walrus-upload-relay)
    echo "Calling testnet update to install walrus binaries..."
    "$SUIBASE_DIR/scripts/testnet" update >/dev/null 2>&1
    
    # Check if binary was installed
    assert_binary_exists "testnet"
    
    echo "✓ Binary installation test passed"
}

test_binary_execution() {
    echo "Testing walrus-upload-relay binary execution..."
    
    RELAY_BINARY="$WORKDIRS/testnet/bin/walrus-upload-relay"
    if [ -f "$RELAY_BINARY" ]; then
        # Test help command to verify binary works
        echo "Testing walrus-upload-relay --help..."
        if "$RELAY_BINARY" --help >/dev/null 2>&1; then
            echo "✓ walrus-upload-relay binary executes successfully"
        else
            fail "walrus-upload-relay binary failed to execute --help"
        fi
    else
        fail "walrus-upload-relay binary not found after installation"
    fi
    
    echo "✓ Binary execution test passed"
}

test_configuration_files() {
    echo "Testing configuration files exist..."
    
    # Verify walrus-config.yaml exists
    assert_config_file_exists "testnet" "walrus-config.yaml"
    
    echo "✓ Configuration files test passed"
}

test_walrus_integration() {
    echo "Testing integration with existing walrus system..."
    
    # Test that both walrus and walrus-upload-relay binaries coexist
    WALRUS_BINARY="$WORKDIRS/testnet/bin/walrus"
    RELAY_BINARY="$WORKDIRS/testnet/bin/walrus-upload-relay"
    
    if [ -f "$WALRUS_BINARY" ]; then
        echo "✓ Standard walrus binary available"
        
        # Test that walrus binary executes
        if "$WALRUS_BINARY" --help >/dev/null 2>&1; then
            echo "✓ Standard walrus binary executes successfully"
        fi
    else
        fail "Standard walrus binary not found: $WALRUS_BINARY"
    fi
    
    # Test that both binaries coexist
    if [ -f "$WALRUS_BINARY" ] && [ -f "$RELAY_BINARY" ]; then
        echo "✓ Both walrus and walrus-upload-relay binaries coexist successfully"
    else
        fail "Binary coexistence test failed"
    fi
    
    echo "✓ Integration test passed"
}

tests() {
    echo "Starting walrus-upload-relay binary management tests..."
    
    # Setup temp directory
    mkdir -p "$TEMP_TEST_DIR"
    
    # Run individual tests
    test_binary_installation
    test_binary_execution  
    test_configuration_files
    test_walrus_integration
    
    # Cleanup happens automatically on test setup, not at end
    
    echo "All walrus-upload-relay binary management tests passed!"
}

# Run the tests
tests