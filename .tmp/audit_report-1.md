# Delivery Acceptance and Project Architecture Audit (Rerun)

## 1. Verdict
- Overall conclusion: **Partial Pass**

## 2. Scope and Static Verification Boundary
- Reviewed:
  - Documentation, manifests, workspace layout, backend entrypoint/middleware/routes, DB schema/query layer, shared DTOs, frontend pages/components/styles, and test sources.
  - Key files include `repo/README.md`, `repo/backend/src/main.rs`, `repo/backend/src/middleware.rs`, `repo/backend/src/routes/*.rs`, `repo/backend/src/db.rs`, `repo/database/init.sql`, `repo/frontend/src/**/*`, `repo/backend/tests/integration.rs`.
- Not reviewed:
  - Runtime behavior under real browser/server execution, external network behavior, and deployment environment specifics.
- Intentionally not executed:
  - Project startup, tests, Docker, external services.
- Manual verification required:
  - True runtime UX/behavior of DND delivery timing, webhook retries under network faults, CSV download behavior in browser, and visual rendering/accessibility.

## 3. Repository / Requirement Mapping Summary
- Prompt core goal mapped: offline-first nonprofit donation transparency portal with 4 roles (Supporter, Project Manager, Finance Reviewer, Administrator), donation + designated budget lines, disclosure/review, engagement, fulfillment proof, analytics instrumentation, and on-prem signed webhooks.
- Main implementation areas mapped:
  - Backend Axum API and auth/nonces/rate-limit middleware (`repo/backend/src/main.rs:40-138`, `repo/backend/src/middleware.rs:48-154`).
  - SQLite persistence and domain logic (`repo/backend/src/db.rs:15-1756`).
  - Frontend Leptos role-facing pages/components (`repo/frontend/src/pages/*.rs`, `repo/frontend/src/components/*.rs`).
  - Static tests in backend integration suite (`repo/backend/tests/integration.rs:1-973`).

## 4. Section-by-section Review

### 1. Hard Gates

#### 1.1 Documentation and static verifiability
- Conclusion: **Partial Pass**
- Rationale: Startup/build/test instructions and route surface are documented and mostly consistent with manifests/entrypoints; however security/schema documentation has inconsistencies.
- Evidence:
  - Run/build/test docs: `repo/README.md:102-141`
  - Entry wiring: `repo/backend/src/main.rs:40-117`
  - Workspace/manifests: `repo/Cargo.toml:1-3`, `repo/backend/Cargo.toml:1-31`, `repo/frontend/Cargo.toml:1-20`
  - Security claim mismatch (receipt encryption claim vs implementation): `repo/README.md:47-49`, `repo/backend/src/db.rs:1339-1344`
  - Schema artifact mismatch (`init.sql` vs runtime schema): `repo/database/init.sql:112-119`, `repo/backend/src/db.rs:123-131`, `repo/backend/src/db.rs:1064-1087`
- Manual verification note: Not required for this conclusion.

#### 1.2 Material deviation from Prompt
- Conclusion: **Pass**
- Rationale: Delivery remains centered on prompt business flows (projects, donations, disclosures, moderation, fulfillment, analytics, webhooks, role governance) with no major unrelated implementation.
- Evidence:
  - Role/security controls: `repo/backend/src/main.rs:53-116`
  - Donation/disclosure/refund flows: `repo/backend/src/routes/donations.rs:9-208`, `repo/backend/src/routes/finance.rs:9-60`
  - Fulfillment controls: `repo/backend/src/routes/fulfillment.rs:71-243`
  - Analytics/webhooks: `repo/backend/src/routes/events.rs:9-95`, `repo/backend/src/routes/webhooks.rs:96-174`

### 2. Delivery Completeness

#### 2.1 Core explicit requirements coverage
- Conclusion: **Partial Pass**
- Rationale: Most core requirements are implemented; residual gaps/inconsistencies are around DND semantics and schema/docs consistency.
- Evidence:
  - Search/filter by cause/status/ZIP: `repo/frontend/src/pages/project_list.rs:13-74`, backend filter SQL `repo/backend/src/db.rs:432-505`
  - Budget-vs-actual bars and designated donation: `repo/frontend/src/pages/project_detail.rs:95-112`, `repo/frontend/src/pages/donate.rs:101-114`, server validation `repo/backend/src/routes/donations.rs:49-58`
  - Pledge and printable receipt: `repo/frontend/src/components/receipt.rs:19-47`
  - Message center + read/unread + DND update endpoint: `repo/frontend/src/components/notification_center.rs:31-76`, `repo/backend/src/routes/auth_routes.rs:118-133`, `repo/backend/src/db.rs:1045-1118`
  - Finance receipt verify/reject reason required: `repo/backend/src/routes/receipts.rs:116-122`
  - Fulfillment timing constraints: `repo/backend/src/routes/fulfillment.rs:146-183`

