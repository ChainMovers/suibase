#!/bin/bash

# Common utilities for suibase tests

# Text formatting functions
print_header() {
    echo ""
    echo "================================"
    echo "$1"
    echo "================================"
}

print_step() {
    echo "--- $1"
}

print_success() {
    echo "✓ $1"
}

print_error() {
    echo "✗ $1" >&2
}

print_warning() {
    echo "⚠ $1" >&2
}

# Validation functions
validate_json() {
    local input="$1"
    echo "$input" | jq . >/dev/null 2>&1
}

# Check if response contains binary data (should NEVER happen from suibase-daemon)
is_binary_response() {
    local response="$1"
    
    # Check for gzip magic numbers
    if echo "$response" | grep -q $'\x1f\x8b\x08'; then
        return 0  # gzip header found - BAD
    fi
    
    # Check for control characters (except normal whitespace)
    if echo "$response" | grep -q $'[\x00-\x08\x0E-\x1F\x7F]'; then
        return 0  # control characters found - BAD
    fi
    
    # Check for garbled JSON-like patterns that indicate compression
    if echo "$response" | grep -q '�.*{.*}' || echo "$response" | grep -q '{.*�.*}'; then
        return 0  # compressed JSON pattern - BAD
    fi
    
    return 1  # appears to be normal text - GOOD
}