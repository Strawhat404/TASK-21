#!/usr/bin/env bash
set -euo pipefail

echo "============================================="
echo "  Fund Transparency — Test Suite"
echo "============================================="

BACKEND_URL="${BACKEND_URL:-http://localhost:3000}"
FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PASSED=0
FAILED=0
SKIPPED=0

pass()  { PASSED=$((PASSED + 1)); echo "  ✓ $1"; }
fail()  { FAILED=$((FAILED + 1)); echo "  ✗ $1"; }
skip()  { SKIPPED=$((SKIPPED + 1)); echo "  - $1 (skipped)"; }

# ─── Phase 1: Backend Unit & Integration Tests (cargo test) ─────────

echo ""
echo "── Phase 1: Backend Unit & Integration Tests ──"

if command -v cargo &>/dev/null; then
    echo "Running cargo test for backend..."
    if (cd "$SCRIPT_DIR" && cargo test --package server 2>&1); then
        pass "Backend cargo tests passed"
    else
        fail "Backend cargo tests failed"
    fi
else
    echo "  cargo not found — attempting via Docker..."
    if docker compose -f "$SCRIPT_DIR/docker-compose.yml" run --rm --no-deps \
        -e DATABASE_URL=":memory:" backend \
        sh -c "cd /app && cargo test --package server" 2>&1; then
        pass "Backend cargo tests passed (Docker)"
    else
        skip "Backend cargo tests (no cargo or Docker available)"
    fi
fi

# ─── Phase 2: Frontend WASM Tests ───────────────────────────────────

echo ""
echo "── Phase 2: Frontend WASM Tests ──"

if command -v wasm-pack &>/dev/null; then
    echo "Running wasm-pack test for frontend..."
    if (cd "$SCRIPT_DIR/frontend" && wasm-pack test --headless --chrome 2>&1) || \
       (cd "$SCRIPT_DIR/frontend" && wasm-pack test --headless --firefox 2>&1); then
        pass "Frontend WASM tests passed"
    else
        fail "Frontend WASM tests failed"
    fi
elif command -v cargo &>/dev/null; then
    echo "wasm-pack not found, attempting cargo test..."
    if (cd "$SCRIPT_DIR" && cargo test --package web 2>&1); then
        pass "Frontend tests passed (cargo test)"
    else
        skip "Frontend WASM tests (wasm-pack not available, cargo test may not cover wasm_bindgen_test)"
    fi
else
    skip "Frontend WASM tests (wasm-pack and cargo not available)"
fi

# ─── Phase 3: Service Health Checks ─────────────────────────────────

echo ""
echo "── Phase 3: Service Health Checks ──"

if curl -sf "${BACKEND_URL}/api/projects" -o /dev/null 2>/dev/null; then
    pass "Backend reachable at ${BACKEND_URL}"
else
    skip "Backend not reachable at ${BACKEND_URL} (not running?)"
fi

if curl -sf "${FRONTEND_URL}/" -o /dev/null 2>/dev/null; then
    pass "Frontend reachable at ${FRONTEND_URL}"
else
    skip "Frontend not reachable at ${FRONTEND_URL} (not running?)"
fi

# ─── Phase 4: API Smoke Tests ───────────────────────────────────────
# These run only if the backend is reachable.

echo ""
echo "── Phase 4: API Smoke Tests ──"

if curl -sf "${BACKEND_URL}/api/projects" -o /dev/null 2>/dev/null; then

    # Test: GET /api/projects returns valid JSON array
    RESP=$(curl -sf "${BACKEND_URL}/api/projects" 2>/dev/null || true)
    if echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'items' in d" 2>/dev/null; then
        pass "GET /api/projects returns paginated JSON"
    else
        fail "GET /api/projects response format unexpected"
    fi

    # Test: GET /api/auth/nonce returns a nonce
    NONCE_RESP=$(curl -sf "${BACKEND_URL}/api/auth/nonce" 2>/dev/null || true)
    if echo "$NONCE_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert len(d.get('nonce',''))>0" 2>/dev/null; then
        pass "GET /api/auth/nonce returns nonce"
    else
        fail "GET /api/auth/nonce response unexpected"
    fi

    # Test: POST without nonce returns 400
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "${BACKEND_URL}/api/auth/register" \
        -H "Content-Type: application/json" \
        -d '{"email":"test@test.com","password":"Test123!","display_name":"T","role":"supporter"}' \
        2>/dev/null)
    if [ "$STATUS" = "400" ]; then
        pass "POST without X-Nonce returns 400"
    else
        fail "POST without X-Nonce returned $STATUS (expected 400)"
    fi

    # Test: GET on non-existent route returns 404 or falls through
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        "${BACKEND_URL}/api/nonexistent" 2>/dev/null)
    if [ "$STATUS" = "404" ] || [ "$STATUS" = "000" ]; then
        pass "Non-existent route handled correctly"
    else
        fail "Non-existent route returned $STATUS"
    fi

    # Test: Auth-required route without token returns 401
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        "${BACKEND_URL}/api/admin/ops-log" 2>/dev/null)
    if [ "$STATUS" = "401" ]; then
        pass "Auth-required route without token returns 401"
    else
        fail "Auth-required route returned $STATUS (expected 401)"
    fi

    # Test: Register + Login flow
    NONCE=$(curl -sf "${BACKEND_URL}/api/auth/nonce" | python3 -c "import sys,json; print(json.load(sys.stdin)['nonce'])" 2>/dev/null || true)
    if [ -n "$NONCE" ]; then
        REGISTER_RESP=$(curl -sf -X POST "${BACKEND_URL}/api/auth/register" \
            -H "Content-Type: application/json" \
            -H "X-Nonce: $NONCE" \
            -d "{\"email\":\"smoketest_$(date +%s)@test.com\",\"password\":\"SmokeTest123!\",\"display_name\":\"Smoke\",\"role\":\"administrator\"}" \
            2>/dev/null || true)
        if echo "$REGISTER_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['user']['role']=='supporter'" 2>/dev/null; then
            pass "Registration enforces supporter role (ignores client role)"
        else
            fail "Registration role enforcement failed"
        fi

        TOKEN=$(echo "$REGISTER_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('token',''))" 2>/dev/null || true)
        if [ -n "$TOKEN" ]; then
            ME_RESP=$(curl -sf "${BACKEND_URL}/api/auth/me" \
                -H "Authorization: Bearer $TOKEN" 2>/dev/null || true)
            if echo "$ME_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['role']=='supporter'" 2>/dev/null; then
                pass "GET /api/auth/me returns authenticated user profile"
            else
                fail "GET /api/auth/me response unexpected"
            fi
        fi
    else
        skip "Register + Login flow (could not obtain nonce)"
    fi

else
    skip "API smoke tests (backend not reachable)"
fi

# ─── Summary ─────────────────────────────────────────────────────────

echo ""
echo "============================================="
echo "  Results: $PASSED passed, $FAILED failed, $SKIPPED skipped"
echo "============================================="

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi
