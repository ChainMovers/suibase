#!/bin/bash

# Test to reproduce the exchange_objects disappearing bug
# This test specifically targets the issue where exchange_objects get lost
# during rpc_urls repair operations

__ORIGINAL_DIR=$PWD
cd "$( dirname "${BASH_SOURCE[0]}" )" || exit 1

# Source the common test infrastructure
source __test_common.sh

test_exchange_objects_preservation() {
    local TEST_CONFIG_DIR="$WORKDIRS/$WORKDIR/config-default"
    local TEST_WALRUS_CONFIG="$TEST_CONFIG_DIR/walrus-config.yaml"
    
    # Create test config directory if it doesn't exist
    mkdir -p "$TEST_CONFIG_DIR"
    
    echo "Testing exchange_objects preservation during full repair_walrus_config_as_needed..."
    
    # Test the FULL repair process, not just rpc_urls repair
    # This should trigger the bug by running all repair functions in sequence
    echo "Creating config that might trigger bug during full walrus config repair..."
    
    # Create a config with outdated system_object to trigger repair_yaml_root_field_as_needed
    cat > "$TEST_WALRUS_CONFIG" << 'EOF'
contexts:
  testnet:
    system_object: 0x0000000000000000000000000000000000000000000000000000000000000000
    staking_object: 0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3
    exchange_objects:
      - 0xf4d164ea2def5fe07dc573992a029e010dba09b1a8dcbc44c5c2e79567f39073
      - 0x19825121c52080bb1073662231cfea5c0e4d905fd13e95f21e9a018f2ef41862
      - 0x83b454e524c71f30803f4d6c302a86fb6a39e96cdfb873c2d1e93bc1c26a3bc5
      - 0x8d63209cf8589ce7aef8f262437163c67577ed09f3e636a9d8e0813843fb8bf1
    wallet_config:
      path: config/client.yaml
      active_env: testnet
    rpc_urls:
      - http://old-url-1.example.com:9999
      - http://old-url-2.example.com:8888
      - http://old-url-3.example.com:7777
default_context: testnet
EOF

    echo "Original config created. Counting exchange_objects..."
    local EXCHANGE_OBJECTS_BEFORE
    EXCHANGE_OBJECTS_BEFORE=$(grep -c -- "- 0x" "$TEST_WALRUS_CONFIG")
    echo "  Exchange objects before: $EXCHANGE_OBJECTS_BEFORE"
    
    # Verify we have 4 exchange objects initially
    if [ "$EXCHANGE_OBJECTS_BEFORE" -ne 4 ]; then
        fail "Test setup error: Expected 4 exchange objects, found $EXCHANGE_OBJECTS_BEFORE"
    fi
    
    echo "  Original exchange_objects:"
    grep -- "- 0x" "$TEST_WALRUS_CONFIG" | sed 's/^/    /'
    
    echo "Calling repair_walrus_config_as_needed (full repair process)..."
    # This should update both object IDs and rpc_urls but preserve exchange_objects
    repair_walrus_config_as_needed "$WORKDIR"
    if [ $? -eq 0 ] || [ $? -eq 1 ]; then  # Success or no changes needed
        echo "✓ rpc_urls repair function executed"
        
        # Count exchange objects after repair
        local EXCHANGE_OBJECTS_AFTER
        EXCHANGE_OBJECTS_AFTER=$(grep -c -- "- 0x" "$TEST_WALRUS_CONFIG")
        echo "  Exchange objects after: $EXCHANGE_OBJECTS_AFTER"
        
        echo "  Exchange objects after repair:"
        grep -- "- 0x" "$TEST_WALRUS_CONFIG" | sed 's/^/    /' || echo "    (none found)"
        
        # This is the critical test - exchange_objects should not be lost
        if [ "$EXCHANGE_OBJECTS_BEFORE" -eq "$EXCHANGE_OBJECTS_AFTER" ]; then
            echo "✓ All exchange_objects preserved during rpc_urls repair"
        else
            fail "BUG REPRODUCED: exchange_objects lost during rpc_urls repair"
            echo "  Expected: $EXCHANGE_OBJECTS_BEFORE exchange objects"
            echo "  Found: $EXCHANGE_OBJECTS_AFTER exchange objects"
            echo "  This is the bug we need to fix!"
            
            echo "  Full config after repair:"
            cat "$TEST_WALRUS_CONFIG" | sed 's/^/    /'
        fi
        
        # Verify that rpc_urls were actually updated
        if grep -q "http://localhost:44342" "$TEST_WALRUS_CONFIG"; then
            echo "✓ rpc_urls were correctly updated"
        else
            fail "rpc_urls were not updated as expected"
        fi
        
        # Verify old URLs are gone
        if ! grep -q "old-url" "$TEST_WALRUS_CONFIG"; then
            echo "✓ Old rpc_urls were removed"
        else
            fail "Old rpc_urls were not properly removed"
        fi
        
    else
        fail "rpc_urls repair function failed to execute"
    fi
    
    echo "Exchange objects preservation test completed."
}

# Main test execution
tests() {
    test_exchange_objects_preservation
}

# Standard test framework execution
main() {
    tests
}

[ "${BASH_SOURCE[0]}" == "${0}" ] && main "$@"