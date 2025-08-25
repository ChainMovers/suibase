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
    sleep 2  # Wait a moment for processes to terminate
    if ! check_walrus_process_stopped "testnet"; then
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
    
    # Wait a moment for services to settle
    sleep 2
    
    echo "Step 4: Enable walrus relay"
    testnet wal-relay enable
    if [ $? -ne 0 ]; then
        echo "ERROR: Failed to enable walrus relay"
        return 1
    fi
    
    # Wait a moment for daemon to process the enable
    sleep 3
    
    echo "Step 5: Check walrus relay status"
    status_output=$(testnet wal-relay status 2>&1)
    echo "Status output: $status_output"
    
    # Check if status contains DOWN (the bug symptom)
    if echo "$status_output" | grep -q "DOWN"; then
        echo "BUG REPRODUCED: Walrus relay status shows DOWN after enable sequence"
        echo "Expected: Status should be OK or INITIALIZING, not DOWN"
        return 1
    elif echo "$status_output" | grep -q "OK"; then
        echo "SUCCESS: Walrus relay status shows OK (bug not present)"
        return 0
    elif echo "$status_output" | grep -q "INITIALIZING"; then
        echo "SUCCESS: Walrus relay status shows INITIALIZING (transitioning, acceptable)"
        return 0
    else
        echo "UNEXPECTED: Walrus relay status shows neither DOWN, OK, nor INITIALIZING"
        echo "Status output: $status_output"
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