#### 2.2 End-to-end deliverable vs partial/demo
- Conclusion: **Pass**
- Rationale: Multi-crate full-stack structure with backend/frontend/common plus documented run/build path and broad route/page set; not a single-file demo.
- Evidence:
  - Structure and crates: `repo/Cargo.toml:1-3`, `repo/README.md:9-15`
  - Backend + frontend entrypoints: `repo/backend/src/main.rs:12-138`, `repo/frontend/src/main.rs:13-60`
  - Docs present: `repo/README.md:1-163`

### 3. Engineering and Architecture Quality

#### 3.1 Structure and module decomposition
- Conclusion: **Pass**
- Rationale: Reasonable module decomposition by domain (routes, middleware, DB, crypto/auth, frontend pages/components) for this scale.
- Evidence:
  - Route modules: `repo/backend/src/routes/mod.rs:1-8`, route files in `repo/backend/src/routes/`
  - Central middleware: `repo/backend/src/middleware.rs:48-154`
  - Shared DTO layer: `repo/common/src/lib.rs:360-520`

#### 3.2 Maintainability/extensibility
- Conclusion: **Partial Pass**
- Rationale: Core logic is extendable, but duplicated schema authority (`db.rs` DDL vs `database/init.sql`) and a few policy assumptions (timezone) reduce maintainability confidence.
- Evidence:
  - Runtime schema in code: `repo/backend/src/db.rs:15-250`
  - Separate SQL schema artifact: `repo/database/init.sql:7-227`
  - DND depends on UTC and no user timezone field: `repo/backend/src/db.rs:1058-1060`, `repo/database/init.sql:13-14`

### 4. Engineering Details and Professionalism

#### 4.1 Error handling, logging, validation, API design
- Conclusion: **Partial Pass**
- Rationale: Good baseline validation and status handling exists; notable gaps are documentation-security mismatch and limited defensive coverage around some edge policies.
- Evidence:
  - Validation examples: file type/size/hash dedup `repo/backend/src/routes/receipts.rs:32-73`; budget-line ownership `repo/backend/src/routes/projects.rs:169-176`; refund affected-row handling `repo/backend/src/routes/donations.rs:185-191`
  - Ops logging present: `repo/backend/src/db.rs:280-287`; applied in sensitive routes `repo/backend/src/routes/admin.rs:92-93`, `repo/backend/src/routes/comments.rs:93-94`
  - Doc/security mismatch: `repo/README.md:47-49` vs `repo/backend/src/db.rs:1339-1344`

#### 4.2 Product-like organization vs demo
- Conclusion: **Pass**
- Rationale: Includes multi-role UI, admin/finance/staff workflows, audit log, moderation, analytics, and webhook administration consistent with a productized baseline.
- Evidence:
  - Admin/finance/staff pages: `repo/frontend/src/pages/admin.rs:52-145`, `repo/frontend/src/pages/finance.rs:40-314`, `repo/frontend/src/pages/staff.rs:10-434`
  - Ops log and CSV export: `repo/backend/src/routes/admin.rs:42-52`, `repo/backend/src/routes/admin.rs:196-217`

### 5. Prompt Understanding and Requirement Fit

#### 5.1 Business goal and implicit constraints fit
- Conclusion: **Partial Pass**
- Rationale: Major business flows are implemented and aligned; remaining fit risk is DND-hour interpretation (UTC vs user-local expectation) and stale schema artifact for static verifiability.
- Evidence:
  - DND implementation uses UTC clock: `repo/backend/src/db.rs:1058-1060`
  - User model stores start/end only, no timezone: `repo/database/init.sql:13-14`
  - Notification DTO/read state present: `repo/common/src/lib.rs:360-365`, `repo/backend/src/routes/notifications.rs:13-44`

### 6. Aesthetics (frontend)

#### 6.1 Visual/interaction design quality
- Conclusion: **Cannot Confirm Statistically**
- Rationale: Styles and interaction states are present in code, but actual rendering quality, layout integrity, and interaction polish require runtime/browser inspection.
- Evidence:
  - CSS structure + hover/print/responsive rules: `repo/frontend/static/style.css:3-272`
  - UI components with interactive states: `repo/frontend/src/components/notification_center.rs:60-65`, `repo/frontend/src/pages/project_list.rs:88-100`
- Manual verification note: Validate desktop/mobile render, spacing hierarchy, and interaction feedback in-browser.

## 5. Issues / Suggestions (Severity-Rated)

### Blocker / High

