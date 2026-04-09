# API Specification: Fund Transparency

**Base URL**: `/api`

## Authentication

All private endpoints require a Bearer Token provided in the `Authorization` header.

### `POST /auth/register`
Creates a new user account. Defaults to `Supporter` role.

### `POST /auth/login`
Authenticates a user and returns a session token.

### `GET /auth/nonce`
Generates a one-time cryptographic nonce for replay protection.

## Projects

### `GET /projects`
Lists all active/published community projects. Supports filtering by cause, zip code, and status.

### `GET /projects/{id}`
Returns full details for a project, including its budget and spending updates.

### `POST /projects`
(Staff) Creates a new community project. Requires `Administrator` or `Project Manager` role.

## Donations & Spending

### `POST /donations`
Processes a new donation for a specified project.

### `POST /projects/expenses`
(Staff) Records a project-related expense against a specific budget line.

### `POST /donations/refund`
Requests a refund for a specific donation. Requires original donor ownership.

## Moderation & Admin

### `GET /moderation/comments/pending`
(Staff) Lists all comments awaiting moderation review.

### `GET /admin/ops-log`
(Admin) Returns the immutable operational log of sensitive system events.

---
*Created by Antigravity AI.*
