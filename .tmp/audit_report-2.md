# Delivery Acceptance and Project Architecture Audit (Static-Only)

## 1. Verdict
- **Overall conclusion: Partial Pass**
- The repository materially implements the requested offline community-giving portal across frontend/backend/data model and includes substantial security controls and tests. However, there are material gaps for accountability/security guarantees (notably CSV accounting inconsistency for refund approvals, server-side second-confirmation semantics, and non-enforced immutability of operations log) that prevent a full Pass.

## 2. Scope and Static Verification Boundary
- **Reviewed**: repository structure, README/docs, backend routes/middleware/auth/db/crypto, frontend pages/components/state/API, SQL schema, and static test suite.
- **Not reviewed**: runtime behavior in browser/server, network behavior on deployment environment, performance under load, OS-level key custody hardening.
- **Intentionally not executed**: project startup, tests, Docker, external integrations.
- **Manual verification required** for claims requiring runtime proof (UI rendering/interaction timing, actual rate-limit behavior under concurrent traffic, webhook delivery timing/backoff behavior in real network conditions).

## 3. Repository / Requirement Mapping Summary
- **Prompt core goal**: offline-first donation transparency portal with four roles (Supporter, Project Manager, Finance Reviewer, Admin), project donations/designated donations, disclosure workflows, moderation/risk controls, fulfillment proof, analytics instrumentation, and auditability.
- **Mapped implementation areas**:
  - Backend Axum route surface and middleware: `repo/backend/src/main.rs:42-116`, `repo/backend/src/middleware.rs:16-169`
  - Auth + roles + signed tokens: `repo/backend/src/auth.rs:32-84`
  - SQLite schema/domain logic: `repo/database/init.sql:1-209`, `repo/backend/src/db.rs`
  - Frontend Leptos pages/components for supporter/staff/admin/finance flows: `repo/frontend/src/pages/*.rs`, `repo/frontend/src/components/*.rs`
  - Tests (static review only): `repo/backend/tests/integration.rs`

## 4. Section-by-section Review

### 4.1 Hard Gates

#### 4.1.1 Documentation and static verifiability
- **Conclusion: Pass**
- **Rationale**: Setup/run/test/env/migration guidance exists and maps to actual workspace structure.
- **Evidence**: `repo/README.md:13-77`, `repo/README.md:79-103`, `repo/backend/src/main.rs:42-116`, `repo/frontend/src/main.rs:13-42`.

#### 4.1.2 Material deviation from Prompt
- **Conclusion: Partial Pass**
- **Rationale**: Core prompt intent is implemented; however, some required semantics are weakened (server-side sensitive-action second confirmation not enforced as a protocol, accounting export inconsistency, immutable log not technically immutable).
- **Evidence**: `repo/frontend/src/pages/finance.rs:223-239`, `repo/backend/src/routes/donations.rs:172-186`, `repo/backend/src/db.rs:1315-1344`, `repo/database/init.sql:125-132`.

### 4.2 Delivery Completeness

#### 4.2.1 Core explicit requirements coverage
- **Conclusion: Partial Pass**
- **Rationale**: Most explicit requirements are implemented (roles, search/filter, donation/designation, receipts, comments/favorites/tickets, notifications+DND, finance review statuses/reasons, fulfillment OTP/time rules, moderation toggles, analytics dedup/suspicion, webhook signing/retry/logging). Material gaps remain in strict accountability controls.
- **Evidence**:
  - Roles/auth: `repo/backend/src/main.rs:42-116`, `repo/backend/src/middleware.rs:140-169`
  - Search/filter ZIP/cause/status: `repo/backend/src/routes/projects.rs:14-32`, `repo/backend/src/db.rs:319-383`
  - Donation + designated budget line: `repo/backend/src/routes/donations.rs:22-45`, `repo/backend/src/db.rs:592-699`
  - Receipt upload constraints and dedup: `repo/backend/src/routes/receipts.rs:20-52`, `repo/backend/src/db.rs:894-952`
  - Finance verify/reject reason: `repo/backend/src/routes/finance.rs:22-62`
  - Fulfillment OTP/time constraints/proof: `repo/backend/src/routes/fulfillment.rs:31-145`, `repo/backend/src/db.rs:995-1083`
  - Analytics dedup (3s)/suspicious: `repo/backend/src/routes/events.rs:28-57`, `repo/backend/src/db.rs:1088-1141`
  - Webhook local-net signed retry/logging: `repo/backend/src/routes/webhooks.rs:18-84`, `repo/backend/src/db.rs:1384-1464`

