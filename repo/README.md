# Fund Transparency Portal

**Project type: fullstack**

A community-focused fundraising platform built for spending transparency and auditability. Donors can contribute to community projects and verify that every cent is spent as intended through real-time expense reporting, receipt disclosure, and fulfillment verification.

## Architecture

```
Browser (WASM)          REST API            SQLite
┌──────────────┐       ┌───────────────┐    ┌────────────┐
│  Leptos CSR  │──────>│  Axum Server  │───>│  Database   │
│  (Trunk/WASM)│<──────│  port 3000    │<───│  (WAL mode) │
│  port 8080   │       │               │    └────────────┘
│  via nginx   │       │  Middleware:   │
└──────────────┘       │  - Rate limit  │
                       │  - Nonce replay│
                       │  - Auth (HMAC) │
                       └───────────────┘
```

**Runtime data flow**: The Leptos frontend compiles to WebAssembly and runs in the browser. It communicates with the Axum backend via REST JSON APIs. Every mutating request (POST/PUT/DELETE) requires a server-issued nonce (`X-Nonce` header) for replay protection. Authenticated requests carry an HMAC-signed session token in the `Authorization: Bearer` header. Sensitive data (receipt files, reviewer notes) is encrypted at rest using AES-256-GCM with a host-managed key. SQLite persists all state in WAL mode with foreign keys enforced.

**Services**: Two Docker containers — `backend` (Rust binary, port 3000) and `frontend` (nginx serving WASM, port 8080). No external databases or cloud services required; fully offline-capable.

## Tech Stack

- **Backend**: Rust, Axum 0.7, SQLite (rusqlite), Tokio
- **Frontend**: Rust, Leptos 0.6, Trunk (WASM), nginx
- **Auth**: Argon2 password hashing, HMAC-SHA256 signed session tokens, AES-256-GCM encryption at rest
- **Shared**: `common` crate with DTOs and enums shared between frontend and backend

## Roles and Security Model

The system serves four roles with ascending privilege:

| Role | Capabilities |
|------|-------------|
| **Supporter** | Browse projects, donate, favorite, comment, submit feedback tickets, manage notifications and DND |
| **Project Manager** | Everything above + create projects, post spending updates, record expenses, upload receipts, manage fulfillment verification, moderate comments on own projects |
| **Finance Reviewer** | Everything above + review/verify receipts, approve/reject expense disclosures, approve/reject refund requests (two-step confirmation), view pending expenses, export CSV |
| **Administrator** | Everything above + assign roles, manage moderation config (sensitive words, enable/disable comments, pre-moderation), unpublish projects (two-step confirmation), view ops log, manage webhooks, view suspicious events, bootstrap first admin |

**Security controls**:
- Sensitive actions (refund approval, project unpublish, comment deletion) require password re-entry **and** a server-issued confirmation token (two-step protocol).
- Rate limiting: 60 requests/minute per session (authenticated) or per IP (anonymous).
- Nonce replay protection: every mutating request requires a fresh 5-minute nonce.
- Receipt uploads: PDF/JPG/PNG only, max 10 MB, SHA-256 fingerprint dedup, encrypted at rest.
- Fulfillment verification: OTP/QR codes with 10-minute expiry, time-consistency rules (start within 2h of arrival, end after start), tamper-evident SHA-256 service record.
- Webhooks: restricted to local/private network URLs only (http scheme, private IP ranges, localhost).

## Getting Started

### Prerequisites

- Docker and Docker Compose

### Startup

```bash
cd repo
docker-compose up
```

Wait for the health check to pass (the backend container reports healthy after ~15 seconds).

- **Frontend**: http://localhost:8080
- **Backend API**: http://localhost:3000

### Verification

After startup, verify the system is working:

```bash
# 1. Backend health: should return a JSON object with "items" array
curl -s http://localhost:3000/api/projects | head -c 200

# 2. Frontend: should return HTML with WASM bundle reference
curl -s http://localhost:8080/ | grep -o '<html[^>]*>'

# 3. Auth flow: register a new user
NONCE=$(curl -s http://localhost:3000/api/auth/nonce | python3 -c "import sys,json; print(json.load(sys.stdin)['nonce'])")
curl -s -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -H "X-Nonce: $NONCE" \
  -d '{"email":"verify@test.com","password":"Verify123!","display_name":"Verifier","role":"supporter"}'
# Should return JSON with "token" and "user" fields

# 4. UI flow: open http://localhost:8080 in a browser, click "Browse Projects",
#    you should see two seed projects (STEM Lab, Green Park). Click "Register"
#    to create an account, then try donating to a project.
```

### Demo Credentials

On first startup, the database is seeded with a Project Manager and two sample projects. To access the full system, bootstrap an admin and create demo accounts:

```bash
# Step 1: Register demo users (supporter is auto-assigned; PM and Finance are self-selectable)
NONCE=$(curl -s http://localhost:3000/api/auth/nonce | python3 -c "import sys,json; print(json.load(sys.stdin)['nonce'])")
curl -s -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" -H "X-Nonce: $NONCE" \
  -d '{"email":"admin@demo.com","password":"AdminPass123!","display_name":"Demo Admin","role":"supporter"}'

# Step 2: Bootstrap admin (only works when no admin exists yet)
TOKEN=$(curl -s http://localhost:3000/api/auth/nonce | python3 -c "import sys,json; print(json.load(sys.stdin)['nonce'])" && \
  curl -s -X POST http://localhost:3000/api/auth/login \
    -H "Content-Type: application/json" -H "X-Nonce: $(curl -s http://localhost:3000/api/auth/nonce | python3 -c "import sys,json; print(json.load(sys.stdin)['nonce'])")" \
    -d '{"email":"admin@demo.com","password":"AdminPass123!"}' | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])")
NONCE=$(curl -s http://localhost:3000/api/auth/nonce | python3 -c "import sys,json; print(json.load(sys.stdin)['nonce'])")
curl -s -X POST http://localhost:3000/api/admin/bootstrap \
  -H "Content-Type: application/json" -H "Authorization: Bearer $TOKEN" -H "X-Nonce: $NONCE" \
  -d '{"password":"AdminPass123!"}'
```

After setup, use these accounts in the UI at http://localhost:8080:

| Role | Email | Password |
|------|-------|----------|
| **Administrator** | `admin@demo.com` | `AdminPass123!` |
| **Project Manager** | `manager@example.org` | `SeedPass1` |

To create additional demo accounts for other roles, register via the UI (`/register`) and select the desired role (Supporter, Project Manager, or Finance Reviewer). Administrator role can only be assigned by an existing admin through the Admin panel.

## Project Structure

```
repo/
  backend/          Rust/Axum REST API server (SQLite, Tokio)
    src/
      main.rs       Server entry point, route wiring
      auth.rs       Password hashing, HMAC token signing
      crypto.rs     AES-256-GCM encryption for data at rest
      db.rs         SQLite schema, queries, seed data
      middleware.rs  Rate limiting, auth extraction, nonce validation
      routes/       Route handlers per domain
    tests/          Integration tests (cargo test --package server)
  frontend/         Rust/Leptos WASM frontend (Trunk)
    src/
      main.rs       App component, router setup
      lib.rs        Library target (exports for tests)
      api.rs        HTTP client (gloo-net)
      state.rs      Auth state management
      analytics.rs  Event tracking
      components/   Reusable UI components
      pages/        Page-level components
    tests/          WASM tests (wasm-pack test / cargo test --package web)
  common/           Shared DTOs, enums, request/response types
  database/         SQL schema reference (init.sql)
  docker-compose.yml
  run_tests.sh      Unified test runner
```

## Testing

### Run all tests

```bash
cd repo
./run_tests.sh
```

The test runner executes six phases:

