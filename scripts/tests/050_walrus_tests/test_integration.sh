#!/bin/bash

# Integration test for Phase 1 walrus-upload-relay implementation
# Tests the complete integration with suibase update commands and workdir management

__ORIGINAL_DIR=$PWD
cd "$( dirname "${BASH_SOURCE[0]}" )" || exit 1

# Source the common test infrastructure
source __test_common.sh

test_testnet_update_integration() {
    echo "Testing testnet update integration with walrus-upload-relay..."
    
    # Clean up any existing testnet setup to start fresh
    local TEST_WORKDIR="$WORKDIRS/testnet"
    local TEST_BIN_DIR="$TEST_WORKDIR/bin"
    local TEST_CONFIG_DIR="$TEST_WORKDIR/config-default"
    
    echo "Backing up existing testnet workdir..."
    local BACKUP_DIR="$TEMP_TEST_DIR/testnet_backup"
    if [ -d "$TEST_WORKDIR" ]; then
        cp -r "$TEST_WORKDIR" "$BACKUP_DIR"
    fi
    
    # Remove walrus binaries to test fresh installation
    rm -f "$TEST_BIN_DIR/walrus" "$TEST_BIN_DIR/walrus-upload-relay" 2>/dev/null || true
    
    echo "Running 'testnet update' to trigger walrus binary installation..."
    
    # Run testnet update and capture output
    if ! "$SUIBASE_DIR/scripts/testnet" update; then
        fail "testnet update command failed"
    fi
    
    echo "✓ testnet update completed successfully"
    
    # Verify walrus-upload-relay binary was installed
    if [ -f "$TEST_BIN_DIR/walrus-upload-relay" ]; then
        echo "✓ walrus-upload-relay binary installed at $TEST_BIN_DIR/walrus-upload-relay"
        
        # Verify it's executable
        if [ -x "$TEST_BIN_DIR/walrus-upload-relay" ]; then
            echo "✓ walrus-upload-relay binary is executable"
        else
            fail "walrus-upload-relay binary is not executable"
        fi
        
        # Verify it can run
        if "$TEST_BIN_DIR/walrus-upload-relay" --help >/dev/null 2>&1; then
            echo "✓ walrus-upload-relay binary executes successfully"
        else
            fail "walrus-upload-relay binary execution failed"
        fi
    else
        fail "walrus-upload-relay binary not found after testnet update"
    fi
    
    # Verify standard walrus binary is also present (no regression)
    if [ -f "$TEST_BIN_DIR/walrus" ] && [ -x "$TEST_BIN_DIR/walrus" ]; then
        echo "✓ Standard walrus binary also present (no regression)"
    else
        fail "Standard walrus binary missing or not executable"
    fi
    
    echo "✓ testnet update integration test passed"
}

test_configuration_integration() {
    echo "Testing configuration file integration..."
    
    local TEST_WORKDIR="$WORKDIRS/testnet"
    local TEST_CONFIG_DIR="$TEST_WORKDIR/config-default"
    local WALRUS_CONFIG="$TEST_CONFIG_DIR/walrus-config.yaml"
    local RELAY_CONFIG="$TEST_CONFIG_DIR/relay-config.yaml"
    
    # Test 1: Verify walrus-config.yaml exists and has correct structure
    if [ -f "$WALRUS_CONFIG" ]; then
        echo "✓ walrus-config.yaml exists at $WALRUS_CONFIG"
        
        # Check for required sections
        if grep -q "contexts:" "$WALRUS_CONFIG" && \
           grep -q "testnet:" "$WALRUS_CONFIG" && \
           grep -q "system_object:" "$WALRUS_CONFIG" && \
           grep -q "exchange_objects:" "$WALRUS_CONFIG"; then
            echo "✓ walrus-config.yaml has required structure"
        else
            fail "walrus-config.yaml missing required sections"
        fi
        
        # Check for rpc_urls section (should be added by repair function)
        if grep -q "rpc_urls:" "$WALRUS_CONFIG"; then
            echo "✓ walrus-config.yaml has rpc_urls section"
        else
            fail "walrus-config.yaml missing rpc_urls section"
        fi
    else
        fail "walrus-config.yaml not found at $WALRUS_CONFIG"
    fi
    
    # Test 2: Test relay-config.yaml auto-creation
    # Remove it if it exists to test auto-creation
    rm -f "$RELAY_CONFIG"
    
    # Source the relay process functions to trigger config creation
    export CFG_walrus_relay_enabled="true"
    export CFG_walrus_relay_local_port="45802"
    
    # shellcheck source=SCRIPTDIR/../../common/__walrus-relay-process.sh  
    source "$SUIBASE_DIR/scripts/common/__walrus-relay-process.sh"
    
    # This should trigger relay-config.yaml creation
    if start_walrus_relay_process >/dev/null 2>&1; then
        # Stop the process immediately
        stop_walrus_relay_process >/dev/null 2>&1
    fi
    
    if [ -f "$RELAY_CONFIG" ]; then
        echo "✓ relay-config.yaml auto-created at $RELAY_CONFIG"
        
        # Verify content format
        if grep -q "tip_config: !no_tip" "$RELAY_CONFIG" && \
           grep -q "tx_freshness_threshold_secs:" "$RELAY_CONFIG" && \
           grep -q "tx_max_future_threshold:" "$RELAY_CONFIG"; then
            echo "✓ relay-config.yaml has correct format"
        else
            fail "relay-config.yaml has incorrect format"
        fi
    else
        fail "relay-config.yaml was not auto-created"
    fi
    
    echo "✓ Configuration integration test passed"
}