#### 4.2.2 End-to-end 0->1 deliverable vs partial demo
- **Conclusion: Pass**
- **Rationale**: Multi-module frontend+backend+common+database with migrations/docs/tests resembles a real service, not a single-file mock.
- **Evidence**: `repo/README.md:5-12`, `repo/backend/src/main.rs:42-116`, `repo/frontend/src/pages/home.rs:13-114`, `repo/database/init.sql:1-209`.

### 4.3 Engineering and Architecture Quality

#### 4.3.1 Structure and module decomposition
- **Conclusion: Pass**
- **Rationale**: Clear separation across middleware/routes/db/auth/crypto and frontend pages/components/services/state.
- **Evidence**: `repo/backend/src/main.rs:42-116`, `repo/backend/src/lib.rs:1-14`, `repo/frontend/src/main.rs:13-42`.

#### 4.3.2 Maintainability/extensibility
- **Conclusion: Partial Pass**
- **Rationale**: Generally maintainable, but some critical controls rely on convention/UI-only behavior rather than hard guarantees.
- **Evidence**: UI-only second-step confirmations `repo/frontend/src/pages/admin.rs:28-33`, `repo/frontend/src/components/comment_section.rs:83-109`; backend accepts single request with password for sensitive actions `repo/backend/src/routes/admin.rs:71-85`, `repo/backend/src/routes/comments.rs:77-91`.

### 4.4 Engineering Details and Professionalism

#### 4.4.1 Error handling/logging/validation/API quality
- **Conclusion: Partial Pass**
- **Rationale**: Validation and typed errors are broadly present; ops logging is consistent; but key handling can panic and CORS is overly broad for sensitive service.
- **Evidence**: validation examples `repo/backend/src/routes/receipts.rs:27-47`, `repo/backend/src/routes/webhooks.rs:31-38`; key panic risk `repo/backend/src/crypto.rs:20-24`; permissive CORS `repo/backend/src/main.rs:35-38`.

#### 4.4.2 Product-like vs demo-only
- **Conclusion: Pass**
- **Rationale**: Includes domain entities, role-based back office, analytics, webhook/audit paths, and integration tests.
- **Evidence**: `repo/database/init.sql:1-209`, `repo/frontend/src/pages/staff.rs:15-259`, `repo/backend/tests/integration.rs:1-813`.

### 4.5 Prompt Understanding and Requirement Fit

#### 4.5.1 Business goal/scenario/constraints fit
- **Conclusion: Partial Pass**
- **Rationale**: Solution is centered on transparency/accountability workflows and offline local processing; however, strict prompt semantics are partially weakened by accountability/security implementation gaps.
- **Evidence**: offline local payment recording with reversal fields `repo/database/init.sql:69-83`; immediate public ledger implication weakened by export inconsistency `repo/backend/src/db.rs:1315-1344`.

### 4.6 Aesthetics (frontend/full-stack)

#### 4.6.1 Visual and interaction quality
- **Conclusion: Cannot Confirm Statistically**
- **Rationale**: CSS and component structure indicate deliberate hierarchy and interaction states, but rendering quality and visual coherence require runtime/browser verification.
- **Evidence**: `repo/frontend/static/style.css:1-713`, `repo/frontend/src/components/project_card.rs:8-43`, `repo/frontend/src/components/notification_center.rs:13-121`.
- **Manual verification required**: responsive behavior, actual spacing/contrast/readability in browser.

## 5. Issues / Suggestions (Severity-Rated)

### Blocker / High

1. **Severity: High**
- **Title**: CSV export can include unapproved reversal records
- **Conclusion**: Fail
- **Evidence**: export query lacks approval filter `repo/backend/src/db.rs:1315-1344`; approval-aware filters used elsewhere `repo/backend/src/db.rs:1203-1244`, `repo/backend/src/db.rs:521-522`.
- **Impact**: Public/accounting exports can misstate finalized financials by counting pending/unapproved refund adjustments.
- **Minimum actionable fix**: Apply the same reversal-approval predicate in export query (`is_reversal = 0 OR reversal_approved = 1`) and add test asserting pending reversals are excluded.

2. **Severity: High**
- **Title**: Sensitive-action “second confirmation” is not enforced server-side
- **Conclusion**: Partial Fail
- **Evidence**: backend sensitive actions accept one API call with password `repo/backend/src/routes/donations.rs:172-186`, `repo/backend/src/routes/admin.rs:71-85`, `repo/backend/src/routes/comments.rs:77-91`; second step exists only in UI state `repo/frontend/src/pages/finance.rs:223-239`, `repo/frontend/src/pages/admin.rs:28-33`, `repo/frontend/src/components/comment_section.rs:83-109`.
- **Impact**: Non-UI clients can bypass intended dual-intent safeguard; increases accidental or coerced high-risk actions.
- **Minimum actionable fix**: Introduce server-issued short-lived confirmation challenge/token tied to action payload; require token + password on commit call.

3. **Severity: High**
- **Title**: “Immutable operations log” is not technically immutable
- **Conclusion**: Partial Fail
- **Evidence**: plain table definition without immutability constraints `repo/database/init.sql:125-132`; writable insert helper `repo/backend/src/db.rs:281-287`; no trigger-based block on update/delete for ops_log in schema.
- **Impact**: Audit trail tamper resistance is weaker than prompt requirement; privileged DB writes can alter history.
- **Minimum actionable fix**: Add DB triggers forbidding UPDATE/DELETE on `ops_log`, optionally hash-chain entries and verify chain during audits.

### Medium

4. **Severity: Medium**
- **Title**: CORS policy is fully permissive
- **Conclusion**: Partial Fail
- **Evidence**: `allow_origin(Any)`, `allow_methods(Any)`, `allow_headers(Any)` in `repo/backend/src/main.rs:35-38`.
- **Impact**: Expands attack surface for browser-based misuse in environments where host/network trust is imperfect.
- **Minimum actionable fix**: Restrict origins/methods/headers to configured allowlist per deployment profile.

5. **Severity: Medium**
- **Title**: Encryption key loading can panic on malformed key file
- **Conclusion**: Fail (reliability)
- **Evidence**: unchecked slice copy `repo/backend/src/crypto.rs:20-24`.
- **Impact**: Service crash/DoS at startup when key file exists but is shorter than 32 bytes.
- **Minimum actionable fix**: Validate key length and return typed error instead of panicking.

6. **Severity: Medium**
- **Title**: Static test suite misses critical regression checks for financial export/confirmation semantics
- **Conclusion**: Partial Fail
- **Evidence**: test suite present `repo/backend/tests/integration.rs:1-813` but no case validating export exclusion of unapproved reversals and no server-level dual-confirmation protocol test.
- **Impact**: Severe defects can pass CI undetected.
- **Minimum actionable fix**: Add integration tests for export accounting invariants and sensitive-action two-step server contract.

### Low

7. **Severity: Low**
- **Title**: Top-level metadata appears inconsistent with delivered project domain
- **Conclusion**: Partial Fail (documentation consistency)
- **Evidence**: `metadata.json:1-2` conflicts with repo scope in `repo/README.md:1-13`.
- **Impact**: Reviewer confusion and reduced traceability in acceptance contexts.
- **Minimum actionable fix**: Update metadata to match actual project and stack.

## 6. Security Review Summary

- **Authentication entry points: Pass**
  - Signed token auth and login/register paths are implemented with password hashing and token verification.
  - Evidence: `repo/backend/src/routes/auth_routes.rs:20-117`, `repo/backend/src/auth.rs:32-84`.

- **Route-level authorization: Partial Pass**
  - Role guards are broadly applied via middleware helpers and per-route checks.
  - Evidence: `repo/backend/src/middleware.rs:140-169`, route uses in `repo/backend/src/routes/*.rs` (e.g., admin `repo/backend/src/routes/admin.rs:15-23`).
  - Gap: sensitive-action second confirmation not enforced server-side (see High issue #2).

- **Object-level authorization: Pass**
  - Ownership checks exist for donations, receipts, project-scoped actions, and notifications.
  - Evidence: `repo/backend/src/routes/donations.rs:125-129`, `repo/backend/src/routes/receipts.rs:53-63`, `repo/backend/src/db.rs:1151-1152`.

- **Function-level authorization: Partial Pass**
  - High-risk operations require roles/password re-entry.
  - Evidence: refund approval/unpublish/comment removal password checks `repo/backend/src/routes/donations.rs:172-186`, `repo/backend/src/routes/admin.rs:71-85`, `repo/backend/src/routes/comments.rs:77-91`.
  - Gap: no server-side two-phase confirmation contract.

- **Tenant / user data isolation: Pass (single-tenant app, user-scoped data paths)**
  - User-scoped queries and owner checks are implemented for private resources.
  - Evidence: notifications `repo/backend/src/db.rs:1151-1152`, receipt ownership checks `repo/backend/src/routes/receipts.rs:53-63`.

- **Admin / internal / debug endpoint protection: Partial Pass**
  - Admin/staff routes are role-guarded.
  - Evidence: admin routes in `repo/backend/src/routes/admin.rs:15-23`, moderation controls `repo/backend/src/routes/moderation.rs:14-46`.
  - Gap: permissive CORS and non-immutable audit log reduce defense depth.

## 7. Tests and Logging Review

- **Unit tests: Cannot Confirm Statistically (as a separate layer)**
  - No distinct unit-test module set found; tests are primarily integration-style.
  - Evidence: `repo/backend/tests/integration.rs:1-813`.

- **API / integration tests: Partial Pass**
  - Strong coverage for many authz/validation flows (401/403, replay nonce, receipt permissions, role assignment, webhook validation).
  - Evidence: `repo/backend/tests/integration.rs:33-813`.
  - Gaps: no direct tests for export reversal-approval invariants and server-side dual-confirmation semantics.

- **Logging categories / observability: Partial Pass**
  - Consistent operations logging exists for key actions.
  - Evidence: ops-log helper `repo/backend/src/db.rs:281-287`; calls across routes (e.g., `repo/backend/src/routes/donations.rs:53-60`, `repo/backend/src/routes/finance.rs:52-59`).
  - Gap: immutable guarantee not DB-enforced.

- **Sensitive-data leakage risk in logs / responses: Partial Pass**
  - Some masking exists (`masked_email`, masked donor in CSV).
  - Evidence: `repo/backend/src/routes/auth_routes.rs:136-145`, `repo/backend/src/db.rs:1337`.
  - Residual risk: broad logging surface needs periodic review; static audit found no explicit plaintext password logging.

## 8. Test Coverage Assessment (Static Audit)

### 8.1 Test Overview
- Unit/API tests exist mainly as backend integration tests using `tokio::test` and HTTP request flows.
- Test entry point: `repo/backend/tests/integration.rs:1-813`.
- Test command documentation exists.
- Evidence: `repo/README.md:69-74`, `repo/backend/tests/integration.rs:1-18`.

### 8.2 Coverage Mapping Table

| Requirement / Risk Point | Mapped Test Case(s) | Key Assertion / Fixture / Mock | Coverage Assessment | Gap | Minimum Test Addition |
|---|---|---|---|---|---|
| Unauthenticated access rejected | `repo/backend/tests/integration.rs:70-79` | expects 401 on protected route | sufficient | none | none |
| Replay protection via nonce | `repo/backend/tests/integration.rs:81-103` | second request with same nonce => 401 | sufficient | none | add expiry boundary test (near 5 min) |
| Role-based admin authorization | `repo/backend/tests/integration.rs:244-281` | non-admin forbidden to assign role | sufficient | none | none |
| Object-level auth on receipts | `repo/backend/tests/integration.rs:735-813` | other user cannot access receipt | sufficient | none | none |
| Moderation/project ownership boundaries | `repo/backend/tests/integration.rs:653-733` | ownership checks for toggles/actions | basically covered | cross-role edge cases | add PM/Admin mixed-role conflict cases |
| Webhook local-network validation | `repo/backend/tests/integration.rs:573-651` | rejects non-local callback URL | sufficient | none | add signed payload verification negative cases |
| Financial export excludes unapproved reversals | **No mapped test found** | N/A | missing | severe accounting risk undetected | add integration test creating pending reversal then export CSV and assert exclusion |
| Sensitive action server-side second confirmation protocol | **No mapped test found** | N/A | missing | severe intent-control risk undetected | add two-step server contract tests for refund/unpublish/comment removal |

### 8.3 Security Coverage Audit
- **Authentication**: **basically covered** (login and unauthorized access tests exist) — `repo/backend/tests/integration.rs:70-79`, `105-144`.
- **Route authorization**: **basically covered** (admin/staff restrictions tested) — `repo/backend/tests/integration.rs:244-281`.
- **Object-level authorization**: **sufficient for sampled critical path** (receipt ownership) — `repo/backend/tests/integration.rs:735-813`.
- **Tenant / data isolation**: **insufficient** (limited explicit multi-user isolation assertions outside select endpoints).
- **Admin / internal protection**: **basically covered**, but severe defects could still remain due to missing tests around confirmation protocol and immutable audit guarantees.

### 8.4 Final Coverage Judgment
- **Partial Pass**
- Major security/authn/authz paths are meaningfully tested, but critical accounting and sensitive-action safeguards are not explicitly covered; tests could still pass while severe accountability defects remain.

## 9. Final Notes
- This audit is static-only and evidence-based; runtime claims were not made.
- Most of the requested platform is present and coherent, but the High-severity issues above should be addressed before delivery acceptance.
