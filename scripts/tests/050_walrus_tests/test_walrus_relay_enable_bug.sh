#!/bin/bash

# Test to reproduce bug where walrus relay status remains DOWN after enabling
# This test follows the exact sequence that triggers the bug

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$SCRIPT_DIR/__test_common.sh"

test_walrus_relay_disable_bug() {
    echo "Testing walrus relay disable bug reproduction..."
    
    # Sequence to test disable bug:
    # 1. Enable walrus relay
    # 2. Start testnet services (walrus process should be running)
    # 3. Disable walrus relay and check if it stops the running process
    
    echo "Step 1: Enable walrus relay"
    if ! testnet wal-relay enable; then
        echo "ERROR: Failed to enable walrus relay"
        return 1
    fi
    
    echo "Step 2: Start testnet services"
    if ! testnet start; then
        echo "ERROR: Failed to start testnet services"
        return 1
    fi
    
    echo "Step 3: Wait for walrus relay to reach OK status"
    # Wait for walrus process to be running before testing disable
    if ! wait_for_walrus_relay_status "testnet" "OK" 15 true; then
        echo "ERROR: Walrus relay did not reach OK status - cannot test disable functionality"
        return 1
    fi
    
    echo "Step 4: Disable walrus relay and check for process stop message"
    disable_output=$(testnet wal-relay disable 2>&1)
    echo "Disable output: $disable_output"
    
    echo "Step 5: Verify disable command stopped the running process"
    # Should see "Stopping twalrus-upload-relay (PID [number])" in output
    if echo "$disable_output" | grep -q "Stopping twalrus-upload-relay"; then
        echo "SUCCESS: Disable command properly stopped the running walrus process"
        return 0
    else
        echo "BUG REPRODUCED: Disable command did not stop the running walrus process"
        echo "Expected: Output should contain 'Stopping twalrus-upload-relay'"
        echo "Actual output: $disable_output"
        return 1
    fi
}

test_walrus_relay_enable_bug() {
    echo "Testing walrus relay enable bug reproduction..."
    
    # Sequence to reproduce the bug:
    echo "Step 1: Disable walrus relay"
    testnet wal-relay disable
    if [ $? -ne 0 ]; then
        echo "ERROR: Failed to disable walrus relay"
        return 1
    fi
    
    echo "Step 2: Stop testnet services"
    testnet stop
    if [ $? -ne 0 ]; then
        echo "ERROR: Failed to stop testnet services"
        return 1
    fi
    
    echo "Step 2a: Verify walrus relay process is stopped after 'testnet stop'"
    # Wait for processes to terminate
    if ! wait_for_process_stopped "testnet" 5; then
        echo "BUG DETECTED: 'testnet stop' did not stop twalrus-upload-relay process"
        return 1
    fi
    
    echo "Step 3: Start testnet services and check for stop/start anomaly"
    start_output=$(testnet start 2>&1)
    
    # Check for the anomaly: start saying "already running" after stop
    if echo "$start_output" | grep -q "already running"; then
        echo "BUG DETECTED: 'testnet start' says 'already running' after 'testnet stop'"
        echo "This indicates stop/start detection logic is broken"
        echo "Start output: $start_output"
        return 1
    fi
    
    # Check if start command succeeded
    if [ $? -ne 0 ]; then
        echo "ERROR: Failed to start testnet services"
        echo "Start output: $start_output"
        return 1
    fi
    echo "✓ Services properly detected as stopped and started correctly"
    
    # Wait for services to settle and be ready
    wait_for_walrus_relay_status "testnet" "OK|DOWN|DISABLED|INITIALIZING" 5 >/dev/null 2>&1 || true
    
    echo "Step 4: Enable walrus relay"
    testnet wal-relay enable
    if [ $? -ne 0 ]; then
        echo "ERROR: Failed to enable walrus relay"
        return 1
    fi
    
    echo "Step 5: Wait for walrus relay to reach expected status"
    # Wait for status to be OK (not DOWN) within 15 seconds
    # Since services are already running, enabling should start the process and show OK
    if wait_for_walrus_relay_status "testnet" "OK" 15 true; then
        echo "SUCCESS: Walrus relay reached expected status (OK)"
        return 0
    else
        echo "BUG REPRODUCED: Walrus relay failed to reach OK status within 15 seconds"
        echo "Expected: Status should be OK when enabled with services running, not DOWN"
        return 1
    fi
}

# Run both tests
echo "=== Walrus Relay Disable Bug Test ==="
disable_test_result=0
if test_walrus_relay_disable_bug; then
    echo "✓ Disable Test PASSED - No bugs detected"
else
    echo "✗ Disable Test FAILED - Bug detected or test error"
    disable_test_result=1
fi

echo ""
echo "=== Walrus Relay Enable Bug Test ==="
enable_test_result=0
if test_walrus_relay_enable_bug; then
    echo "✓ Enable Test PASSED - No bugs detected"
else
    echo "✗ Enable Test FAILED - Bug detected or test error"
    enable_test_result=1
fi

echo ""
echo "=== Test Summary ==="
if [ $disable_test_result -eq 0 ] && [ $enable_test_result -eq 0 ]; then
    echo "✓ ALL TESTS PASSED - No walrus relay bugs detected"
    exit 0
else
    if [ $disable_test_result -ne 0 ]; then
        echo "✗ Disable functionality has bugs"
    fi
    if [ $enable_test_result -ne 0 ]; then
        echo "✗ Enable functionality has bugs" 
    fi
    echo "✗ TESTS FAILED - Walrus relay bugs detected"
    exit 1
fi