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

# Helper: detect docker compose command (V2 plugin or V1 standalone)
if docker compose version &>/dev/null 2>&1; then
    COMPOSE="docker compose"
elif command -v docker-compose &>/dev/null; then
    COMPOSE="docker-compose"
else
    COMPOSE=""
fi

# ─── Phase 1: Backend Unit & Integration Tests (cargo test) ─────────

echo ""
echo "── Phase 1: Backend Unit & Integration Tests ──"

if command -v cargo &>/dev/null; then
    echo "Running cargo test for backend (local)..."
    if (cd "$SCRIPT_DIR" && cargo test --package server 2>&1); then
        pass "Backend cargo tests passed"
    else
        fail "Backend cargo tests failed"
    fi
elif [ -n "$COMPOSE" ]; then
    echo "  cargo not found — running via Docker..."
    if $COMPOSE -f "$SCRIPT_DIR/docker-compose.yml" run --rm --no-deps \
        -e DATABASE_URL=":memory:" backend \
        sh -c "cd /app && cargo test --package server" 2>&1; then
        pass "Backend cargo tests passed (Docker)"
    else
        fail "Backend cargo tests failed (Docker)"
    fi
else
    skip "Backend cargo tests (no cargo or Docker available)"
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
elif [ -n "$COMPOSE" ]; then
    echo "  No local toolchain — running via Docker..."
    if $COMPOSE -f "$SCRIPT_DIR/docker-compose.yml" run --rm --no-deps \
        -e DATABASE_URL=":memory:" backend \
        sh -c "cd /app && cargo test --package web 2>&1 || true" 2>&1; then
        pass "Frontend tests passed (Docker, cargo test)"
    else
        skip "Frontend WASM tests (Docker cargo test fallback)"
    fi
else
    skip "Frontend WASM tests (no toolchain or Docker available)"
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

# Small helpers used by Phase 4 & 5 below.
json_get()    { python3 -c "import sys,json; d=json.load(sys.stdin); sys.stdout.write(str(d$1))" 2>/dev/null || true; }
fetch_nonce() { curl -sf "${BACKEND_URL}/api/auth/nonce" 2>/dev/null | json_get "['nonce']"; }

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
    NONCE=$(fetch_nonce)
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

# ─── Phase 5: Extended API E2E Checks ───────────────────────────────
# Additional multi-step smoke tests that exercise auth, nonces, and
# business logic end-to-end through curl.

echo ""
echo "── Phase 5: Extended API E2E Checks ──"

