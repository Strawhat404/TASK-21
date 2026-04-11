# API Specification: Fund Transparency

**Base URL**: `/api`

## Authentication

All private endpoints require a Bearer Token in the `Authorization` header:

```
Authorization: Bearer <token>
```

Tokens are HMAC-SHA256 signed and expire after 24 hours. All mutating requests (`POST`, `PUT`, `DELETE`) must include an `X-Nonce` header with a one-time nonce obtained from `GET /auth/nonce`.

**Rate Limiting**: 60 requests per 60-second window, keyed by token (authenticated) or client IP (anonymous). Exceeding the limit returns `429 Too Many Requests`.

### Roles

| Role | Description |
|------|-------------|
| `supporter` | Default role. Can donate, comment, favorite, and subscribe. |
| `project_manager` | Can create projects, post updates, record expenses, manage fulfillments. |
| `finance_reviewer` | Approves/rejects expenses, receipts, and refunds. |
| `administrator` | Full access: role assignment, moderation config, webhooks, system admin. |

### Common Response Types

**Success**:
```json
{ "message": "string" }
```

**Error**:
```json
{ "error": "string" }
```

**Two-Step Confirmation** (sensitive actions return this on step 1):
```json
{ "confirmation_token": "string", "message": "string" }
```

---

## 1. Authentication

### `POST /auth/register`

Creates a new user account. The server ignores the `role` field and always assigns `supporter`.

**Request**:
```json
{
  "email": "string",
  "password": "string",
  "display_name": "string",
  "role": "string (ignored)"
}
```

**Response** `200`:
```json
{
  "token": "string",
  "user": {
    "id": "string",
    "email": "string",
    "display_name": "string",
    "role": "supporter",
    "dnd_start": "21:00",
    "dnd_end": "07:00",
    "timezone": "UTC",
    "created_at": "string (ISO 8601)"
  }
}
```

---

### `POST /auth/login`

Authenticates an existing user and returns a session token.

**Request**:
```json
{
  "email": "string",
  "password": "string"
}
```

**Response** `200`:
```json
{
  "token": "string",
  "user": { "...UserProfile" }
}
```

**Errors**: `401 Unauthorized` — invalid credentials.

---

### `GET /auth/nonce`

Generates a one-time cryptographic nonce for replay protection. Nonces expire after 5 minutes.

**Auth**: Not required.

**Response** `200`:
```json
{ "nonce": "string (hex)" }
```

---

### `GET /auth/me`

Returns the profile of the currently authenticated user.

**Auth**: Required.

**Response** `200`:
```json
{
  "id": "string",
  "email": "string",
  "display_name": "string",
  "role": "supporter | project_manager | finance_reviewer | administrator",
  "dnd_start": "string (HH:MM)",
  "dnd_end": "string (HH:MM)",
  "timezone": "string",
  "created_at": "string (ISO 8601)"
}
```

---

### `PUT /auth/dnd`

Updates the user's Do-Not-Disturb notification window.

**Auth**: Required.

**Request**:
```json
{
  "dnd_start": "string (HH:MM)",
  "dnd_end": "string (HH:MM)",
  "timezone": "string (UTC offset, e.g. '+05:30', '-08:00', or 'UTC')"
}
```

**Response** `200`:
```json
{ "message": "DND settings updated" }
```

---

## 2. Projects

### `GET /projects`

Lists all published community projects. Supports filtering and pagination.

**Auth**: Not required.

**Query Parameters**:
| Param | Type | Description |
|-------|------|-------------|
| `cause` | string | Filter by cause category |
| `status` | string | Filter by project status |
| `zip_code` | string | Filter by ZIP code |
| `search` | string | Full-text search on title/description |
| `page` | integer | Page number (default: 1) |
| `per_page` | integer | Items per page (default: 20) |

**Response** `200`:
```json
{
  "items": [
    {
      "id": "string",
      "title": "string",
      "cause": "string",
      "zip_code": "string",
      "status": "draft | active | funded | closed | unpublished",
      "goal_cents": 0,
      "raised_cents": 0,
      "spent_cents": 0,
      "manager_name": "string",
      "created_at": "string (ISO 8601)"
    }
  ],
  "total": 0,
  "page": 1,
  "per_page": 20
}
```

