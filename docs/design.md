# Architectural Design Document: Fund Transparency

**Project Purpose**: A fundraising portal for community projects emphasizing auditability and spending transparency. For every donation, a donor should be able to see the exact expense it contributed to and view the corresponding receipt.

## 1. High-Level Architecture

The platform follows a client-server architecture with a shared common library for type safety across the Rust frontend and backend.

```
┌──────────────────────┐      REST + Nonce       ┌──────────────────────┐
│   Leptos Frontend    │ ──────────────────────►  │    Axum Backend      │
│   (WASM, CSR)        │ ◄──────────────────────  │    (Rust, Tokio)     │
└──────────────────────┘       JSON responses     └──────────┬───────────┘
         │                                                    │
         │  shared types                                      │ SQL queries
         ▼                                                    ▼
┌──────────────────────┐                          ┌──────────────────────┐
│   Common Library     │                          │   SQLite Database    │
│   (Enums, DTOs)      │                          │   (WAL mode, FK on)  │
└──────────────────────┘                          └──────────────────────┘
```

### 1a. Component Overview

| Component | Technology | Role |
|-----------|-----------|------|
| **Backend** | Rust, Axum 0.7, Tokio | Stateless REST API server. Handles authentication, authorization, business logic, encryption, and database persistence. |
| **Frontend** | Rust, Leptos 0.6 (CSR), Trunk | WebAssembly SPA served via Nginx. Provides the UI for donors, project managers, finance reviewers, and administrators. |
| **Common** | Rust crate | Shared enums (`Role`, `ProjectStatus`, `DisclosureStatus`, etc.) and data transfer objects. Eliminates type drift between frontend and backend. |
| **Database** | SQLite 3 | Persistent structured storage. WAL journal mode for concurrent reads. Foreign keys enforced. |

### 1b. Deployment Architecture

Both services are containerized via Docker Compose:

```
┌─────────────────────────────────────────────────┐
│                 Docker Compose                   │
│                                                  │
│  ┌─────────────┐        ┌─────────────────────┐ │
│  │  Frontend    │        │   Backend           │ │
│  │  (Nginx)     │        │   (Rust binary)     │ │
│  │  :8080 → :80 │        │   :3000             │ │
│  └─────────────┘        └──────────┬──────────┘ │
│                                     │            │
│                          ┌──────────▼──────────┐ │
│                          │  Volumes            │ │
│                          │  - db_data (SQLite) │ │
│                          │  - key_data (keys)  │ │
│                          └─────────────────────┘ │
└─────────────────────────────────────────────────┘
```

- **Backend container**: Built from a multi-stage Dockerfile (`rust:1.87-bookworm` builder → `debian:bookworm-slim` runtime). Includes `curl` for health checks.
- **Frontend container**: Trunk compiles the Leptos app to WASM, served by Nginx on port 80.
- **Health check**: Backend exposes `GET /api/projects` as a liveness probe (10s interval, 5 retries, 15s start period).
- **Volumes**: `db_data` persists the SQLite database; `key_data` persists HMAC and encryption keys outside the repo tree.

---

## 2. Database Design

### 2a. Schema Overview

The database contains 20 tables organized into logical domains:

| Domain | Tables | Purpose |
|--------|--------|---------|
| **Identity** | `users` | User accounts with Argon2 password hashes and DND preferences |
| **Projects** | `projects`, `budget_lines`, `spending_updates`, `update_likes` | Project definitions, budget allocations, and progress updates |
| **Funding** | `donations`, `expenses`, `receipts` | Financial records with reversal tracking and encrypted receipt storage |
| **Community** | `comments`, `favorites`, `subscriptions`, `tickets` | User engagement: comments, favorites, subscriptions, support tickets |
| **Notifications** | `notifications` | Per-user notification queue with DND deferral support |
| **Security** | `nonces`, `ops_log`, `sensitive_confirmations` | Replay protection, immutable audit log, two-step confirmation tokens |
| **Moderation** | `moderation_config` | Singleton row for comment moderation settings |
| **Fulfillment** | `fulfillments`, `checkpoint_codes` | Service verification with OTP-based checkpoints |
| **Analytics** | `analytics_events` | User interaction tracking with dedup and fraud detection |
| **Webhooks** | `webhooks`, `webhook_delivery_log` | Outbound event delivery with retry tracking |

### 2b. Key Design Decisions

