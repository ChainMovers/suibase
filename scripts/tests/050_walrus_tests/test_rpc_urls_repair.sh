#!/bin/bash

# Test script for repair_walrus_rpc_urls_as_needed function
# This tests the smart rpc_urls configuration functionality

__ORIGINAL_DIR=$PWD
cd "$( dirname "${BASH_SOURCE[0]}" )" || exit 1

# Source the common test infrastructure
source __test_common.sh


# Source the walrus repair function for testing
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"

# Test function for rpc_urls repair functionality
test_rpc_urls_repair() {
    local TEST_CONFIG_DIR="$WORKDIRS/$WORKDIR/config-default"
    local TEST_WALRUS_CONFIG="$TEST_CONFIG_DIR/walrus-config.yaml"
    
    # Create test config directory if it doesn't exist
    mkdir -p "$TEST_CONFIG_DIR"
    
    echo "Testing rpc_urls repair functionality..."
    
    # Test 1: Create a walrus config without rpc_urls section
    echo "Test 1: Adding rpc_urls to config without existing section..."
    cat > "$TEST_WALRUS_CONFIG" << EOF
contexts:
  $WORKDIR:
    system_object: 0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af
    staking_object: 0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3
    exchange_objects:
      - 0xf4d164ea2def5fe07dc573992a029e010dba09b1a8dcbc44c5c2e79567f39073
      - 0x19825121c52080bb1073662231cfea5c0e4d905fd13e95f21e9a018f2ef41862
      - 0x83b454e524c71f30803f4d6c302a86fb6a39e96cdfb873c2d1e93bc1c26a3bc5
      - 0x8d63209cf8589ce7aef8f262437163c67577ed09f3e636a9d8e93bc1c26a3bc5
    wallet_config:
      path: config/client.yaml
      active_env: $WORKDIR
default_context: $WORKDIR
EOF

    # Call the repair function
    if repair_walrus_rpc_urls_as_needed "$TEST_WALRUS_CONFIG" "$WORKDIR"; then
        echo "✓ rpc_urls repair function executed successfully"
        
        # Verify the rpc_urls section was added
        if grep -q "rpc_urls:" "$TEST_WALRUS_CONFIG"; then
            echo "✓ rpc_urls section was added"
            
            # Verify it contains both proxy and direct RPC
            local expected_proxy_url="http://${CFG_proxy_host_ip}:${CFG_proxy_port_number}"
            if grep -q "$expected_proxy_url" "$TEST_WALRUS_CONFIG" && grep -q "https://fullnode.$WORKDIR.sui.io:443" "$TEST_WALRUS_CONFIG"; then
                echo "✓ Both proxy and direct RPC URLs are present"
            else
                fail "rpc_urls section doesn't contain expected URLs"
            fi
        else
            fail "rpc_urls section was not added"
        fi
    else
        fail "rpc_urls repair function failed"
    fi
    
    # Test 2: Replace existing rpc_urls with different content
    echo "Test 2: Replacing existing rpc_urls section..."
    cat > "$TEST_WALRUS_CONFIG" << EOF
contexts:
  $WORKDIR:
    system_object: 0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af
    staking_object: 0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3
    exchange_objects:
      - 0xf4d164ea2def5fe07dc573992a029e010dba09b1a8dcbc44c5c2e79567f39073
    wallet_config:
      path: config/client.yaml
      active_env: $WORKDIR
    rpc_urls:
      - http://old.example.com
default_context: $WORKDIR
EOF

    # Call the repair function
    if repair_walrus_rpc_urls_as_needed "$TEST_WALRUS_CONFIG" "$WORKDIR"; then
        echo "✓ rpc_urls repair function replaced existing section"
        
        # Verify the old URL is gone and new ones are present
        if ! grep -q "http://old.example.com" "$TEST_WALRUS_CONFIG"; then
            echo "✓ Old RPC URL was removed"
        else
            fail "Old RPC URL was not removed"
        fi
        
        local expected_proxy_url="http://${CFG_proxy_host_ip}:${CFG_proxy_port_number}"
        if grep -q "$expected_proxy_url" "$TEST_WALRUS_CONFIG" && grep -q "https://fullnode.$WORKDIR.sui.io:443" "$TEST_WALRUS_CONFIG"; then
            echo "✓ New smart RPC URLs are present"
        else
            fail "New smart RPC URLs are missing"
        fi
    else
        fail "rpc_urls repair function failed to replace existing section"
    fi
    
    # Test 3: No changes needed when config is already correct
    echo "Test 3: No changes when config is already correct..."
    
    # Save current content
    local BEFORE_CONTENT
    BEFORE_CONTENT=$(cat "$TEST_WALRUS_CONFIG")
    
    # Call repair function again
    if ! repair_walrus_rpc_urls_as_needed "$TEST_WALRUS_CONFIG" "$WORKDIR"; then
        echo "✓ No changes needed - function returned 1 as expected"
        
        # Verify content is unchanged
        local AFTER_CONTENT
        AFTER_CONTENT=$(cat "$TEST_WALRUS_CONFIG")
        
        if [ "$BEFORE_CONTENT" = "$AFTER_CONTENT" ]; then
            echo "✓ Config content unchanged when no repair needed"
        else
            fail "Config content was modified when no changes were needed"
        fi
    else
        fail "Function should return 1 when no changes are needed"
    fi
    
    echo "✓ All rpc_urls repair tests passed"
}

# Main test execution
tests() {
    test_rpc_urls_repair
}

# Standard test framework execution
main() {
    tests
}

[ "${BASH_SOURCE[0]}" == "${0}" ] && main "$@"