---

### `GET /projects/{id}`

Returns full details for a single project including budget lines and spending updates.

**Auth**: Not required.

**Response** `200`:
```json
{
  "id": "string",
  "title": "string",
  "description": "string",
  "cause": "string",
  "zip_code": "string",
  "status": "string",
  "goal_cents": 0,
  "raised_cents": 0,
  "spent_cents": 0,
  "manager_id": "string",
  "manager_name": "string",
  "budget_lines": [
    {
      "id": "string",
      "project_id": "string",
      "name": "string",
      "allocated_cents": 0,
      "spent_cents": 0
    }
  ],
  "updates": [
    {
      "id": "string",
      "project_id": "string",
      "title": "string",
      "body": "string",
      "author_name": "string",
      "like_count": 0,
      "created_at": "string (ISO 8601)"
    }
  ],
  "created_at": "string (ISO 8601)"
}
```

---

### `POST /projects`

Creates a new community project with budget lines.

**Auth**: Required — `project_manager` or `administrator`.

**Request**:
```json
{
  "title": "string",
  "description": "string",
  "cause": "string",
  "zip_code": "string",
  "goal_cents": 0,
  "budget_lines": [
    { "name": "string", "allocated_cents": 0 }
  ]
}
```

**Response** `200`: `ProjectDetail` (same as `GET /projects/{id}`).

---

### `POST /projects/updates`

Posts a spending update to a project. Notifies all project subscribers.

**Auth**: Required — `project_manager` or `administrator`.

**Request**:
```json
{
  "project_id": "string",
  "title": "string",
  "body": "string"
}
```

**Response** `200`:
```json
{ "message": "Update posted" }
```

---

### `POST /projects/expenses`

Records a project expense against a specific budget line.

**Auth**: Required — `project_manager` or `administrator`.

**Request**:
```json
{
  "project_id": "string",
  "budget_line_id": "string",
  "amount_cents": 0,
  "description": "string",
  "receipt_data": "string (optional, base64)"
}
```

**Response** `200`:
```json
{ "message": "Expense recorded" }
```

---

### `GET /projects/{id}/expenses`

Returns all expenses for a project.

**Auth**: Required — project owner, `finance_reviewer`, or `administrator`.

**Response** `200`:
```json
[
  {
    "id": "string",
    "project_id": "string",
    "budget_line_id": "string",
    "budget_line_name": "string",
    "amount_cents": 0,
    "description": "string",
    "receipt_url": "string | null",
    "disclosure_status": "pending | approved | rejected",
    "reviewer_note": "string | null",
    "created_at": "string (ISO 8601)"
  }
]
```

---

### `POST /projects/{id}/favorite`

Toggles the current user's favorite status for a project.

**Auth**: Required.

**Response** `200`:
```json
{ "favorited": true }
```

---

### `GET /favorites`

Returns a list of project IDs the current user has favorited.

**Auth**: Required.

**Response** `200`:
```json
["project-id-1", "project-id-2"]
```

---

### `GET /favorites/projects`

Returns full project summaries for all favorited projects.

**Auth**: Required.

**Response** `200`:
```json
[ { "...ProjectSummary" } ]
```

---

### `POST /projects/{id}/subscribe`

Subscribes the current user to project update notifications.

**Auth**: Required.

**Response** `200`:
```json
{ "message": "Subscribed" }
```

---

### `POST /projects/{id}/unsubscribe`

Unsubscribes the current user from project update notifications.

**Auth**: Required.

**Response** `200`:
```json
{ "message": "Unsubscribed" }
```

---

### `POST /updates/{id}/like`

Toggles a like on a spending update.

**Auth**: Required.

**Response** `200`:
```json
{ "liked": true }
```

---

## 3. Donations & Refunds

### `POST /donations`

Processes a donation to a project. Automatically subscribes the donor to updates. Fires webhooks.

**Auth**: Required.

**Request**:
```json
{
  "project_id": "string",
  "amount_cents": 0,
  "payment_method": "cash | check | card_terminal (optional, default: cash)",
  "budget_line_id": "string (optional, must belong to project)"
}
```