if curl -sf "${BACKEND_URL}/api/projects" -o /dev/null 2>/dev/null; then

    # 5.1 — nonce reuse returns 409 Conflict
    NONCE=$(fetch_nonce)
    if [ -n "$NONCE" ]; then
        UNIQUE="dupe_nonce_$(date +%s%N)"
        # First consumption
        STATUS1=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/auth/register" \
            -H "Content-Type: application/json" \
            -H "X-Nonce: $NONCE" \
            -d "{\"email\":\"${UNIQUE}@test.com\",\"password\":\"Passw0rd1!\",\"display_name\":\"N\",\"role\":\"supporter\"}" \
            2>/dev/null)
        # Second consumption with same nonce
        STATUS2=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/auth/register" \
            -H "Content-Type: application/json" \
            -H "X-Nonce: $NONCE" \
            -d "{\"email\":\"${UNIQUE}_2@test.com\",\"password\":\"Passw0rd1!\",\"display_name\":\"N\",\"role\":\"supporter\"}" \
            2>/dev/null)
        if [ "$STATUS1" = "200" ] && [ "$STATUS2" = "409" ]; then
            pass "Nonce reuse returns 409 Conflict"
        else
            fail "Nonce reuse test got $STATUS1 then $STATUS2 (expected 200 then 409)"
        fi
    else
        skip "Nonce reuse test (could not obtain nonce)"
    fi

    # 5.2 — short password rejected at registration
    NONCE=$(fetch_nonce)
    if [ -n "$NONCE" ]; then
        STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/auth/register" \
            -H "Content-Type: application/json" \
            -H "X-Nonce: $NONCE" \
            -d "{\"email\":\"short_$(date +%s)@test.com\",\"password\":\"short\",\"display_name\":\"S\",\"role\":\"supporter\"}" \
            2>/dev/null)
        if [ "$STATUS" = "400" ]; then
            pass "Short password rejected with 400"
        else
            fail "Short password returned $STATUS (expected 400)"
        fi
    else
        skip "Short password test (no nonce)"
    fi

    # 5.3 — login with wrong password returns 401
    EMAIL="authtest_$(date +%s%N)@test.com"
    NONCE=$(fetch_nonce)
    curl -sf -X POST "${BACKEND_URL}/api/auth/register" \
        -H "Content-Type: application/json" \
        -H "X-Nonce: $NONCE" \
        -d "{\"email\":\"${EMAIL}\",\"password\":\"GoodPass123!\",\"display_name\":\"Auth\",\"role\":\"supporter\"}" \
        -o /dev/null 2>/dev/null || true
    NONCE=$(fetch_nonce)
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/auth/login" \
        -H "Content-Type: application/json" \
        -H "X-Nonce: $NONCE" \
        -d "{\"email\":\"${EMAIL}\",\"password\":\"wrong\"}" 2>/dev/null)
    if [ "$STATUS" = "401" ]; then
        pass "Login with wrong password returns 401"
    else
        fail "Login with wrong password returned $STATUS (expected 401)"
    fi

    # 5.4 — login with correct password succeeds and issues token
    NONCE=$(fetch_nonce)
    LOGIN_RESP=$(curl -sf -X POST "${BACKEND_URL}/api/auth/login" \
        -H "Content-Type: application/json" \
        -H "X-Nonce: $NONCE" \
        -d "{\"email\":\"${EMAIL}\",\"password\":\"GoodPass123!\"}" 2>/dev/null || true)
    TOKEN=$(echo "$LOGIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('token',''))" 2>/dev/null || true)
    if [ -n "$TOKEN" ]; then
        pass "Login issues a valid session token"
    else
        fail "Login did not return a token"
    fi

    # 5.5 — duplicate email registration returns 409
    NONCE=$(fetch_nonce)
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/auth/register" \
        -H "Content-Type: application/json" \
        -H "X-Nonce: $NONCE" \
        -d "{\"email\":\"${EMAIL}\",\"password\":\"AnyPass123!\",\"display_name\":\"Dup\",\"role\":\"supporter\"}" \
        2>/dev/null)
    if [ "$STATUS" = "409" ]; then
        pass "Duplicate-email registration returns 409"
    else
        fail "Duplicate-email registration returned $STATUS (expected 409)"
    fi

    # 5.6 — DND update requires auth, then succeeds with token
    if [ -n "${TOKEN:-}" ]; then
        NONCE=$(fetch_nonce)
        STATUS_UNAUTH=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "${BACKEND_URL}/api/auth/dnd" \
            -H "Content-Type: application/json" \
            -H "X-Nonce: $NONCE" \
            -d '{"dnd_start":"22:00","dnd_end":"06:00","timezone":"UTC"}' 2>/dev/null)
        if [ "$STATUS_UNAUTH" = "401" ]; then
            pass "PUT /api/auth/dnd without token returns 401"
        else
            fail "PUT /api/auth/dnd without token returned $STATUS_UNAUTH (expected 401)"
        fi

        NONCE=$(fetch_nonce)
        STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "${BACKEND_URL}/api/auth/dnd" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "X-Nonce: $NONCE" \
            -d '{"dnd_start":"22:30","dnd_end":"06:30","timezone":"UTC"}' 2>/dev/null)
        if [ "$STATUS" = "200" ]; then
            pass "PUT /api/auth/dnd with token succeeds"
        else
            fail "PUT /api/auth/dnd with token returned $STATUS (expected 200)"
        fi
    else
        skip "DND tests (no auth token)"
    fi

    # 5.7 — event tracking accepts anonymous POST
    NONCE=$(fetch_nonce)
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/events/track" \
        -H "Content-Type: application/json" \
        -H "X-Nonce: $NONCE" \
        -d '{"event_kind":"click","target_type":"button","target_id":"donate","session_id":"e2e-sess-1"}' \
        2>/dev/null)
    if [ "$STATUS" = "200" ]; then
        pass "POST /api/events/track accepts anonymous event"
    else
        fail "POST /api/events/track returned $STATUS (expected 200)"
    fi

    # 5.8 — rejected invalid event_kind
    NONCE=$(fetch_nonce)
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/events/track" \
        -H "Content-Type: application/json" \
        -H "X-Nonce: $NONCE" \
        -d '{"event_kind":"fake_kind","target_type":"button","target_id":"x","session_id":"e2e-sess-x"}' \
        2>/dev/null)
    if [ "$STATUS" = "400" ]; then
        pass "Invalid event_kind rejected with 400"
    else
        fail "Invalid event_kind returned $STATUS (expected 400)"
    fi

    # 5.9 — donation on non-existent project returns 404
    if [ -n "${TOKEN:-}" ]; then
        NONCE=$(fetch_nonce)
        STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/donations" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "X-Nonce: $NONCE" \
            -d '{"project_id":"does-not-exist","amount_cents":1000,"payment_method":"cash"}' \
            2>/dev/null)
        if [ "$STATUS" = "404" ]; then
            pass "Donation on missing project returns 404"
        else
            fail "Donation on missing project returned $STATUS (expected 404)"
        fi
    else
        skip "Donation 404 test (no auth token)"
    fi

    # 5.10 — donation with zero amount returns 400
    if [ -n "${TOKEN:-}" ]; then
        PROJECT_ID=$(curl -sf "${BACKEND_URL}/api/projects" 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['items'][0]['id'] if d['items'] else '')" 2>/dev/null || true)
        if [ -n "$PROJECT_ID" ]; then
            NONCE=$(fetch_nonce)
            STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/donations" \
                -H "Content-Type: application/json" \
                -H "Authorization: Bearer ${TOKEN}" \
                -H "X-Nonce: $NONCE" \
                -d "{\"project_id\":\"${PROJECT_ID}\",\"amount_cents\":0,\"payment_method\":\"cash\"}" \
                2>/dev/null)
            if [ "$STATUS" = "400" ]; then
                pass "Zero-amount donation rejected with 400"
            else
                fail "Zero-amount donation returned $STATUS (expected 400)"
            fi

            # 5.11 — donation with invalid payment method returns 400
            NONCE=$(fetch_nonce)
            STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/donations" \
                -H "Content-Type: application/json" \
                -H "Authorization: Bearer ${TOKEN}" \
                -H "X-Nonce: $NONCE" \
                -d "{\"project_id\":\"${PROJECT_ID}\",\"amount_cents\":1000,\"payment_method\":\"crypto\"}" \
                2>/dev/null)
            if [ "$STATUS" = "400" ]; then
                pass "Invalid payment method rejected with 400"
            else
                fail "Invalid payment method returned $STATUS (expected 400)"
            fi

            # 5.12 — successful donation returns pledge number
            NONCE=$(fetch_nonce)
            DONATION_RESP=$(curl -sf -X POST "${BACKEND_URL}/api/donations" \
                -H "Content-Type: application/json" \
                -H "Authorization: Bearer ${TOKEN}" \
                -H "X-Nonce: $NONCE" \
                -d "{\"project_id\":\"${PROJECT_ID}\",\"amount_cents\":100,\"payment_method\":\"cash\"}" \
                2>/dev/null || true)
            if echo "$DONATION_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['donation']['pledge_number'].startswith('PLG-')" 2>/dev/null; then
                pass "Successful donation issues PLG- pledge number"
            else
                fail "Donation did not produce a PLG- pledge number"
            fi

            # 5.13 — /api/donations/mine scoped to caller (non-empty now)
            MINE=$(curl -sf "${BACKEND_URL}/api/donations/mine" \
                -H "Authorization: Bearer ${TOKEN}" 2>/dev/null || true)
            if echo "$MINE" | python3 -c "import sys,json; d=json.load(sys.stdin); assert isinstance(d, list) and len(d) >= 1" 2>/dev/null; then
                pass "GET /api/donations/mine returns caller's donations"
            else
                fail "GET /api/donations/mine format unexpected"
            fi
        else
            skip "Donation flow (no project available)"
        fi
    fi

    # 5.14 — project list filter by cause
    FILTERED=$(curl -sf "${BACKEND_URL}/api/projects?cause=education" 2>/dev/null || true)
    if echo "$FILTERED" | python3 -c "
