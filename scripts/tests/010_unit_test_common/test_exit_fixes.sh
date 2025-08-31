#!/bin/bash

# Test script to verify that our exit 1 fixes work correctly
# This script intentionally tests edge cases that could cause exit 1

# Ignore SIGPIPE on macOS to prevent test failures
if [[ "$(uname)" == "Darwin" ]]; then
    trap '' SIGPIPE
    echo "SIGPIPE trap installed"
fi
set -e  # Enable exit on error to test our fixes

echo "=== Testing Exit 1 Fixes ==="

# Test 1: grep with || pattern when file doesn't exist
echo "--- Test 1: grep with fallback when file missing ---"
NONEXISTENT_FILE="/tmp/nonexistent_test_file_$$"
result=$(grep "some_pattern" "$NONEXISTENT_FILE" 2>/dev/null || echo "not_found")
if [ "$result" = "not_found" ]; then
    echo "✓ grep with fallback works correctly when file missing"
else
    echo "✗ grep with fallback failed: $result"
    exit 1
fi

# Test 2: grep with || pattern when pattern not found
echo "--- Test 2: grep with fallback when pattern missing ---"
echo "some content here" > "/tmp/test_file_$$"
result=$(grep "missing_pattern" "/tmp/test_file_$$" 2>/dev/null || echo "not_found")
if [ "$result" = "not_found" ]; then
    echo "✓ grep with fallback works correctly when pattern missing"
else
    echo "✗ grep with fallback failed: $result"
    exit 1
fi
rm -f "/tmp/test_file_$$"

# Test 3: rm -f instead of rm (should not fail on missing files)
echo "--- Test 3: rm -f for missing files ---"
rm -f "/tmp/nonexistent_backup_file_$$"
echo "✓ rm -f works correctly for missing files"

# Test 4: Command substitution with fallback
echo "--- Test 4: Command substitution with fallback ---"
result=$(cat "/tmp/nonexistent_$$" 2>/dev/null || echo "default_value")
if [ "$result" = "default_value" ]; then
    echo "✓ Command substitution with fallback works correctly"
else
    echo "✗ Command substitution with fallback failed: $result"
    exit 1
fi

echo
echo "=== All Exit 1 Fix Tests Passed! ==="