**Response** `200`:
```json
{
  "donation": {
    "id": "string",
    "pledge_number": "string",
    "project_id": "string",
    "project_title": "string",
    "donor_id": "string",
    "amount_cents": 0,
    "payment_method": "string",
    "is_reversal": false,
    "reversal_of": null,
    "reversal_approved": null,
    "budget_line_id": "string | null",
    "budget_line_name": "string | null",
    "created_at": "string (ISO 8601)"
  }
}
```

---

### `GET /donations/mine`

Returns all donations made by the authenticated user.

**Auth**: Required.

**Response** `200`:
```json
[ { "...DonationRecord" } ]
```

---

### `POST /donations/refund`

Requests a refund for a donation. Creates a negative reversal record. Only the original donor can request.

**Auth**: Required.

**Request**:
```json
{
  "donation_id": "string",
  "reason": "string"
}
```

**Response** `200`:
```json
{ "donation": { "...DonationRecord (reversal)" } }
```

---

### `POST /donations/refund/approve`

Approves or rejects a pending refund. Uses two-step confirmation with password verification.

**Auth**: Required — `finance_reviewer` or `administrator`.

**Request (step 1 — get confirmation token)**:
```json
{
  "donation_id": "string",
  "approved": true,
  "password": "string"
}
```

**Response (step 1)** `200`:
```json
{ "confirmation_token": "string", "message": "Confirm action" }
```

**Request (step 2 — execute)**:
```json
{
  "donation_id": "string",
  "approved": true,
  "password": "string",
  "confirmation_token": "string"
}
```

**Response (step 2)** `200`:
```json
{ "message": "Refund approved" }
```

---

### `GET /donations/refund/pending`

Lists all pending (unapproved) refund reversals.

**Auth**: Required — `finance_reviewer` or `administrator`.

**Response** `200`:
```json
[ { "...DonationRecord (reversals)" } ]
```

---

## 4. Comments & Tickets

### `GET /projects/{id}/comments`

Lists approved comments for a project.

**Auth**: Not required.

**Response** `200`:
```json
[
  {
    "id": "string",
    "project_id": "string",
    "author_id": "string",
    "author_name": "string",
    "body": "string",
    "moderation_status": "string",
    "created_at": "string (ISO 8601)"
  }
]
```

---

### `POST /comments`

Creates a new comment on a project. Checks moderation config and sensitive word filters. May queue for pre-moderation review.

**Auth**: Required.

**Request**:
```json
{
  "project_id": "string",
  "body": "string"
}
```

**Response** `200`:
```json
{ "message": "Comment created" }
```

---

### `POST /comments/{id}/delete`

Deletes a comment. Uses two-step confirmation with password verification.

**Auth**: Required — `administrator` only.

**Request**:
```json
{
  "password": "string",
  "confirmation_token": "string (optional, omit for step 1)"
}
```

**Response**: Two-step flow (see "Two-Step Confirmation" in Common Response Types).

---

### `GET /projects/{id}/tickets`

Lists support tickets for a project.

**Auth**: Required — project owner, `finance_reviewer`, or `administrator`.

**Response** `200`:
```json
[
  {
    "id": "string",
    "project_id": "string",
    "submitter_id": "string",
    "submitter_name": "string",
    "subject": "string",
    "body": "string",
    "status": "open | closed",
    "response": "string | null",
    "created_at": "string (ISO 8601)"
  }
]
```

---

### `POST /tickets`

Submits a support ticket for a project.

**Auth**: Required.

**Request**:
```json
{
  "project_id": "string",
  "subject": "string",
  "body": "string"
}
```

**Response** `200`:
```json
{ "message": "Ticket submitted" }
```

---

### `POST /tickets/respond`

Responds to a ticket. Only the project owner or an admin can respond.

**Auth**: Required — `project_manager` or `administrator`.

**Request**:
```json
{
  "ticket_id": "string",
  "response": "string"
}
```

**Response** `200`:
```json
{ "message": "Ticket response recorded" }
```

---

## 5. Notifications

### `GET /notifications`

Lists all notifications for the current user. Notifications during DND windows are deferred.

**Auth**: Required.