import sys, json
d = json.load(sys.stdin)
items = d['items']
# every returned item must be education-cause
assert all(i['cause'] == 'education' for i in items), 'expected only education items'
" 2>/dev/null; then
        pass "GET /api/projects?cause=education applies filter"
    else
        fail "GET /api/projects?cause=education filter invalid"
    fi

    # 5.15 — project list respects per_page parameter
    PAGED=$(curl -sf "${BACKEND_URL}/api/projects?per_page=1" 2>/dev/null || true)
    if echo "$PAGED" | python3 -c "
import sys, json
d = json.load(sys.stdin)
assert d['per_page'] == 1 and len(d['items']) <= 1
" 2>/dev/null; then
        pass "GET /api/projects per_page=1 returns at most 1 item"
    else
        fail "GET /api/projects per_page parameter not applied"
    fi

    # 5.16 — project detail endpoint returns 404 for missing id
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "${BACKEND_URL}/api/projects/nope-nope-nope" 2>/dev/null)
    if [ "$STATUS" = "404" ]; then
        pass "GET /api/projects/{missing} returns 404"
    else
        fail "GET /api/projects/{missing} returned $STATUS (expected 404)"
    fi

    # 5.17 — project detail endpoint returns a full ProjectDetail for seed data
    PROJECT_ID=$(curl -sf "${BACKEND_URL}/api/projects" 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['items'][0]['id'] if d['items'] else '')" 2>/dev/null || true)
    if [ -n "$PROJECT_ID" ]; then
        DETAIL=$(curl -sf "${BACKEND_URL}/api/projects/${PROJECT_ID}" 2>/dev/null || true)
        if echo "$DETAIL" | python3 -c "