test_end_to_end_workflow() {
    echo "Testing end-to-end Phase 1 workflow..."
    
    local TEST_WORKDIR="$WORKDIRS/testnet"
    local TEST_BIN_DIR="$TEST_WORKDIR/bin"
    local TEST_CONFIG_DIR="$TEST_WORKDIR/config-default"
    
    # Step 1: Verify all components are in place
    echo "Step 1: Verifying all Phase 1 components..."
    
    if [ -f "$TEST_BIN_DIR/walrus-upload-relay" ] && \
       [ -f "$TEST_CONFIG_DIR/walrus-config.yaml" ] && \
       [ -f "$TEST_CONFIG_DIR/relay-config.yaml" ]; then
        echo "✓ All required files present"
    else
        fail "Missing required files for end-to-end test"
    fi
    
    # Step 2: Test process can start and respond
    echo "Step 2: Testing process startup and health check..."
    
    export CFG_walrus_relay_enabled="true"
    export CFG_walrus_relay_local_port="45802"
    
    # Clean up any existing processes
    cleanup_port_conflicts
    
    # Source process functions
    # shellcheck source=SCRIPTDIR/../../common/__walrus-relay-process.sh  
    source "$SUIBASE_DIR/scripts/common/__walrus-relay-process.sh"
    
    # Start the process
    if start_walrus_relay_process; then
        if [ -n "$WALRUS_RELAY_PROCESS_PID" ]; then
            echo "✓ Process started with PID $WALRUS_RELAY_PROCESS_PID"
            
            # Wait for process to be ready
            if wait_for_process_ready "45802" "/v1/tip-config" 15; then
                echo "✓ Process responding to health checks"
                
                # Test both health check endpoints
                if curl -s "http://localhost:45802/v1/tip-config" >/dev/null && \
                   curl -s "http://localhost:45802/v1/api" >/dev/null; then
                    echo "✓ Both health check endpoints working"
                else
                    fail "Health check endpoints not responding"
                fi
            else
                fail "Process failed to become ready"
            fi
            
            # Stop the process
            stop_walrus_relay_process
            echo "✓ Process stopped successfully"
        else
            fail "Process started but PID not set"
        fi
    else
        fail "Failed to start walrus-upload-relay process"
    fi
    
    echo "✓ End-to-end workflow test passed"
}

test_mainnet_integration() {
    echo "Testing mainnet workdir integration..."
    
    # Test that mainnet update also works
    local MAINNET_WORKDIR="$WORKDIRS/mainnet"
    local MAINNET_BIN_DIR="$MAINNET_WORKDIR/bin"
    
    echo "Backing up existing mainnet workdir..."
    local BACKUP_DIR="$TEMP_TEST_DIR/mainnet_backup"
    if [ -d "$MAINNET_WORKDIR" ]; then
        cp -r "$MAINNET_WORKDIR" "$BACKUP_DIR"
    fi
    
    # Remove walrus binaries to test fresh installation
    rm -f "$MAINNET_BIN_DIR/walrus" "$MAINNET_BIN_DIR/walrus-upload-relay" 2>/dev/null || true
    
    echo "Running 'mainnet update' to test mainnet integration..."
    
    if ! "$SUIBASE_DIR/scripts/mainnet" update; then
        fail "mainnet update command failed"
    fi
    
    # Verify walrus-upload-relay was installed
    if [ -f "$MAINNET_BIN_DIR/walrus-upload-relay" ] && [ -x "$MAINNET_BIN_DIR/walrus-upload-relay" ]; then
        echo "✓ walrus-upload-relay installed for mainnet"
    else
        fail "walrus-upload-relay not installed for mainnet"
    fi
    
    # Verify walrus-config.yaml exists for mainnet
    if [ -f "$MAINNET_WORKDIR/config-default/walrus-config.yaml" ]; then
        echo "✓ mainnet walrus-config.yaml exists"
    else
        fail "mainnet walrus-config.yaml missing"
    fi
    
    echo "✓ mainnet integration test passed"
}

# Main test execution
tests() {
    echo "Starting Phase 1 integration tests..."
    
    # Setup
    mkdir -p "$TEMP_TEST_DIR"
    
    # Run integration tests
    test_testnet_update_integration
    echo ""
    test_configuration_integration  
    echo ""
    test_end_to_end_workflow
    echo ""
    test_mainnet_integration
    
    echo "All Phase 1 integration tests passed!"
}

# Standard test framework execution
main() {
    tests
}

[ "${BASH_SOURCE[0]}" == "${0}" ] && main "$@"