**Response** `200`:
```json
[
  {
    "id": "string",
    "user_id": "string",
    "title": "string",
    "body": "string",
    "is_read": false,
    "is_deferred": false,
    "created_at": "string (ISO 8601)"
  }
]
```

---

### `POST /notifications/{id}/read`

Marks a single notification as read.

**Auth**: Required.

**Response** `200`:
```json
{ "message": "Notification marked as read" }
```

---

### `POST /notifications/read-all`

Marks all notifications as read for the current user.

**Auth**: Required.

**Response** `200`:
```json
{ "message": "All notifications marked as read" }
```

---

## 6. Receipts & Vouchers

### `POST /receipts/upload`

Uploads a receipt for an expense. Files are AES-256-GCM encrypted at rest and SHA-256 fingerprinted for deduplication.

**Auth**: Required — `project_manager` or `administrator` (must own the project).

**Constraints**: Accepted file types: `application/pdf`, `image/jpeg`, `image/png`. Max size: 10 MB.

**Request**:
```json
{
  "expense_id": "string",
  "file_name": "string",
  "file_type": "string (MIME)",
  "file_size": 0,
  "file_data_base64": "string"
}
```

**Response** `200`:
```json
{
  "id": "string",
  "expense_id": "string",
  "file_name": "string",
  "file_type": "string",
  "file_size": 0,
  "sha256_fingerprint": "string (hex)",
  "status": "uploaded",
  "rejection_reason": null,
  "reviewer_id": null,
  "created_at": "string (ISO 8601)"
}
```

---

### `POST /receipts/review`

Approves or rejects a receipt. Rejection requires a reason.

**Auth**: Required — `finance_reviewer` or `administrator`.

**Request**:
```json
{
  "receipt_id": "string",
  "verified": true,
  "rejection_reason": "string (required if verified=false)"
}
```

**Response** `200`:
```json
{ "message": "Receipt reviewed" }
```

---

### `GET /receipts/pending`

Lists all receipts awaiting review.

**Auth**: Required — `finance_reviewer` or `administrator`.

**Response** `200`:
```json
[ { "...ReceiptRecord" } ]
```

---

### `GET /expenses/{id}/receipts`

Lists all receipts attached to an expense.

**Auth**: Required — project owner, `finance_reviewer`, or `administrator`.

**Response** `200`:
```json
[ { "...ReceiptRecord" } ]
```

---

## 7. Content Moderation

### `GET /moderation/config`

Returns the current moderation configuration.

**Auth**: Required — `administrator` only.

**Response** `200`:
```json
{
  "comments_enabled": true,
  "require_pre_moderation": false,
  "sensitive_words": ["word1", "word2"]
}
```

---

### `PUT /moderation/config`

Updates the moderation configuration.

**Auth**: Required — `administrator` only.

**Request**:
```json
{
  "comments_enabled": true,
  "require_pre_moderation": false,
  "sensitive_words": ["word1", "word2"]
}
```

**Response** `200`:
```json
{ "message": "Moderation config updated" }
```

---

### `GET /moderation/comments/pending`

Lists comments awaiting moderation review. Admins see all; project managers see only their own projects' comments.

**Auth**: Required — `project_manager` or `administrator`.

**Response** `200`:
```json
[ { "...Comment" } ]
```

---

### `POST /moderation/comments/review`

Approves or rejects a pending comment. Project managers are scoped to their own projects.

**Auth**: Required — `project_manager` or `administrator`.

**Request**:
```json
{
  "comment_id": "string",
  "approved": true
}
```

**Response** `200`:
```json
{ "message": "Comment moderated" }
```

---

## 8. Fulfillment Verification

Fulfillment verification uses a three-checkpoint process (arrival → start → end) with OTP codes and QR codes. On completion, a SHA-256 service record hash is computed as an immutable proof of service.

### `POST /fulfillments`

Creates a new fulfillment record for a project.

**Auth**: Required — `project_manager` or `administrator`.

**Request**:
```json
{ "project_id": "string" }
```