1. **Backend unit & integration tests** (`cargo test --package server`) — covers auth, crypto, middleware, DB operations, and 80+ route-level integration tests including IDOR checks, role gating, nonce replay, two-step confirmation protocols, and multi-step workflow tests.
2. **Frontend WASM tests** (`wasm-pack test` or `cargo test --package web`) — covers DTO serde round-trips for all 40+ shared types, UI logic (budget bar thresholds, currency formatting, query builders, validation), and component render tests (BudgetBar, ProjectCard, ReceiptDisplay, HomePage).
3. **Service health checks** — verifies backend and frontend are reachable.
4. **API smoke tests** — curl-based checks for auth, nonce, role enforcement.
5. **Extended E2E checks** — 23 multi-step curl tests covering registration, login, donations, filtering, role gating, event tracking, and more.
6. **Frontend content checks** — verifies HTML and WASM bundle are served.

### Run tests without local toolchains (Docker only)

```bash
cd repo
# Backend tests via Docker
docker-compose run --rm --no-deps -e DATABASE_URL=":memory:" backend \
  sh -c "cd /app && cargo test --package server"

# Full test suite (starts services then runs smoke/E2E tests)
docker-compose up -d
./run_tests.sh
docker-compose down
```

### Run specific test suites

```bash
# Backend only
cargo test --package server

# Frontend only (needs wasm-pack)
cd frontend && wasm-pack test --headless --chrome

# Single test file
cargo test --package server --test api_extended
cargo test --package server --test workflow_e2e
cargo test --package server --test integration
```

## API Overview

All endpoints are prefixed with `/api`. Mutating requests require `X-Nonce` header. Authenticated routes require `Authorization: Bearer <token>`.

| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| GET | `/api/projects` | No | List projects (supports `?cause=`, `?status=`, `?zip_code=`, `?search=`, `?page=`, `?per_page=`) |
| GET | `/api/projects/{id}` | No | Project detail with budget lines and updates |
| GET | `/api/projects/{id}/comments` | No | Public comments for a project |
| POST | `/api/auth/register` | No | Register (role enforced server-side) |
| POST | `/api/auth/login` | No | Login, returns session token |
| GET | `/api/auth/nonce` | No | Get a fresh nonce for mutating requests |
| POST | `/api/events/track` | No | Track analytics event (anonymous allowed) |
| GET | `/api/auth/me` | Yes | Current user profile |
| PUT | `/api/auth/dnd` | Yes | Update Do Not Disturb settings |
| POST | `/api/projects` | PM/Admin | Create project with budget lines |
| POST | `/api/donations` | Yes | Make a donation |
| GET | `/api/donations/mine` | Yes | List own donations |
| POST | `/api/comments` | Yes | Post a comment (subject to moderation) |
| POST | `/api/admin/bootstrap` | Yes | Bootstrap first administrator |
| POST | `/api/admin/assign-role` | Admin | Assign role to user |
| GET | `/api/admin/stats` | PM/Fin/Admin | Dashboard statistics |
| GET | `/api/admin/ops-log` | Admin | Immutable operations log |
| GET | `/api/admin/export/csv` | Fin/Admin | Export donations as CSV |

See `backend/src/main.rs` for the complete route table (~50 endpoints).

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Port 3000 or 8080 already in use | Stop conflicting services or change ports in `docker-compose.yml` |
| Frontend shows blank page | Check browser console for WASM load errors; ensure backend is healthy first |
| `docker-compose up` fails to build | Run `docker-compose build --no-cache` to rebuild from scratch |
| "Nonce already used" (409) errors | Each POST/PUT/DELETE needs a fresh nonce; fetch `/api/auth/nonce` before each request |
| "Too Many Requests" (429) | Rate limit is 60 req/min per session; wait 60 seconds |
| Bootstrap returns 409 | An admin already exists; use the admin panel to assign roles instead |
| Receipts rejected as duplicate | Identical file content (same SHA-256) was already uploaded |
| Database locked errors | Ensure only one backend instance is running against the same DB file |

### Reset to clean state

```bash
cd repo
docker-compose down -v   # removes volumes (database + keys)
docker-compose up         # fresh start with seed data
```