- **Text primary keys**: All IDs are UUIDs stored as `TEXT`. This avoids integer overflow on high-volume tables and simplifies distributed ID generation.
- **Soft status tracking**: Projects use a `status` column (`draft` → `active` → `funded` → `closed` / `unpublished`) rather than hard deletes.
- **Donation reversals**: Refunds are modeled as negative donation records (`is_reversal = 1`, `reversal_of = original_id`). They remain excluded from totals until `reversal_approved = 1`.
- **Immutable audit log**: The `ops_log` table has `BEFORE UPDATE` and `BEFORE DELETE` triggers that `RAISE(ABORT)`, making the log append-only at the database level.
- **Indexes**: Strategic indexes on foreign keys, timestamps, and frequently filtered columns (`cause`, `status`, `zip_code`) to support listing and filtering queries.

---

## 3. Backend Architecture

### 3a. Request Pipeline

Every request passes through a layered middleware stack before reaching the route handler:

```
Request
  │
  ▼
┌─────────────────────┐
│  CORS Layer          │  Validates origin, methods, and headers
├─────────────────────┤
│  Rate Limit Layer    │  60 req/min per token or IP
├─────────────────────┤
│  Nonce Layer         │  Validates X-Nonce header on POST/PUT/DELETE
├─────────────────────┤
│  Auth Layer          │  Extracts AuthUser from Bearer token (authed routes only)
├─────────────────────┤
│  Route Handler       │  Business logic + DB queries
└─────────────────────┘
  │
  ▼
Response
```

### 3b. Module Structure

```
backend/src/
├── main.rs          Application entry, route registration, middleware wiring
├── lib.rs           AppState definition (DbPool, HMAC secret, encryption key, rate limiter)
├── auth.rs          Argon2 password hashing, HMAC-SHA256 token creation/validation, nonce generation
├── crypto.rs        AES-256-GCM encrypt/decrypt for strings and bytes, key management
├── middleware.rs     Auth extraction, nonce validation, rate limiting, role/ownership checks
├── db.rs            All SQLite queries (schema creation, CRUD, aggregations)
└── routes/
    ├── mod.rs           Module re-exports
    ├── auth_routes.rs   Register, login, nonce, me, DND settings
    ├── projects.rs      CRUD, updates, expenses, favorites, subscriptions, likes
    ├── donations.rs     Donate, my donations, refund request, refund approval, pending refunds
    ├── comments.rs      Comment CRUD, tickets, ticket responses
    ├── notifications.rs List, mark read, mark all read
    ├── receipts.rs      Upload (encrypted), review, pending, list by expense
    ├── moderation.rs    Config get/set, pending comments, comment review
    ├── fulfillment.rs   Create, list, get, code generation, checkpoint recording, proof
    ├── events.rs        Track event, data quality metrics, suspicious events
    ├── webhooks.rs      CRUD, delivery log
    ├── finance.rs       Pending expenses, expense review
    └── admin.rs         Stats, ops log, unpublish, role assignment, bootstrap, CSV export
```

### 3c. Application State

All route handlers share an `AppState` via Axum's state extraction:

```rust
pub struct AppState {
    pub db: DbPool,              // Arc<Mutex<Connection>> — serialized SQLite access
    pub hmac_secret: Vec<u8>,    // 64-byte key for session token signing
    pub encryption_key: [u8; 32], // AES-256 key for receipt/note encryption
    pub rate_limiter: RateLimitState, // In-memory sliding window rate limiter
}
```

---

## 4. Frontend Architecture

### 4a. Technology

The frontend is a Leptos 0.6 CSR (client-side rendered) application compiled to WebAssembly via Trunk. It communicates with the backend exclusively through JSON REST calls.

### 4b. Routing & Pages

| Path | Component | Access |
|------|-----------|--------|
| `/` | `HomePage` | Public |
| `/login` | `LoginPage` | Public |
| `/register` | `RegisterPage` | Public |
| `/projects` | `ProjectListPage` | Public |
| `/projects/:id` | `ProjectDetailPage` | Public |
| `/donate/:id` | `DonatePage` | `AuthGuard` |
| `/profile` | `ProfilePage` | `AuthGuard` |
| `/dashboard` | `DashboardPage` | `RoleGuard` (PM, FR, Admin) |
| `/admin` | `AdminPage` | `RoleGuard` (Admin) |
| `/finance` | `FinancePage` | `RoleGuard` (FR, Admin) |
| `/staff` | `StaffPage` | `RoleGuard` (PM, FR, Admin) |
| `/fulfillment/:id` | `FulfillmentPage` | `RoleGuard` (PM, Admin) |
| `/fulfillment/:id/proof` | `ServiceProofPage` | `RoleGuard` (PM, Admin) |

### 4c. State Management

- **Auth state**: Global `AuthState` provided via Leptos context. Stores `auth_token` and `current_user` in both reactive signals and `localStorage`.
- **API layer**: Centralized `api.rs` module provides typed `get_json<T>` and `post_json<T, B>` helpers. All mutating requests automatically fetch a nonce before sending.
- **Route guards**: `AuthGuard` redirects unauthenticated users to `/login`. `RoleGuard` checks the user's role against an allowed list.

### 4d. Component Structure

```
frontend/src/
├── main.rs            App root, router, global state providers
├── state.rs           AuthState type definition
├── api.rs             HTTP client (nonce-aware, typed JSON helpers)
├── analytics.rs       Client-side event tracking (emits to /api/events/track)
├── components/
│   ├── nav.rs              Navigation bar with role-aware links
│   ├── project_card.rs     Project summary card
│   ├── budget_bar.rs       Budget progress visualization
│   ├── comment_section.rs  Comment list + compose form with moderation status
│   ├── notification_center.rs  Notification dropdown with mark-read actions
│   ├── receipt.rs          Receipt display component
│   ├── route_guard.rs      AuthGuard + RoleGuard wrapper components
│   └── mod.rs
└── pages/
    ├── home.rs, login.rs, project_list.rs, project_detail.rs
    ├── donate.rs, profile.rs, dashboard.rs
    ├── admin.rs, staff.rs, finance.rs, fulfillment.rs
    └── mod.rs
```

---

## 5. Security Model

### 5a. Authentication

- **Password storage**: Argon2id with random salt per user.
- **Session tokens**: `base64url(user_id|expiry_epoch).hmac_sha256_hex`. Tokens are stateless — validation requires only the HMAC secret. TTL: 24 hours.
- **Nonce replay protection**: Every mutating request (`POST`, `PUT`, `DELETE`) must include an `X-Nonce` header. The nonce is consumed server-side and expires after 5 minutes. Replayed nonces return `409 Conflict`.

### 5b. Authorization

| Layer | Mechanism |
|-------|-----------|
| **Route-level** | Auth middleware rejects unauthenticated requests to protected routes (`401`) |
| **Role-level** | `require_role()` checks user role against allowed list (`403`) |
| **Ownership-level** | `require_project_owner()` verifies the user manages the target project (`403`) |
| **Two-step confirmation** | Sensitive actions (unpublish, refund approval, comment deletion) require password + server-issued confirmation token |

### 5c. Encryption at Rest

- **Algorithm**: AES-256-GCM (authenticated encryption with associated data).
- **Scope**: Receipt file data, expense receipt fields, and reviewer notes are encrypted before storage.
- **Key management**: Encryption key is loaded from `ENCRYPTION_KEY_PATH` (default: `/var/lib/fund_transparency/encryption.key`). Auto-generated on first run. Must be at least 32 bytes.

### 5d. Data Privacy

- **PII masking**: Email addresses are masked in ops_log entries (e.g., `a***@example.com`).
- **CSV export masking**: The `GET /admin/export/csv` endpoint masks donor PII in exported data.
- **Immutable audit log**: All sensitive operations (role changes, refund approvals, project unpublishing) are logged to `ops_log` with database-enforced immutability.

### 5e. Rate Limiting

- **Implementation**: In-memory sliding window rate limiter using `parking_lot::Mutex<HashMap<String, Vec<Instant>>>`.
- **Key**: Authenticated requests are keyed by the `Authorization` header value; anonymous requests are keyed by `X-Forwarded-For`, `X-Real-IP`, or a fallback "unknown" key.
- **Limit**: 60 requests per 60-second window. Returns `429 Too Many Requests` when exceeded.

### 5f. Webhook Security

- Webhook URLs are restricted to local/private network addresses (localhost, `.local`, `127.0.0.1`, `10.x.x.x`, `172.16-31.x.x`, `192.168.x.x`, `169.254.x.x`, `::1`).
- Each webhook receives an HMAC-SHA256 signing secret for payload verification.
- Failed deliveries are retried up to 3 times with exponential backoff (1s, 2s, 4s).