1. Severity: **High**
- Title: Security documentation claims receipt-at-rest encryption that is not implemented
- Conclusion: **Fail**
- Evidence: `repo/README.md:47-49`, `repo/backend/src/db.rs:1339-1344`
- Impact: Misstated security posture can mislead deployment/compliance decisions and audit sign-off.
- Minimum actionable fix: Either implement receipt payload encryption before DB insert (and decrypt on authorized retrieval) or correct README to reflect actual behavior.

### Medium

2. Severity: **Medium**
- Title: Schema source inconsistency between `database/init.sql` and runtime schema
- Conclusion: **Partial Fail**
- Evidence: `repo/database/init.sql:112-119`, `repo/backend/src/db.rs:123-131`, `repo/backend/src/db.rs:1064-1087`
- Impact: Static verifiers/manual DB bootstrapping from SQL script can diverge from runtime behavior, especially notification defer semantics.
- Minimum actionable fix: Make one schema source of truth; update `database/init.sql` to include `is_deferred` (and keep synchronized), or document/runtime-generate schema only and remove stale SQL artifact.

3. Severity: **Medium**
- Title: DND hour logic is UTC-based without user timezone model
- Conclusion: **Partial Fail**
- Evidence: `repo/backend/src/db.rs:1058-1060`, `repo/database/init.sql:13-14`
- Impact: “Do Not Disturb” windows can trigger at incorrect local hours for users.
- Minimum actionable fix: Add per-user timezone/offset and evaluate DND in user-local time (or explicitly enforce/document single server timezone policy).

4. Severity: **Medium**
- Title: Security-critical route coverage remains incomplete despite expanded tests
- Conclusion: **Partial Fail**
- Evidence:
  - Receipt IDOR test is helper-level, not route-level: `repo/backend/tests/integration.rs:689-707`
  - Moderation scoping test is helper-level: `repo/backend/tests/integration.rs:735-759`
  - Webhook URL policy test does not hit route path: `repo/backend/tests/integration.rs:227-238`
- Impact: Severe route-level authz regressions may remain undetected while tests still pass.
- Minimum actionable fix: Add route-level integration tests for 401/403/404 and positive paths on receipt upload, moderation review, and webhook creation URL validation.

### Low

5. Severity: **Low**
- Title: Anonymous rate-limit bucket is shared globally
- Conclusion: **Partial Fail**
- Evidence: `repo/backend/src/middleware.rs:55-59`
- Impact: One unauthenticated client can consume the shared anonymous quota and affect others.
- Minimum actionable fix: Key anonymous limits by client IP (or IP+UA) rather than a single static key.

## 6. Security Review Summary

- Authentication entry points: **Pass**
  - Evidence: auth routes + token validation `repo/backend/src/routes/auth_routes.rs:10-94`, `repo/backend/src/auth.rs:32-84`.
- Route-level authorization: **Pass**
  - Evidence: role checks across admin/finance/webhooks/moderation `repo/backend/src/routes/admin.rs:26-31`, `repo/backend/src/routes/finance.rs:13-25`, `repo/backend/src/routes/webhooks.rs:21-25`.
- Object-level authorization: **Partial Pass**
  - Evidence: ownership checks added in receipts/comments/fulfillment `repo/backend/src/routes/receipts.rs:75-80`, `repo/backend/src/routes/moderation.rs:92-98`, `repo/backend/src/routes/fulfillment.rs:127-132`.
  - Residual risk: limited route-level regression tests for these controls (`repo/backend/tests/integration.rs:689-759`).
- Function-level authorization: **Pass**
  - Evidence: `require_role` and `require_project_owner` usage `repo/backend/src/middleware.rs:132-154`, route call sites above.
- Tenant/user data isolation: **Partial Pass**
  - Evidence: user-scoped reads/writes for donations/notifications and ownership checks `repo/backend/src/routes/donations.rs:125-130`, `repo/backend/src/routes/notifications.rs:17-40`.
  - Residual risk: anonymous shared rate-limit bucket and partial authz route coverage.
- Admin/internal/debug protection: **Pass**
  - Evidence: admin-only for ops log/webhooks/mod config and password re-entry on sensitive actions `repo/backend/src/routes/admin.rs:42-90`, `repo/backend/src/routes/webhooks.rs:23-25`, `repo/backend/src/routes/comments.rs:74-88`.

## 7. Tests and Logging Review

- Unit tests: **Partial Pass**
  - Evidence: crypto/auth/nonce and DB helper validations in `repo/backend/tests/integration.rs:113-260`, `648-826`.
- API/integration tests: **Partial Pass**
  - Evidence: route-level tests exist for bootstrap/assign-role/unpublish `repo/backend/tests/integration.rs:830-973`; broad gaps remain for several high-risk route authz cases.
