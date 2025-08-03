#!/bin/bash

# Script to run proxy performance benchmarks on demand
# This runs outside of regular CI/CD testing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUIBASE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
RUST_DIR="$SUIBASE_DIR/rust/suibase"

echo "Running Suibase proxy performance benchmarks..."
echo "============================================="
echo

cd "$RUST_DIR"

# Run benchmarks with the benchmarks feature flag
echo "Running proxy performance benchmark tests..."
cargo test --features benchmarks --test proxy_performance_benchmark_test -- --nocapture

echo
echo "Benchmarks completed!"