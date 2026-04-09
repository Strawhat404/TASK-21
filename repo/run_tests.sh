#!/usr/bin/env bash
set -e

echo "=== Running Fund Transparency Tests ==="

cd "$(dirname "$0")"

echo "--- Unit & Integration Tests ---"
cargo test --workspace 2>&1

echo "=== All tests passed ==="