---

## 6. Key Workflows

### 6a. Donation & Refund Flow

```
Donor                          Backend                     Finance Reviewer
  │                               │                              │
  │── POST /donations ──────────► │                              │
  │                               │── auto-subscribe donor       │
  │                               │── fire webhooks              │
  │◄── DonationRecord ────────── │                              │
  │                               │                              │
  │── POST /donations/refund ───► │                              │
  │                               │── create reversal record     │
  │◄── DonationRecord ────────── │                              │
  │                               │                              │
  │                               │◄── POST /donations/refund/approve (step 1)
  │                               │──► confirmation_token ──────►│
  │                               │◄── POST /donations/refund/approve (step 2)
  │                               │── approve/reject reversal    │
  │                               │──► ApiSuccess ──────────────►│
```

- Donations target an active project, optionally directed to a specific budget line.
- Budget lines are validated to belong to the target project (cross-project budget line rejection).
- Unapproved reversals are excluded from raised/spent totals.

### 6b. Expense Disclosure Flow

```
Project Manager              Backend                  Finance Reviewer
  │                             │                            │
  │── POST /projects/expenses ─►│                            │
  │                             │── encrypt receipt data     │
  │                             │── validate budget line     │
  │◄── ApiSuccess ─────────────│                            │
  │                             │                            │
  │── POST /receipts/upload ──►│                            │
  │                             │── encrypt file (AES-GCM)  │
  │                             │── SHA-256 fingerprint      │
  │                             │── duplicate detection      │
  │◄── ReceiptRecord ─────────│                            │
  │                             │                            │
  │                             │◄── POST /finance/review ──│
  │                             │── approve/reject expense   │
  │                             │◄── POST /receipts/review ─│
  │                             │── verify/reject receipt    │
```

- Expenses start as `pending` and become publicly visible only after finance review approval.
- Receipt uploads are validated for file type (PDF, JPEG, PNG) and size (max 10 MB).

### 6c. Fulfillment Verification Flow

```
Project Manager                        Backend
  │                                       │
  │── POST /fulfillments ────────────────►│ create record
  │                                       │
  │── POST /fulfillments/code ───────────►│ generate OTP + QR
  │◄── CheckpointCodeResponse ───────────│ (6-digit, 10-min expiry)
  │                                       │
  │── POST /fulfillments/checkpoint ─────►│ validate code
  │   (arrival)                           │ record timestamp
  │                                       │
  │── POST /fulfillments/code ───────────►│
  │── POST /fulfillments/checkpoint ─────►│ (start, within 2h of arrival)
  │                                       │
  │── POST /fulfillments/code ───────────►│
  │── POST /fulfillments/checkpoint ─────►│ (end, after start)
  │                                       │── compute SHA-256 hash
  │                                       │── mark complete
  │                                       │
  │── GET /fulfillments/{id}/proof ──────►│
  │◄── ServiceProof ─────────────────────│ (immutable hash)
```

### 6d. Content Moderation Flow

- Comments can be globally enabled/disabled via moderation config.
- When `require_pre_moderation` is enabled, new comments enter `pending_review` status.
- A sensitive word filter automatically flags comments containing configured words.
- Project managers can moderate comments on their own projects; administrators can moderate all.

---

## 7. Error Handling

- All API errors return JSON: `{ "error": "description" }` with appropriate HTTP status codes.
- Standard status codes: `400` (bad request), `401` (unauthorized), `403` (forbidden), `404` (not found), `409` (nonce conflict), `429` (rate limited), `500` (internal error).
- The backend avoids panics in request handlers — errors are propagated via `Result` types and mapped to status codes.
- Database errors in non-critical paths (e.g., notification creation) are logged but do not fail the primary operation.

---

## 8. Observability

- **Ops log**: Every sensitive action (role change, refund approval, project unpublish, etc.) is recorded with actor, action, detail, and timestamp. The log is immutable at the database level.
- **Webhook delivery log**: Tracks every webhook delivery attempt with status code, success flag, and error message.
- **Analytics events**: Client-side and server-side event tracking with duplicate and suspicious burst detection. Data quality metrics available via `GET /events/quality`.
- **Structured logging**: Backend uses `RUST_LOG` environment variable for log level control.
