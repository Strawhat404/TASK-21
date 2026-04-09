# Community Giving & Fund Transparency Portal

An offline-first platform for local nonprofits to run project-based donation drives,
disclose spending, and close the loop with supporter engagement and accountable
fulfillment proof.

## Architecture

```
repo/
  common/       Shared Rust types (DTOs, enums) used by both server and frontend
  backend/      Axum REST API server with SQLite persistence
  frontend/     Leptos CSR (WebAssembly) single-page application
  database/     Runtime SQLite data directory
```

### Roles

| Role             | Capabilities |
|------------------|-------------|
| Supporter        | Browse projects, donate, comment, favorite, follow updates |
| Project Manager  | Create projects, post updates, record expenses, manage fulfillment |
| Finance Reviewer | Verify receipts, approve/reject expense disclosures and refunds |
| Administrator    | All of the above plus moderation config, webhooks, ops log, user management |

Registration always assigns the **Supporter** role; higher roles are granted by
an Administrator via `POST /api/admin/assign-role`.

**Bootstrap (first admin):** On a fresh deployment with no administrators, any
authenticated user can call `POST /api/admin/bootstrap` with their password to
promote themselves to Administrator. This endpoint is disabled once an admin exists.

### Security

- **Authentication** — Argon2id password hashing, HMAC-SHA256 signed session
  tokens (24 h TTL).
- **Replay protection** — Every mutating request (POST/PUT/PATCH/DELETE) must
  carry a fresh `X-Nonce` header consumed from `GET /api/auth/nonce` (5-minute
  validity). Missing or reused nonces are rejected.
- **Rate limiting** — 60 requests per minute per session token.
- **IDOR prevention** — Project Managers can only modify their own projects.
  `require_project_owner()` is enforced on updates, expenses, fulfillments,
  ticket responses.
- **Data privacy** — Expenses, receipts, tickets, and fulfillment records are
  behind authentication with project-scoped authorization. PII is masked in CSV
  exports.
- **Encryption at rest** — Reviewer notes on expenses and receipt file data are
  encrypted with AES-256-GCM (nonce || ciphertext) using a host-managed key
  stored outside the repository tree.
- **SQL hardening** — All queries use parameterized placeholders; no user input
  is ever interpolated into SQL strings.
- **Content moderation** — Configurable sensitive-word filter and optional
  pre-moderation queue.
- **Sensitive actions** — Project unpublish, comment removal, and refund
  approval all require password re-entry.

### Offline Payments

Donations are recorded as local cash / check / card-terminal entries. Refunds
create negative reversal records that require Finance Reviewer approval (with
password confirmation). Unapproved reversals are excluded from all accounting
totals.

### Fulfillment Verification

Three-checkpoint flow (arrival, start, end) using 6-digit OTP codes with
10-minute expiry. Time-consistency rules: start must be within 2 hours of
arrival; end must follow start. Completion generates a SHA-256 tamper-evident
service record downloadable as proof.

### Webhooks

On-prem only — URLs are validated by parsing the host and checking against
RFC 1918 / loopback / link-local IP ranges. Payloads are HMAC-SHA256 signed,
retried up to 3 times with exponential backoff, and every delivery attempt is
logged.

### Event Instrumentation

Unified local event model (impressions, clicks, dwell time, sessions) with
3-second deduplication, burst detection (>20 events / 10 s flagged), and
data-quality metrics on the admin KPI console.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ENCRYPTION_KEY_PATH` | `/var/lib/fund_transparency/encryption.key` | AES-256 encryption key file (auto-generated if missing) |
| `HMAC_KEY_PATH` | `/var/lib/fund_transparency/hmac.key` | HMAC signing secret file (auto-generated if missing) |
| `DB_PATH` | `data/fund_transparency.db` | SQLite database file path |
| `CORS_ALLOWED_ORIGINS` | `http://localhost:8080,http://127.0.0.1:8080` | Comma-separated list of allowed CORS origins |

Keys are stored **outside the repository tree** by default. For local
development, override to a writable path:

```bash
export ENCRYPTION_KEY_PATH=./data/encryption.key
export HMAC_KEY_PATH=./data/hmac.key
```

The `data/` directory is in `.gitignore` and must never be committed.

## Prerequisites

- Rust stable (1.75+)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev/) for the WASM frontend: `cargo install trunk`

## Building

```bash
# Build the backend
cargo build -p server --release

# Build the frontend (WASM)
cargo build -p web --target wasm32-unknown-unknown --release
# Or use Trunk for a dev server with hot reload:
cd frontend && trunk serve
```

## Running

```bash
# Start the API server (port 3000)
cargo run -p server

# In another terminal, start the frontend dev server (port 8080, proxies /api to :3000)
cd frontend && trunk serve
```

The server creates `data/fund_transparency.db`, `data/hmac.key`, and
`data/encryption.key` on first run.

## Testing

```bash
cargo test -p server
```

The test suite covers registration role lock, auth token lifecycle, nonce
replay rejection, refund ownership, IDOR checks, webhook URL validation, and
accounting integrity.

## API Overview

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/auth/register` | No | Register (always Supporter) |
| POST | `/api/auth/login` | No | Login, returns signed token |
| GET  | `/api/auth/nonce` | No | Fresh replay-protection nonce |
| GET  | `/api/projects` | No | List/filter/search projects |
| GET  | `/api/projects/:id` | No | Project detail with budget breakdown |
| POST | `/api/donations` | Token | Record offline donation |
| POST | `/api/donations/refund` | Token | Request refund (donor only) |
| POST | `/api/donations/refund/approve` | Token+pw | Approve/reject refund (finance) |
| POST | `/api/comments` | Token | Post comment (moderation-aware) |
| POST | `/api/comments/:id/delete` | Token+pw | Delete comment (admin only) |
| POST | `/api/receipts/upload` | Token | Upload receipt (PDF/JPG/PNG, 10 MB max, SHA-256 dedup) |
| POST | `/api/receipts/review` | Token | Verify/reject receipt (finance) |
| POST | `/api/fulfillments` | Token | Create fulfillment record |
| POST | `/api/fulfillments/code` | Token | Generate checkpoint OTP |
| POST | `/api/fulfillments/checkpoint` | Token | Record checkpoint with OTP |
| GET  | `/api/admin/stats` | Token | Dashboard KPIs with date range |
| GET  | `/api/admin/export/csv` | Token | Masked donation CSV export |
