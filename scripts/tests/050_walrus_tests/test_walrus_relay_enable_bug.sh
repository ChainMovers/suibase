#!/bin/bash

# Test to reproduce bug where walrus relay status remains DOWN after enabling
# This test follows the exact sequence that triggers the bug

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=SCRIPTDIR/__test_common.sh
source "$SCRIPT_DIR/__test_common.sh"

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
    # Wait for status to be OK or INITIALIZING (not DOWN) within 15 seconds
    if wait_for_walrus_relay_status "testnet" "OK|INITIALIZING" 15 true; then
        echo "SUCCESS: Walrus relay reached expected status (OK or INITIALIZING)"
        return 0
    else
        echo "BUG REPRODUCED: Walrus relay failed to reach expected status within 15 seconds"
        echo "Expected: Status should be OK or INITIALIZING, not DOWN"
        return 1
    fi
}

# Run the test
echo "=== Walrus Relay Enable Bug Test ==="
if test_walrus_relay_enable_bug; then
    echo "✓ Test PASSED - No bugs detected"
    exit 0
else
    echo "✗ Test FAILED - Bug detected or test error"
    exit 1
fi