**Response** `200`:
```json
{
  "id": "string",
  "project_id": "string",
  "arrival_at": null,
  "start_at": null,
  "end_at": null,
  "arrival_code": null,
  "start_code": null,
  "end_code": null,
  "is_complete": false,
  "service_record_hash": null,
  "created_at": "string (ISO 8601)"
}
```

---

### `GET /projects/{id}/fulfillments`

Lists all fulfillment records for a project.

**Auth**: Required — project owner.

**Response** `200`:
```json
[ { "...FulfillmentRecord" } ]
```

---

### `GET /fulfillments/{id}`

Returns a single fulfillment record.

**Auth**: Required — project owner.

**Response** `200`: `FulfillmentRecord`

---

### `POST /fulfillments/code`

Generates a 6-digit OTP code for a checkpoint. Codes expire after 10 minutes. Returns an SVG QR code.

**Auth**: Required — `project_manager` or `administrator`.

**Request**:
```json
{
  "fulfillment_id": "string",
  "checkpoint": "arrival | start | end"
}
```

**Response** `200`:
```json
{
  "code": "string (6 digits)",
  "expires_at": "string (ISO 8601)",
  "checkpoint": "string",
  "qr_code_svg": "string (SVG) | null"
}
```

---

### `POST /fulfillments/checkpoint`

Records a checkpoint by validating the OTP code. Enforces time-consistency (start within 2 hours of arrival; end after start). Computes SHA-256 service record hash on completion.

**Auth**: Required — project owner.

**Request**:
```json
{
  "fulfillment_id": "string",
  "checkpoint": "arrival | start | end",
  "code": "string"
}
```

**Response** `200`:
```json
{ "message": "Checkpoint recorded" }
```

---

### `GET /fulfillments/{id}/proof`

Returns a cryptographic proof of service for a completed fulfillment.

**Auth**: Required — project owner.

**Response** `200`:
```json
{
  "fulfillment_id": "string",
  "project_id": "string",
  "project_title": "string",
  "arrival_at": "string (ISO 8601)",
  "start_at": "string (ISO 8601)",
  "end_at": "string (ISO 8601)",
  "service_record_hash": "string (SHA-256 hex)",
  "generated_at": "string (ISO 8601)"
}
```

---

## 9. Event Tracking & Analytics

### `POST /events/track`

Tracks a user interaction event. Includes duplicate detection (3-second window) and suspicious burst detection. Fires webhooks.

**Auth**: Optional (anonymous events allowed).

**Request**:
```json
{
  "event_kind": "impression | click | dwell_time | session_start | session_end",
  "target_type": "string",
  "target_id": "string",
  "session_id": "string",
  "dwell_ms": 0,
  "metadata": "string (optional)"
}
```

**Response** `200`:
```json
{ "message": "Event tracked" }
```

---

### `GET /events/quality`

Returns data quality metrics for analytics events.

**Auth**: Required — `project_manager`, `finance_reviewer`, or `administrator`.

**Response** `200`:
```json
{
  "total_events": 0,
  "duplicate_events": 0,
  "suspicious_events": 0,
  "duplicate_rate": 0.0,
  "suspicious_rate": 0.0,
  "events_by_kind": [["impression", 42], ["click", 17]]
}
```

---

### `GET /events/suspicious`

Returns all events flagged as suspicious.

**Auth**: Required — `administrator` only.

**Response** `200`:
```json
[
  {
    "id": "string",
    "event_kind": "string",
    "target_type": "string",
    "target_id": "string",
    "session_id": "string",
    "user_id": "string | null",
    "dwell_ms": 0,
    "is_duplicate": false,
    "is_suspicious": true,
    "created_at": "string (ISO 8601)"
  }
]
```

---

## 10. Webhooks

### `POST /webhooks`

Registers a new webhook. URL must be a local/private network address. The server generates an HMAC-SHA256 signing secret.

**Auth**: Required — `administrator` only.

**Request**:
```json
{
  "name": "string",
  "url": "string (must be local/private: localhost, .local, 127.0.0.1, 10.x, 172.16-31.x, 192.168.x, ::1)",
  "event_types": ["donation.created", "event.tracked"]
}
```

**Response** `200`:
```json
{
  "id": "string",
  "name": "string",
  "url": "string",
  "secret": "string (HMAC signing key)",
  "event_types": ["string"],
  "enabled": true,
  "created_at": "string (ISO 8601)"
}
```