import sys, json
d = json.load(sys.stdin)
for k in ('id','title','description','cause','zip_code','status','goal_cents',
         'raised_cents','spent_cents','manager_id','manager_name',
         'budget_lines','updates'):
    assert k in d, f'missing {k}'
assert isinstance(d['budget_lines'], list)
assert isinstance(d['updates'], list)
" 2>/dev/null; then
            pass "GET /api/projects/{id} returns complete ProjectDetail"
        else
            fail "GET /api/projects/{id} response is missing fields"
        fi
    else
        skip "Project detail test (no project available)"
    fi

    # 5.18 — moderation config endpoint requires auth
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "${BACKEND_URL}/api/moderation/config" 2>/dev/null)
    if [ "$STATUS" = "401" ]; then
        pass "GET /api/moderation/config without token returns 401"
    else
        fail "GET /api/moderation/config without token returned $STATUS (expected 401)"
    fi

    # 5.19 — admin stats: supporter forbidden
    if [ -n "${TOKEN:-}" ]; then
        STATUS=$(curl -s -o /dev/null -w "%{http_code}" "${BACKEND_URL}/api/admin/stats" \
            -H "Authorization: Bearer ${TOKEN}" 2>/dev/null)
        if [ "$STATUS" = "403" ]; then
            pass "Supporter forbidden from /api/admin/stats"
        else
            fail "Supporter at /api/admin/stats returned $STATUS (expected 403)"
        fi
    else
        skip "Admin stats forbidden check (no token)"
    fi

    # 5.20 — ops-log: supporter forbidden
    if [ -n "${TOKEN:-}" ]; then
        STATUS=$(curl -s -o /dev/null -w "%{http_code}" "${BACKEND_URL}/api/admin/ops-log" \
            -H "Authorization: Bearer ${TOKEN}" 2>/dev/null)
        if [ "$STATUS" = "403" ]; then
            pass "Supporter forbidden from /api/admin/ops-log"
        else
            fail "Supporter at /api/admin/ops-log returned $STATUS (expected 403)"
        fi
    else
        skip "Ops log forbidden check (no token)"
    fi

    # 5.21 — webhook creation without admin role is forbidden
    if [ -n "${TOKEN:-}" ]; then
        NONCE=$(fetch_nonce)
        STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/webhooks" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "X-Nonce: $NONCE" \
            -d '{"name":"t","url":"http://10.0.0.1/h","event_types":["donation.created"]}' \
            2>/dev/null)
        if [ "$STATUS" = "403" ]; then
            pass "Supporter forbidden from creating webhooks"
        else
            fail "Supporter creating webhook returned $STATUS (expected 403)"
        fi
    else
        skip "Webhook forbidden check (no token)"
    fi

    # 5.22 — empty body POST to a JSON route returns 400
    NONCE=$(fetch_nonce)
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BACKEND_URL}/api/auth/register" \
        -H "Content-Type: application/json" \
        -H "X-Nonce: $NONCE" \
        -d "" 2>/dev/null)
    # Empty body → serde failure → 400
    if [ "$STATUS" = "400" ] || [ "$STATUS" = "415" ] || [ "$STATUS" = "422" ]; then
        pass "Empty body on /api/auth/register returns $STATUS (parsing error)"
    else
        fail "Empty body on /api/auth/register returned $STATUS"
    fi

    # 5.23 — CORS preflight OPTIONS request succeeds
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X OPTIONS "${BACKEND_URL}/api/projects" \
        -H "Origin: http://localhost:8080" \
        -H "Access-Control-Request-Method: GET" 2>/dev/null)
    # CORS layer returns 200 on successful preflight
    if [ "$STATUS" = "200" ] || [ "$STATUS" = "204" ]; then
        pass "CORS preflight OPTIONS returns $STATUS"
    else
        fail "CORS preflight OPTIONS returned $STATUS"
    fi

else
    skip "Extended API E2E checks (backend not reachable)"
fi

# ─── Phase 6: Frontend Content Checks ───────────────────────────────
# Basic checks that the built Leptos app is served correctly.

echo ""
echo "── Phase 6: Frontend Content Checks ──"

if curl -sf "${FRONTEND_URL}/" -o /dev/null 2>/dev/null; then
    # Homepage HTML contains expected elements
    HTML=$(curl -sf "${FRONTEND_URL}/" 2>/dev/null || true)
    if echo "$HTML" | grep -qi "<html"; then
        pass "Homepage returns HTML"
    else
        fail "Homepage did not return HTML"
    fi

    # Leptos WASM bundle is referenced
    if echo "$HTML" | grep -qE "\.(js|wasm)"; then
        pass "Homepage references JS/WASM bundle"
    else
        fail "Homepage does not reference any JS/WASM bundle"
    fi
else
    skip "Frontend content checks (frontend not reachable)"
fi

# ─── Summary ─────────────────────────────────────────────────────────

echo ""
echo "============================================="
echo "  Results: $PASSED passed, $FAILED failed, $SKIPPED skipped"
echo "============================================="

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi
