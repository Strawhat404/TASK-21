# Fund Transparency Portal

A community-focused fundraising platform built for spending transparency and auditability. Donors can contribute to community projects and verify that every cent is spent as intended through real-time expense reporting, receipt disclosure, and fulfillment verification.

## Project Structure

- `docs/` — Design documents, API spec, and clarifying questions
- `repo/` — Main codebase
  - `repo/backend/` — Rust/Axum REST API server (SQLite, Tokio)
  - `repo/frontend/` — Rust/Leptos WASM frontend (Trunk)
  - `repo/common/` — Shared data models and types
  - `repo/database/` — SQL schema (`init.sql`)
- `sessions/` — Session traces

## Tech Stack

- **Backend**: Rust, Axum, SQLite (rusqlite), Tokio
- **Frontend**: Rust, Leptos, Trunk (WASM)
- **Auth**: Argon2 password hashing, HMAC-signed session tokens, AES-GCM encryption at rest

## Running with Docker

```bash
cd repo
docker compose up
```

Frontend: `http://localhost:8080`  
Backend API: `http://localhost:3000`

## First Run

On first run, bootstrap an admin account:

```bash
curl -X POST http://localhost:3000/api/admin/bootstrap \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@example.com","password":"YourPassword123!","display_name":"Admin"}'
```