---

### `GET /webhooks`

Lists all registered webhooks.

**Auth**: Required — `administrator` only.

**Response** `200`:
```json
[ { "...WebhookConfig" } ]
```

---

### `DELETE /webhooks/{id}`

Deletes a webhook.

**Auth**: Required — `administrator` only.

**Response** `200`:
```json
{ "message": "Webhook deleted" }
```

---

### `GET /webhooks/{id}/deliveries`

Returns the last 50 delivery attempts for a webhook. Failed deliveries are retried up to 3 times with exponential backoff (1s, 2s, 4s).

**Auth**: Required — `administrator` only.

**Response** `200`:
```json
[
  {
    "id": "string",
    "webhook_id": "string",
    "event_type": "string",
    "payload_summary": "string",
    "attempt": 1,
    "status_code": 200,
    "success": true,
    "error_message": null,
    "created_at": "string (ISO 8601)"
  }
]
```

---

## 11. Finance Review

### `GET /finance/pending`

Lists all expenses awaiting review.

**Auth**: Required — `finance_reviewer` or `administrator`.

**Response** `200`:
```json
[ { "...ExpenseRecord" } ]
```

---

### `POST /finance/review`

Approves or rejects an expense disclosure.

**Auth**: Required — `finance_reviewer` or `administrator`.

**Request**:
```json
{
  "expense_id": "string",
  "approved": true,
  "note": "string (optional)"
}
```

**Response** `200`:
```json
{ "message": "Expense reviewed" }
```

---

## 12. Admin

### `GET /admin/stats`

Returns dashboard analytics with optional filters.

**Auth**: Required — `project_manager`, `finance_reviewer`, or `administrator`.

**Query Parameters**:
| Param | Type | Description |
|-------|------|-------------|
| `from` | string | Start date (ISO 8601) |
| `to` | string | End date (ISO 8601) |
| `cause` | string | Filter by cause |
| `status` | string | Filter by project status |

**Response** `200`:
```json
{
  "gmv_cents": 0,
  "total_donations": 0,
  "unique_donors": 0,
  "average_donation_cents": 0,
  "repeat_donor_rate": 0.0,
  "conversion_rate": 0.0
}
```

---

### `GET /admin/ops-log`

Returns the immutable audit log. Supports pagination.

**Auth**: Required — `administrator` only.

**Query Parameters**:
| Param | Type | Description |
|-------|------|-------------|
| `page` | integer | Page number |
| `per_page` | integer | Items per page |

**Response** `200`:
```json
[
  {
    "id": "string",
    "actor_id": "string",
    "actor_name": "string",
    "action": "string",
    "detail": "string",
    "created_at": "string (ISO 8601)"
  }
]
```

---

### `POST /admin/projects/{id}/unpublish`

Unpublishes a project. Uses two-step confirmation with password verification.

**Auth**: Required — `administrator` only.

**Request**:
```json
{
  "password": "string",
  "confirmation_token": "string (optional, omit for step 1)"
}
```

**Response**: Two-step confirmation flow.

---

### `POST /admin/assign-role`

Assigns a new role to a user. Requires password verification.

**Auth**: Required — `administrator` only.

**Request**:
```json
{
  "user_id": "string",
  "role": "supporter | project_manager | finance_reviewer | administrator",
  "password": "string"
}
```

**Response** `200`:
```json
{ "message": "Role assigned" }
```

---

### `POST /admin/bootstrap`

Promotes the first registered user to Administrator. Only works when no administrators exist in the system. One-time use.

**Auth**: Required (any user).

**Request**:
```json
{
  "password": "string",
  "confirmation_token": "string (optional)"
}
```

**Response** `200`:
```json
{ "message": "Admin bootstrapped" }
```

---

### `GET /admin/export/csv`

Exports donation data as CSV with PII masking. Supports optional date and cause filters.

**Auth**: Required — `finance_reviewer` or `administrator`.

**Query Parameters**: Same as `GET /admin/stats`.

**Response**: `text/csv` file download (`Content-Disposition: attachment; filename="donations.csv"`).
