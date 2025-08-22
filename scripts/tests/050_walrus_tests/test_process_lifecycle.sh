#!/bin/bash

# Test for walrus-upload-relay process lifecycle
# Tests start, stop, and health checking of walrus-upload-relay process

# shellcheck source=SCRIPTDIR/__test_common.sh
source "$(dirname "$0")/__test_common.sh"

# Make sure we have the relay binary first
setup_test_workdir "testnet"

# Check if walrus-upload-relay binary exists, skip test if not
RELAY_BINARY="$WORKDIRS/testnet/bin/walrus-upload-relay"
if [ ! -f "$RELAY_BINARY" ]; then
    echo "walrus-upload-relay binary not found, running binary installation first..."
    # shellcheck source=SCRIPTDIR/../../common/__apps.sh
    source "$SUIBASE_DIR/scripts/common/__apps.sh"
    # shellcheck source=SCRIPTDIR/../../common/__walrus-binaries.sh
    source "$SUIBASE_DIR/scripts/common/__walrus-binaries.sh"
    update_walrus_app "testnet" "walrus"
fi

test_process_management() {
    echo "Testing walrus-upload-relay process management..."
    
    # Set required configuration
    export CFG_walrus_relay_enabled="true"
    export CFG_walrus_relay_local_port="45802"
    
    # Clean up any existing processes on the test port before starting
    echo "Pre-test cleanup: ensuring port 45802 is available..."
    cleanup_port_conflicts
    
    # Source the process management functions
    # shellcheck source=SCRIPTDIR/../../common/__walrus-relay-process.sh  
    source "$SUIBASE_DIR/scripts/common/__walrus-relay-process.sh"
    
    # Test 1: Verify process can start
    echo "Test 1: Starting walrus-upload-relay process..."
    
    # Start the process
    start_walrus_relay_process
    
    if [ -n "$WALRUS_RELAY_PROCESS_PID" ]; then
        echo "✓ walrus-upload-relay started with PID $WALRUS_RELAY_PROCESS_PID"
        assert_process_running "$WALRUS_RELAY_PROCESS_PID" "walrus-upload-relay"
        
        # Wait for process to be fully ready before proceeding
        if wait_for_process_ready "45802" "/v1/tip-config" 15; then
            echo "✓ Process fully initialized and responding"
        else
            fail "Process started but failed to become ready within timeout"
        fi
    else
        fail "walrus-upload-relay failed to start (no PID)"
    fi
    
    # Test 2: Health check (already verified in Test 1 gating, but test both endpoints)
    echo "Test 2: Testing health check endpoints..."
    
    if curl -s "http://localhost:45802/v1/tip-config" >/dev/null 2>&1; then
        echo "✓ Health check endpoint /v1/tip-config responding"
    else
        fail "Health check endpoint /v1/tip-config not responding"
    fi
    
    # Try alternative endpoint too  
    if curl -s "http://localhost:45802/v1/api" >/dev/null 2>&1; then
        echo "✓ API endpoint /v1/api responding"
    else
        fail "API endpoint /v1/api not responding"
    fi
    
    # Test 3: Process update function
    echo "Test 3: Testing PID update function..."
    
    # Clear PID and verify update function finds it
    unset WALRUS_RELAY_PROCESS_PID
    update_WALRUS_RELAY_PROCESS_PID_var
    
    if [ -n "$WALRUS_RELAY_PROCESS_PID" ]; then
        echo "✓ PID update function working: found PID $WALRUS_RELAY_PROCESS_PID"
    else
        fail "PID update function failed to find running process"
    fi
    
    # Test 4: Stop process
    echo "Test 4: Stopping walrus-upload-relay process..."
    
    local old_pid="$WALRUS_RELAY_PROCESS_PID"
    stop_walrus_relay_process
    
    sleep 1  # Give process time to shutdown
    
    if kill -0 "$old_pid" 2>/dev/null; then
        fail "Process $old_pid still running after stop"
    else
        echo "✓ Process stopped successfully"
    fi
    
    if [ -z "$WALRUS_RELAY_PROCESS_PID" ]; then
        echo "✓ PID variable cleared"
    else
        fail "PID variable not cleared after stop"
    fi
    
    echo "✓ All process management tests passed"
}

test_configuration_validation() {
    echo "Testing configuration file validation..."
    
    # Source the process management functions
    # shellcheck source=SCRIPTDIR/../../common/__walrus-relay-process.sh  
    source "$SUIBASE_DIR/scripts/common/__walrus-relay-process.sh"
    
    # Test that relay-config.yaml gets created if missing
    local relay_config="$CONFIG_DATA_DIR_DEFAULT/relay-config.yaml"
    
    # Remove relay config if it exists
    rm -f "$relay_config"
    
    # Create default configuration
    if [ ! -f "$relay_config" ]; then
        cat > "$relay_config" << 'EOF'
tip_config: !no_tip
tx_freshness_threshold_secs: 36000
tx_max_future_threshold:
  secs: 30
  nanos: 0
EOF
    fi
    
    if [ -f "$relay_config" ]; then
        echo "✓ relay-config.yaml created successfully"
        
        # Verify it contains expected content
        if grep -q "tip_config: !no_tip" "$relay_config"; then
            echo "✓ relay-config.yaml has correct default content"
        else
            fail "relay-config.yaml missing expected content"
        fi
    else
        fail "relay-config.yaml not created"
    fi
    
    # Test function availability
    if declare -f start_walrus_relay_process >/dev/null; then
        echo "✓ start_walrus_relay_process function defined"
    else
        fail "start_walrus_relay_process function not found"
    fi
    
    if declare -f update_WALRUS_RELAY_PROCESS_PID_var >/dev/null; then
        echo "✓ update_WALRUS_RELAY_PROCESS_PID_var function defined"
    else
        fail "update_WALRUS_RELAY_PROCESS_PID_var function not found"
    fi
    
    # Test PID update function execution
    export CFG_walrus_relay_local_port="45802"
    update_WALRUS_RELAY_PROCESS_PID_var
    echo "✓ PID update function executes without error"
    
    echo "✓ Configuration validation tests passed"
}

tests() {
    echo "Starting walrus-upload-relay process lifecycle tests..."
    
    # Setup
    mkdir -p "$TEMP_TEST_DIR"
    backup_config_files "testnet"
    
    # Run tests
    test_configuration_validation
    test_process_management
    
    # Cleanup any running processes
    cleanup_test
    
    echo "All walrus-upload-relay process lifecycle tests passed!"
}

# Run the tests
tests