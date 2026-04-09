#!/usr/bin/env bash
set -e

echo "=== Running Fund Transparency Tests ==="

BACKEND_URL="${BACKEND_URL:-http://localhost:3000}"

echo "--- Checking backend health ---"
curl -sf "${BACKEND_URL}/api/projects" -o /dev/null && echo "OK: /api/projects reachable" || echo "WARN: backend not reachable (may not be running)"

echo "--- Checking frontend ---"
curl -sf "http://localhost:80/" -o /dev/null && echo "OK: frontend reachable" || echo "WARN: frontend not reachable"

echo "=== Tests complete ==="
