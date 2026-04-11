# Fund Transparency — Business Logic Questions Log

## Authentication & Accounts

**Soft delete vs hard delete for users**
Question: The prompt did not specify whether deleting a user account is a physical delete or a logical delete.
My Understanding: Given the financial audit requirements, permanently deleting a user would break the integrity of donation records, expense approvals, and ops log entries that reference that user.
Solution: Implemented logical delete — users are never hard-deleted. Account deactivation is handled via role/status changes, and all historical records remain intact.

**Role hierarchy and permission scope**
Question: The prompt mentions roles (Donor, Project Manager, Finance Reviewer, Administrator) but does not clarify whether higher roles inherit lower role permissions or have completely separate permission sets.
My Understanding: Roles are distinct permission sets, not a strict hierarchy. A Finance Reviewer should not automatically have Project Manager capabilities, and vice versa.
Solution: Each role has an explicit permission set enforced at the route level via `require_role()`. Administrators have access to all routes. No implicit inheritance between roles.

**Session token expiry and renewal**
Question: The prompt did not specify how long sessions should last or whether tokens should be refreshed automatically.
My Understanding: For a financial transparency platform, sessions should expire to reduce the risk of token theft, but not so aggressively that it disrupts normal usage.
Solution: Tokens are valid for 24 hours from issuance. No automatic renewal — users must re-login after expiry. Token TTL is embedded in the HMAC payload and validated on every request.

## Donations & Refunds

**Inventory/budget rollback after refund**
Question: The prompt mentioned donors can request refunds, but did not specify how the project's raised total is updated when a refund is approved vs. when it is merely requested.
My Understanding: A refund request alone should not reduce the raised total — only an approved refund should. Otherwise, a flood of unreviewed refund requests could artificially deflate project funding displays.
Solution: Refunds are modeled as reversal donation records (`is_reversal = 1`). Unapproved reversals (`reversal_approved IS NULL` or `= 0`) are excluded from `raised_cents` aggregations. Only approved reversals affect the displayed total.

**Cross-project budget line donations**
Question: The prompt allows donors to direct donations to a specific budget line, but did not specify what happens if a donor submits a budget line ID that belongs to a different project.
My Understanding: This would be a data integrity error — a donation to Project A should not be attributed to a budget line from Project B.
Solution: When a donation specifies a `budget_line_id`, the backend validates that the budget line belongs to the target `project_id`. Cross-project references are rejected with `400 Bad Request`.

**Refund approval two-step confirmation**
Question: The prompt did not specify whether refund approval should require additional confirmation beyond standard authentication.
My Understanding: Refund approval is a financially sensitive and irreversible action. A single authenticated request is insufficient protection against CSRF or accidental clicks.
Solution: Refund approval uses a two-step confirmation protocol: Step 1 submits the password and receives a server-issued confirmation token. Step 2 resubmits with the token to execute the approval. Tokens are bound to the specific action and target and expire after a short TTL.

## Expenses & Receipts

**Receipt deduplication**
Question: The prompt did not specify what happens if the same receipt file is uploaded twice, either accidentally or intentionally.
My Understanding: Duplicate receipts would inflate expense records and undermine the transparency goal of the platform.
Solution: Each uploaded receipt is SHA-256 hashed before encryption. The fingerprint is stored in `receipts.sha256_fingerprint`. If an identical fingerprint already exists for the same expense, the upload is rejected with a duplicate error.

**Expense visibility before finance review**
Question: The prompt did not clarify whether expenses should be publicly visible immediately after a project manager submits them, or only after finance review.
My Understanding: Unreviewed expenses could contain errors or fraudulent claims. Showing them publicly before review would undermine the platform's credibility.
Solution: Expenses start in `pending` status and are only publicly visible after a Finance Reviewer approves them. Project managers can see their own pending expenses in the dashboard.

**Receipt encryption scope**
Question: The prompt mentioned receipt storage but did not specify whether receipts should be encrypted at rest.
My Understanding: Receipts may contain sensitive financial information (vendor names, amounts, account details). Storing them in plaintext would be a security risk.
Solution: Receipt file data and associated notes are encrypted using AES-256-GCM before storage. The encryption key is stored outside the repository tree and auto-generated on first run.

## Projects & Budget

