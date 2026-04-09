# Fund Transparency Project

A community-focused fundraising platform with a focus on spending transparency and auditability. This portal allows donors to contribute to community projects and verify that every cent is spent as intended through real-time expense reporting and receipt disclosure.

## Project Structure

This project follows a standardized task layout for clear separation of concerns:

- **`docs/`**: Architectural design documents, API specifications, and task records.
- **`repo/`**: The main codebase, divided into backend, frontend, and database sub-internals.
    - **`repo/backend/`**: Rust/Axum server handling logic, authentication, and API endpoints.
    - **`repo/frontend/`**: Rust/Leptos web application and static assets.
    - **`repo/common/`**: Shared Rust logic and data models for both frontend and backend.
    - **`repo/database/`**: SQL schema definitions and local data storage.
- **`sessions/`**: Placeholder for conversation logs and process records.

## Tech Stack

- **Backend**: Rust (Axum, Rusqlite, Tokio)
- **Frontend**: Rust (Leptos, Trunk)
- **Database**: SQLite 3

---
*Created as part of the Fund Transparency Remediation Task.*
