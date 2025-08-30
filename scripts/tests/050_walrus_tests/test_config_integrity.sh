#!/bin/bash

# Regression test for walrus-config.yaml integrity during rpc_urls repair
# This test ensures that the repair function doesn't lose other configuration lines

__ORIGINAL_DIR=$PWD
cd "$( dirname "${BASH_SOURCE[0]}" )" || exit 1

# Source the common test infrastructure
source __test_common.sh


# Source the walrus repair function for testing
SCRIPT_COMMON_CALLER="$(readlink -f "$0")"
source "$SUIBASE_DIR/scripts/common/__globals.sh" "$SCRIPT_COMMON_CALLER" "$WORKDIR"

# Test that repair function preserves all configuration lines
test_config_integrity() {
    local TEST_CONFIG_DIR="$WORKDIRS/$WORKDIR/config-default"
    local TEST_WALRUS_CONFIG="$TEST_CONFIG_DIR/walrus-config.yaml"
    
    # Create test config directory if it doesn't exist
    mkdir -p "$TEST_CONFIG_DIR"
    
    echo "Testing configuration integrity during rpc_urls repair..."
    
    # Test 1: Complete config without rpc_urls - ensure nothing is lost
    echo "Test 1: Adding rpc_urls to complete config without losing other lines..."
    cat > "$TEST_WALRUS_CONFIG" << EOF
contexts:
  $WORKDIR:
    system_object: 0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af
    staking_object: 0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3
    exchange_objects:
      - 0xf4d164ea2def5fe07dc573992a029e010dba09b1a8dcbc44c5c2e79567f39073
      - 0x19825121c52080bb1073662231cfea5c0e4d905fd13e95f21e9a018f2ef41862
      - 0x83b454e524c71f30803f4d6c302a86fb6a39e96cdfb873c2d1e93bc1c26a3bc5
      - 0x8d63209cf8589ce7aef8f262437163c67577ed09f3e636a9d8e0813843fb8bf1
    wallet_config:
      path: config/client.yaml
      active_env: $WORKDIR
default_context: $WORKDIR
EOF

    # Remember original line count and specific lines
    local ORIGINAL_LINES
    ORIGINAL_LINES=$(wc -l < "$TEST_WALRUS_CONFIG")
    
    # Key lines that must be preserved
    local SYSTEM_OBJECT_BEFORE
    local EXCHANGE_OBJECTS_COUNT_BEFORE
    local DEFAULT_CONTEXT_BEFORE
    SYSTEM_OBJECT_BEFORE=$(grep "system_object:" "$TEST_WALRUS_CONFIG")
    EXCHANGE_OBJECTS_COUNT_BEFORE=$(grep -c "0x" "$TEST_WALRUS_CONFIG")
    DEFAULT_CONTEXT_BEFORE=$(grep "default_context:" "$TEST_WALRUS_CONFIG")
    
    echo "  Original line count: $ORIGINAL_LINES"
    echo "  Exchange objects before: $EXCHANGE_OBJECTS_COUNT_BEFORE"
    echo "  Default context before: [$DEFAULT_CONTEXT_BEFORE]"
    
    # Call the repair function
    if repair_walrus_rpc_urls_as_needed "$TEST_WALRUS_CONFIG" "$WORKDIR"; then
        echo "✓ rpc_urls repair function executed successfully"
        
        # Verify critical lines are preserved
        local SYSTEM_OBJECT_AFTER
        local EXCHANGE_OBJECTS_COUNT_AFTER
        local DEFAULT_CONTEXT_AFTER
        SYSTEM_OBJECT_AFTER=$(grep "system_object:" "$TEST_WALRUS_CONFIG")
        EXCHANGE_OBJECTS_COUNT_AFTER=$(grep -c "0x" "$TEST_WALRUS_CONFIG")
        DEFAULT_CONTEXT_AFTER=$(grep "default_context:" "$TEST_WALRUS_CONFIG")
        
        echo "  Exchange objects after: $EXCHANGE_OBJECTS_COUNT_AFTER"
        echo "  Default context after: [$DEFAULT_CONTEXT_AFTER]"
        
        # Check system_object preserved
        if [ "$SYSTEM_OBJECT_BEFORE" = "$SYSTEM_OBJECT_AFTER" ]; then
            echo "✓ system_object line preserved"
        else
            fail "system_object line was modified or lost"
            echo "  Before: $SYSTEM_OBJECT_BEFORE"
            echo "  After: $SYSTEM_OBJECT_AFTER"
        fi
        
        # Check exchange_objects preserved
        if [ "$EXCHANGE_OBJECTS_COUNT_BEFORE" = "$EXCHANGE_OBJECTS_COUNT_AFTER" ]; then
            echo "✓ All exchange_objects preserved"
        else
            fail "exchange_objects were lost"
            echo "  Before count: $EXCHANGE_OBJECTS_COUNT_BEFORE"
            echo "  After count: $EXCHANGE_OBJECTS_COUNT_AFTER"
        fi
        
        # Check default_context preserved
        if [ "$DEFAULT_CONTEXT_BEFORE" = "$DEFAULT_CONTEXT_AFTER" ]; then
            echo "✓ default_context line preserved"
        else
            fail "default_context was modified or lost"
            echo "  Before: [$DEFAULT_CONTEXT_BEFORE]"
            echo "  After: [$DEFAULT_CONTEXT_AFTER]"
        fi
        
        # Check that rpc_urls was added
        if grep -q "rpc_urls:" "$TEST_WALRUS_CONFIG"; then
            echo "✓ rpc_urls section was added"
        else
            fail "rpc_urls section was not added"
        fi
        
        # Show final config for debugging
        echo "  Final configuration:"
        cat "$TEST_WALRUS_CONFIG" | sed 's/^/    /'
        
    else
        fail "rpc_urls repair function failed"
    fi
    
    echo ""
    
    # Test 2: Config ending with default_context (no trailing newline scenario)
    echo "Test 2: Config ending exactly with default_context..."
    
    # Create config that ends with default_context
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
default_context: $WORKDIR
EOF
    
    # Remove the final newline to test edge case
    printf '%s' "$(cat "$TEST_WALRUS_CONFIG")" > "$TEST_WALRUS_CONFIG"
    
    DEFAULT_CONTEXT_BEFORE=$(grep "default_context:" "$TEST_WALRUS_CONFIG")
    
    # Call repair function
    if repair_walrus_rpc_urls_as_needed "$TEST_WALRUS_CONFIG" "$WORKDIR"; then
        DEFAULT_CONTEXT_AFTER=$(grep "default_context:" "$TEST_WALRUS_CONFIG")
        
        if [ "$DEFAULT_CONTEXT_BEFORE" = "$DEFAULT_CONTEXT_AFTER" ]; then
            echo "✓ default_context preserved even when at end of file"
        else
            fail "default_context lost when at end of file"
            echo "  Before: [$DEFAULT_CONTEXT_BEFORE]"
            echo "  After: [$DEFAULT_CONTEXT_AFTER]"
        fi
    else
        fail "Repair function failed on edge case config"
    fi
    
    echo ""
    
    # Test 3: Config with existing rpc_urls - ensure replacement doesn't break other lines
    echo "Test 3: Replacing existing rpc_urls without breaking other lines..."
    
    cat > "$TEST_WALRUS_CONFIG" << EOF
contexts:
  $WORKDIR:
    system_object: 0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af
    staking_object: 0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3
    exchange_objects:
      - 0xf4d164ea2def5fe07dc573992a029e010dba09b1a8dcbc44c5c2e79567f39073
      - 0x19825121c52080bb1073662231cfea5c0e4d905fd13e95f21e9a018f2ef41862
    wallet_config:
      path: config/client.yaml
      active_env: $WORKDIR
    rpc_urls:
      - http://old.example.com:9999
      - http://another.old.url:8888
default_context: $WORKDIR
EOF

    EXCHANGE_OBJECTS_COUNT_BEFORE=$(grep -c "0x" "$TEST_WALRUS_CONFIG")
    DEFAULT_CONTEXT_BEFORE=$(grep "default_context:" "$TEST_WALRUS_CONFIG")
    
    # Call repair function
    if repair_walrus_rpc_urls_as_needed "$TEST_WALRUS_CONFIG" "$WORKDIR"; then
        EXCHANGE_OBJECTS_COUNT_AFTER=$(grep -c "0x" "$TEST_WALRUS_CONFIG")
        DEFAULT_CONTEXT_AFTER=$(grep "default_context:" "$TEST_WALRUS_CONFIG")
        
        # Check exchange objects not affected by rpc_urls replacement
        if [ "$EXCHANGE_OBJECTS_COUNT_BEFORE" = "$EXCHANGE_OBJECTS_COUNT_AFTER" ]; then
            echo "✓ exchange_objects preserved during rpc_urls replacement"
        else
            fail "exchange_objects affected by rpc_urls replacement"
        fi
        
        # Check default_context preserved
        if [ "$DEFAULT_CONTEXT_BEFORE" = "$DEFAULT_CONTEXT_AFTER" ]; then
            echo "✓ default_context preserved during rpc_urls replacement"
        else
            fail "default_context lost during rpc_urls replacement"
        fi
        
        # Verify old URLs are gone and new ones present
        if ! grep -q "old.example.com" "$TEST_WALRUS_CONFIG"; then
            echo "✓ Old rpc_urls removed"
        else
            fail "Old rpc_urls not removed"
        fi
        
        # Check for the expected proxy URL (dynamic based on workdir config)
        local expected_proxy_url="http://${CFG_proxy_host_ip}:${CFG_proxy_port_number}"
        if grep -q "$expected_proxy_url" "$TEST_WALRUS_CONFIG"; then
            echo "✓ New smart rpc_urls added"
        else
            fail "New smart rpc_urls not added (expected: $expected_proxy_url)"
        fi
        
    else
        fail "Repair function failed during rpc_urls replacement"
    fi
    
    echo "✓ All configuration integrity tests passed"
}

# Main test execution
tests() {
    test_config_integrity
}

# Standard test framework execution
main() {
    tests
}

[ "${BASH_SOURCE[0]}" == "${0}" ] && main "$@"