- Logging categories/observability: **Pass**
  - Evidence: immutable ops log appends across sensitive operations `repo/backend/src/db.rs:280-287`, `repo/backend/src/routes/admin.rs:92-93`, `repo/backend/src/routes/donations.rs:193-195`.
- Sensitive-data leakage risk in logs/responses: **Partial Pass**
  - Evidence: masked CSV export `repo/backend/src/db.rs:1292-1300`; masked email in registration log `repo/backend/src/routes/auth_routes.rs:60`, `135-141`.
  - Risk: README security claim overstates encryption for receipt payload storage.

## 8. Test Coverage Assessment (Static Audit)

### 8.1 Test Overview
- Unit/API tests exist: **Yes** (single backend integration test module containing both helper-level and route-level tests).
- Frameworks: Rust `#[test]` and `#[tokio::test]` with Axum `oneshot`.
- Test entry points: `repo/backend/tests/integration.rs:1-973`.
- Documented test command: `cargo test -p server` in `repo/README.md:133-137`.

### 8.2 Coverage Mapping Table

| Requirement / Risk Point | Mapped Test Case(s) | Key Assertion / Fixture / Mock | Coverage Assessment | Gap | Minimum Test Addition |
|---|---|---|---|---|---|
| Auth token integrity/expiry | `repo/backend/tests/integration.rs:127-152` | signature mismatch/expiry rejection assertions | sufficient | none | none |
| Nonce replay protection | `repo/backend/tests/integration.rs:165-184` | consume once then reject replay | sufficient | no route-level nonce error-path per major endpoint | add 1-2 route tests per mutating class for missing nonce/used nonce |
| Registration role lock | `repo/backend/tests/integration.rs:113-123` | created user role is supporter | basically covered | helper-level only | add route-level register payload with privileged role and assert supporter in response |
| Bootstrap/role assignment controls | `repo/backend/tests/integration.rs:830-947` | 200 for admin bootstrap/assign, 403 non-admin, 409 bootstrap blocked | sufficient | none major | none |
| Refund accounting semantics | `repo/backend/tests/integration.rs:242-260` | unapproved reversal excluded; then approved reflected | basically covered | no full refund request+approve route chain | add end-to-end refund flow route test |
| Receipt upload ownership (IDOR) | `repo/backend/tests/integration.rs:689-707` | `require_project_owner` helper pass/fail | insufficient | route path not tested | add `/api/receipts/upload` 403 test for non-owner manager |
| Moderation project scoping | `repo/backend/tests/integration.rs:735-759` | comment->project + ownership helper | insufficient | route path not tested | add `/api/moderation/comments/review` 403 test for non-owner PM |
| DND deferred notifications | `repo/backend/tests/integration.rs:763-775` | persisted and `is_deferred=true` assertion | basically covered | timezone semantics untested | add tests for timezone-aware behavior after model extension |
| Fulfillment time consistency | `repo/backend/tests/integration.rs:803-826` | timestamp parsing only | insufficient | does not assert route rejection for invalid ordering | add route-level checkpoint tests for start>2h and end<=start |
| Webhook local URL policy | `repo/backend/tests/integration.rs:227-238` | IP private-range assumptions only | insufficient | no route-level create webhook with public URL | add route tests for `400` on public URL and `200` on private URL |

### 8.3 Security Coverage Audit
- Authentication: **Basically covered** by token tests (`repo/backend/tests/integration.rs:127-152`), but lacks many malformed-header route tests.
- Route authorization: **Insufficient** for several sensitive endpoints; strong coverage exists for admin bootstrap/assign-role, but not for receipts/moderation/webhook URL policy at route level (`repo/backend/tests/integration.rs:830-947` vs `689-759`, `227-238`).
- Object-level authorization: **Insufficient** in tests (mostly helper checks, not endpoint checks).
- Tenant/data isolation: **Basically covered** for selected flows (refund ownership data assumptions) but still vulnerable to undetected route regressions.
- Admin/internal protection: **Basically covered** for bootstrap/assign-role/unpublish; other admin surfaces are not comprehensively route-tested.

### 8.4 Final Coverage Judgment
- **Partial Pass**
- Covered major risks: token integrity, nonce replay primitive, bootstrap role controls, and core accounting semantics.
- Remaining uncovered risks: route-level object-authorization and policy enforcement gaps mean tests could still pass while severe authorization defects remain.

## 9. Final Notes
- Rerun confirms multiple prior blocker/high findings were fixed (role bootstrap/assignment path, receipt ownership checks, budget-line integrity checks, moderation scoping improvements, and sensitive-action UX improvements).
- Remaining issues are now concentrated in security/documentation consistency and route-level test depth rather than core feature absence.