**Project status transitions**
Question: The prompt described project states but did not define the allowed transitions between them (e.g., can a funded project be re-opened?).
My Understanding: Status transitions should follow a logical lifecycle. Allowing arbitrary transitions would create inconsistent data states.
Solution: Implemented a status machine: `draft` → `active` → `funded` → `closed`. Projects can also be `unpublished` from `active` by an administrator. Reverse transitions (e.g., `funded` → `active`) are not permitted.

**Budget line over-allocation**
Question: The prompt did not specify whether donations to a specific budget line should be capped at the budget line's allocated amount.
My Understanding: Allowing unlimited donations to a single budget line could result in over-funding one area while others remain unfunded, distorting the project's financial picture.
Solution: Budget line donations are tracked but not hard-capped at the allocation amount. The UI displays the allocation vs. received amounts so donors can make informed decisions. Hard caps were not implemented to avoid blocking legitimate donations.

## Comments & Moderation

**Comment deletion: physical or logical**
Question: The prompt did not specify whether deleting a comment is a hard delete or a soft delete.
My Understanding: Deleted comments that were part of a public discussion leave confusing gaps. However, keeping deleted content visible could expose harmful material.
Solution: Comments are soft-deleted — they are marked with a `deleted_at` timestamp and hidden from public views, but retained in the database for moderation audit purposes.

**Sensitive word filter behavior**
Question: The prompt mentioned a sensitive word filter but did not specify whether matching words should block submission entirely or just flag the comment for review.
My Understanding: A hard block would frustrate users who use flagged words in legitimate contexts. A flag-for-review approach is more flexible.
Solution: Two configurable policies: `block` (submission rejected until the word is removed) and `replace` (word is replaced with `[REDACTED]` and the comment is saved). The active policy is set by administrators via the moderation config.

## Notifications & DND

**Notification delivery during DND window**
Question: The prompt mentioned Do Not Disturb preferences but did not specify what happens to notifications generated during the DND window — are they discarded or deferred?
My Understanding: Discarding notifications during DND would cause users to miss important updates. Deferring them is the expected behavior for a DND feature.
Solution: Notifications created during a user's DND window are marked `is_deferred = 1`. A background process delivers deferred notifications when the DND window ends.

## Security & Replay Protection

**Nonce scope: per-user or global**
Question: The prompt required replay protection but did not specify whether nonces should be scoped per user or globally unique across all users.
My Understanding: A global nonce table is simpler and provides stronger replay protection — a nonce used by any user cannot be reused by any other user.
Solution: Nonces are stored in a global `nonces` table keyed by the nonce value. They expire after 5 minutes and are consumed (deleted) on first use. Replayed or expired nonces return `409 Conflict`.

**Webhook URL restriction scope**
Question: The prompt mentioned webhook support but did not specify any restrictions on webhook target URLs.
My Understanding: Allowing arbitrary webhook URLs would enable SSRF attacks where an admin-level user could probe internal services by registering a webhook pointing to internal network addresses.
Solution: Webhook URLs are validated against a blocklist of private/local network ranges (localhost, `127.x.x.x`, `10.x.x.x`, `172.16-31.x.x`, `192.168.x.x`). Attempts to register webhooks pointing to these addresses are rejected with `400 Bad Request`.

## Analytics & Fulfillment

**Analytics event deduplication**
Question: The prompt mentioned client-side analytics tracking but did not specify how to handle duplicate events (e.g., a user rapidly clicking the same button).
My Understanding: Without deduplication, analytics data would be inflated and unreliable for decision-making.
Solution: Events are deduplicated by `(session_id, event_type, target_id)` within a short time window. Suspicious burst patterns (many identical events in rapid succession) are flagged in the `analytics_events` table for review via the data quality metrics endpoint.

**Fulfillment checkpoint ordering**
Question: The prompt described a fulfillment verification flow with arrival, start, and end checkpoints but did not specify whether these must occur in strict order.
My Understanding: Out-of-order checkpoints (e.g., recording an end time before a start time) would produce invalid fulfillment records.
Solution: Checkpoint validation enforces ordering: `arrival` must precede `start` (within 2 hours), and `start` must precede `end`. Attempts to record checkpoints out of order are rejected with `400 Bad Request`.
