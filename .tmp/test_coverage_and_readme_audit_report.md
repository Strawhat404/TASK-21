# 1. Test Coverage Audit

## Scope
- Method: static inspection only (no execution).
- Inspected evidence:
  - `repo/backend/src/main.rs`
  - `repo/backend/tests/api_extended.rs`
  - `repo/backend/tests/workflow_e2e.rs`
  - `repo/backend/tests/integration.rs`
  - `repo/backend/src/middleware.rs`
  - `repo/frontend/tests/*.rs`
  - `repo/frontend/src/main.rs`, `repo/frontend/src/lib.rs`, `repo/frontend/Cargo.toml`
  - `repo/run_tests.sh`

## Project Type Detection
- README top-level declaration: `Project type: fullstack` present (`repo/README.md:3`).
- Effective type: **fullstack**.

## Backend Endpoint Inventory
Source: `repo/backend/src/main.rs:62-137`.

1. `POST /api/auth/register`
2. `POST /api/auth/login`
3. `GET /api/auth/nonce`
4. `GET /api/projects`
5. `GET /api/projects/:id`
6. `GET /api/projects/:id/comments`
7. `POST /api/events/track`
8. `GET /api/auth/me`
9. `PUT /api/auth/dnd`
10. `POST /api/projects`
11. `POST /api/projects/updates`
12. `POST /api/projects/expenses`
13. `POST /api/projects/:id/favorite`
14. `POST /api/projects/:id/subscribe`
15. `POST /api/projects/:id/unsubscribe`
16. `POST /api/updates/:id/like`
17. `GET /api/favorites`
18. `GET /api/favorites/projects`
19. `GET /api/projects/:id/tickets`
20. `GET /api/projects/:id/expenses`
21. `GET /api/projects/:id/fulfillments`
22. `GET /api/fulfillments/:id`
23. `GET /api/fulfillments/:id/proof`
24. `GET /api/expenses/:id/receipts`
25. `POST /api/donations`
26. `GET /api/donations/mine`
27. `POST /api/donations/refund`
28. `POST /api/donations/refund/approve`
29. `GET /api/donations/refund/pending`
30. `POST /api/comments`
31. `POST /api/comments/:id/delete`
32. `POST /api/tickets`
33. `POST /api/tickets/respond`
34. `GET /api/notifications`
35. `POST /api/notifications/:id/read`
36. `POST /api/notifications/read-all`
37. `POST /api/receipts/upload`
38. `POST /api/receipts/review`
39. `GET /api/receipts/pending`
40. `GET /api/moderation/config`
41. `PUT /api/moderation/config`
42. `GET /api/moderation/comments/pending`
43. `POST /api/moderation/comments/review`
44. `POST /api/fulfillments`
45. `POST /api/fulfillments/code`
46. `POST /api/fulfillments/checkpoint`
47. `GET /api/events/quality`
48. `GET /api/events/suspicious`
49. `POST /api/webhooks`
50. `GET /api/webhooks`
51. `DELETE /api/webhooks/:id`
52. `GET /api/webhooks/:id/deliveries`
53. `GET /api/finance/pending`
54. `POST /api/finance/review`
55. `GET /api/admin/stats`
56. `GET /api/admin/ops-log`
57. `POST /api/admin/projects/:id/unpublish`
58. `GET /api/admin/export/csv`
59. `POST /api/admin/assign-role`
60. `POST /api/admin/bootstrap`

## API Test Classification
### 1) True No-Mock HTTP
- Present and dominant.
- Evidence: real router bootstrap + middleware + handlers, then HTTP request dispatch via `oneshot`.
  - `repo/backend/tests/api_extended.rs:37-222, 243-291`
  - `repo/backend/tests/workflow_e2e.rs:31-122, 135-175`
  - `repo/backend/tests/integration.rs:29-81`

### 2) HTTP with Mocking
- Not found.
- Mock/stub scan findings: no matches for `jest.mock|vi.mock|sinon.stub|mockImplementation|overrideProvider` in backend/frontend tests and runtime code.

### 3) Non-HTTP (unit/integration without HTTP)
- Present.
- Evidence: direct DB/auth/crypto/middleware tests in `repo/backend/tests/integration.rs` and `repo/backend/src/middleware.rs:171-217`.

## Mock Detection
- `jest.mock`: none found.
- `vi.mock`: none found.
- `sinon.stub`: none found.
- DI override patterns (`overrideProvider` etc.): none found.
- Direct controller/service bypass exists in non-HTTP tests (expected under category 3), e.g. `db::*` tests in `repo/backend/tests/integration.rs`.

## API Test Mapping Table
| Endpoint | Covered | Test type | Test files | Evidence |
|---|---|---|---|---|
| `POST /api/auth/register` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs` | `route_post_without_nonce_rejected` (`repo/backend/tests/integration.rs:325`), `route_register_duplicate_email_returns_409` (`repo/backend/tests/api_extended.rs:1639`) |
| `POST /api/auth/login` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_login_valid_credentials` (`repo/backend/tests/integration.rs:1709`), `flow_login_after_register_returns_valid_token` (`repo/backend/tests/workflow_e2e.rs:890`) |
| `GET /api/auth/nonce` | yes | true no-mock HTTP | all 3 backend test files | helper request in `get_nonce` (`integration.rs:103-110`, `api_extended.rs:226-240`, `workflow_e2e.rs:124-133`) |
| `GET /api/projects` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs`, `workflow_e2e.rs` | `route_list_projects_returns_created` (`integration.rs:1861`), `route_list_projects_filters_by_cause` (`api_extended.rs:338`) |
| `GET /api/projects/:id` | yes | true no-mock HTTP | `api_extended.rs` | `route_get_project_by_id_returns_detail` (`repo/backend/tests/api_extended.rs:308`) |
| `GET /api/projects/:id/comments` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_list_comments_for_project_returns_empty_when_none` (`api_extended.rs:401`), `flow_pre_moderation_approval_reveals_comment` (`workflow_e2e.rs:607`) |
| `POST /api/events/track` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs` | `route_track_event_anonymous` (`integration.rs:2088`), `route_track_event_flags_duplicate_message` (`api_extended.rs:1082`) |
| `GET /api/auth/me` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_auth_me_returns_user_profile` (`integration.rs:1671`) |
| `PUT /api/auth/dnd` | yes | true no-mock HTTP | `api_extended.rs` | `route_update_dnd_persists` (`api_extended.rs:785`) |
| `POST /api/projects` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_create_project_happy_path` (`integration.rs:1787`) |
| `POST /api/projects/updates` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_post_project_update_owner_notifies_subscribers` (`api_extended.rs:1122`) |
| `POST /api/projects/expenses` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs`, `workflow_e2e.rs` | `route_record_expense_cross_project_budget_line_rejected` (`api_extended.rs:1142`) |
| `POST /api/projects/:id/favorite` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_favorite_toggles_on_and_off` (`api_extended.rs:661`) |
| `POST /api/projects/:id/subscribe` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_subscribe_and_unsubscribe` (`api_extended.rs:718`) |
| `POST /api/projects/:id/unsubscribe` | yes | true no-mock HTTP | `api_extended.rs` | `route_subscribe_and_unsubscribe` (`api_extended.rs:718`) |
| `POST /api/updates/:id/like` | yes | true no-mock HTTP | `api_extended.rs` | `route_toggle_like_on_and_off` (`api_extended.rs:749`) |
| `GET /api/favorites` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_list_favorites_after_toggle` (`api_extended.rs:693`) |
| `GET /api/favorites/projects` | yes | true no-mock HTTP | `api_extended.rs` | `route_list_favorite_projects_returns_summaries` (`api_extended.rs:1748`) |
| `GET /api/projects/:id/tickets` | yes | true no-mock HTTP | `api_extended.rs` | `route_list_tickets_for_project_owner` (`api_extended.rs:1787`) |
| `GET /api/projects/:id/expenses` | yes | true no-mock HTTP | `integration.rs` | `route_expenses_requires_project_authz` (`integration.rs:402`) |
| `GET /api/projects/:id/fulfillments` | yes | true no-mock HTTP | `api_extended.rs` | `route_list_fulfillments_for_project_owner` (`api_extended.rs:1844`) |
| `GET /api/fulfillments/:id` | yes | true no-mock HTTP | `api_extended.rs` | `route_get_fulfillment_by_id_owner_succeeds` (`api_extended.rs:1877`) |
| `GET /api/fulfillments/:id/proof` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_service_proof_incomplete_returns_400` (`api_extended.rs:1425`), `flow_fulfillment_arrival_start_end_produces_proof` (`workflow_e2e.rs:530`) |
| `GET /api/expenses/:id/receipts` | yes | true no-mock HTTP | `api_extended.rs` | `route_list_receipts_finance_access` (`api_extended.rs:1318`) |
| `POST /api/donations` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs`, `workflow_e2e.rs` | `route_donation_happy_path` (`integration.rs:1893`) |
| `GET /api/donations/mine` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_my_donations_scoped_to_caller` (`api_extended.rs:917`) |
| `POST /api/donations/refund` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_request_refund_by_donor_creates_pending_reversal` (`api_extended.rs:954`) |
| `POST /api/donations/refund/approve` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_refund_approve_requires_confirmation_token` (`integration.rs:1489`) |
| `GET /api/donations/refund/pending` | yes | true no-mock HTTP | `api_extended.rs` | `route_pending_refunds_finance_admin_only` (`api_extended.rs:977`) |
| `POST /api/comments` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_post_comment_happy_path_appears_in_listing` (`api_extended.rs:414`) |
| `POST /api/comments/:id/delete` | yes | true no-mock HTTP | `integration.rs` | `route_comment_delete_requires_confirmation_token` (`integration.rs:1604`) |
| `POST /api/tickets` | yes | true no-mock HTTP | `api_extended.rs` | `route_submit_ticket_happy_path` (`api_extended.rs:519`) |
| `POST /api/tickets/respond` | yes | true no-mock HTTP | `api_extended.rs` | `route_respond_ticket_owner_succeeds_non_owner_rejected` (`api_extended.rs:539`) |
| `GET /api/notifications` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_list_notifications_scoped_to_user` (`api_extended.rs:590`) |
| `POST /api/notifications/:id/read` | yes | true no-mock HTTP | `api_extended.rs` | `route_mark_notification_read_flips_flag` (`api_extended.rs:608`) |
| `POST /api/notifications/read-all` | yes | true no-mock HTTP | `api_extended.rs` | `route_mark_all_read` (`api_extended.rs:631`) |
| `POST /api/receipts/upload` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs`, `workflow_e2e.rs` | `route_receipt_upload_owner_succeeds` (`integration.rs:1022`) |
| `POST /api/receipts/review` | yes | true no-mock HTTP | `api_extended.rs` | `route_review_receipt_verified_happy_path` (`api_extended.rs:1297`) |
| `GET /api/receipts/pending` | yes | true no-mock HTTP | `api_extended.rs` | `route_pending_receipts_finance_only` (`api_extended.rs:1922`) |
| `GET /api/moderation/config` | yes | true no-mock HTTP | `api_extended.rs` | `route_moderation_config_admin_only` (`api_extended.rs:819`) |
| `PUT /api/moderation/config` | yes | true no-mock HTTP | `api_extended.rs`, `workflow_e2e.rs` | `route_moderation_config_update_persists` (`api_extended.rs:830`) |
| `GET /api/moderation/comments/pending` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_moderation_pending_scoped_for_pm` (`integration.rs:1109`) |
| `POST /api/moderation/comments/review` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_moderation_pm_scoped_to_own_project` (`integration.rs:1058`) |
| `POST /api/fulfillments` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs`, `workflow_e2e.rs` | `route_create_fulfillment_happy_path` (`integration.rs:1962`) |
| `POST /api/fulfillments/code` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs`, `workflow_e2e.rs` | `route_generate_checkpoint_code` (`integration.rs:1993`) |
| `POST /api/fulfillments/checkpoint` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs`, `workflow_e2e.rs` | `route_record_checkpoint_with_valid_code` (`integration.rs:2030`) |
| `GET /api/events/quality` | yes | true no-mock HTTP | `api_extended.rs` | `route_events_quality_accepts_pm_fin_admin` (`api_extended.rs:1043`) |
| `GET /api/events/suspicious` | yes | true no-mock HTTP | `api_extended.rs` | `route_events_suspicious_admin_only` (`api_extended.rs:1066`) |
| `POST /api/webhooks` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs`, `workflow_e2e.rs` | `route_webhook_create_and_list` (`integration.rs:2117`) |
| `GET /api/webhooks` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_webhook_create_and_list` (`integration.rs:2117`) |
| `DELETE /api/webhooks/:id` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_webhook_delete` (`integration.rs:2168`) |
| `GET /api/webhooks/:id/deliveries` | yes | true no-mock HTTP | `api_extended.rs` | `route_webhook_deliveries_admin_only` (`api_extended.rs:1955`) |
| `GET /api/finance/pending` | yes | true no-mock HTTP | `workflow_e2e.rs` | `flow_finance_reviews_expense_and_exports_csv` (`workflow_e2e.rs:468`) |
| `POST /api/finance/review` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_finance_review_changes_expense_status` (`integration.rs:2202`) |
| `GET /api/admin/stats` | yes | true no-mock HTTP | `api_extended.rs` | `route_admin_stats_denies_supporter` (`api_extended.rs:864`) |
| `GET /api/admin/ops-log` | yes | true no-mock HTTP | `api_extended.rs` | `route_admin_ops_log_admin_only` (`api_extended.rs:894`) |
| `POST /api/admin/projects/:id/unpublish` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs`, `workflow_e2e.rs` | `route_sensitive_action_requires_two_steps` (`integration.rs:1381`) |
| `GET /api/admin/export/csv` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_csv_export_returns_csv_content_type` (`integration.rs:2279`) |
| `POST /api/admin/assign-role` | yes | true no-mock HTTP | `integration.rs`, `api_extended.rs`, `workflow_e2e.rs` | `route_assign_role_by_admin` (`integration.rs:890`) |
| `POST /api/admin/bootstrap` | yes | true no-mock HTTP | `integration.rs`, `workflow_e2e.rs` | `route_bootstrap_promotes_first_user` (`integration.rs:832`) |

## Coverage Summary
- Total endpoints: **60**
- Endpoints with HTTP tests: **60**
- Endpoints with TRUE no-mock HTTP tests: **60**
- HTTP coverage: **100.0%**
- True API coverage: **100.0%**

## Unit Test Summary
### Backend Unit Tests
- Test files:
  - `repo/backend/tests/integration.rs`
  - `repo/backend/src/middleware.rs` (`#[cfg(test)]`)
- Modules covered:
  - Controllers/routes: broad route-level checks across all route groups.
  - Services/business logic: role/ownership checks, refund flow, moderation checks, nonce behavior.
  - Repository/DB: direct `db::*` tests (donations, receipts, fulfillments, stats, confirmations, ops log).
  - Auth/guards/middleware: `auth`, nonce middleware, rate limiter, auth extraction.
- Important backend modules not tested: none obvious at route level (all route endpoints are covered by HTTP tests).

### Frontend Unit Tests (STRICT REQUIREMENT)
- Frontend test files found:
  - `repo/frontend/tests/ui_logic.rs`
  - `repo/frontend/tests/common_types.rs`
  - `repo/frontend/tests/dto_serde_extra.rs`
  - `repo/frontend/tests/component_render.rs`
- Framework/tools detected:
  - `wasm-bindgen-test` (`repo/frontend/Cargo.toml:22-23`)
- Actual frontend module/component imports + render evidence:
  - `BudgetBar`, `ProjectCard`, `ReceiptDisplay`, `HomePage` imported and mounted in browser DOM tests (`repo/frontend/tests/component_render.rs:14-18, 23-35`).
- Components/modules covered:
  - `components/budget_bar.rs` (render + class/width checks)
  - `components/project_card.rs` (links, badges, amounts)
  - `components/receipt.rs` (receipt fields/format/links)
  - `pages/home.rs` (hero/features/CTA rendering)
- Important frontend components/modules not tested:
  - `components/nav.rs`, `components/route_guard.rs`, `components/notification_center.rs`, `components/comment_section.rs`
  - pages: `login`, `project_list`, `project_detail`, `donate`, `profile`, `dashboard`, `admin`, `finance`, `staff`, `fulfillment`

**Mandatory verdict: Frontend unit tests: PRESENT**

### Cross-Layer Observation
- Backend testing is significantly deeper than frontend component/page testing.
- Frontend tests now exist and are valid, but breadth across UI surfaces is still limited.

## API Observability Check
- Strong overall:
  - endpoint path explicit in requests,
  - explicit JSON request bodies,
  - response assertions usually include status + payload fields.
- Weak spots:
  - a subset of tests assert status only without deep payload assertions.

## Tests Check
- Success paths: covered.
- Failure paths: covered broadly (authz/authn/nonce/replay/validation).
- Edge cases: present (dedup, moderation modes, webhook URL validation, two-step confirmations).
- Validation/auth/permissions: strong.
- Integration boundaries: strong for backend (HTTP+middleware+DB in same request path).
- Over-mocking risk: low (no mock/stub usage detected).
- `run_tests.sh` evaluation:
  - Not Docker-only; prefers local `cargo`/`wasm-pack` and Python tooling first (`repo/run_tests.sh:34-70, 111`).
  - Docker fallback exists but is partial (`repo/run_tests.sh:41-49, 74-82`).
  - Verdict on this check: **FLAG (local dependency present)**.

## End-to-End Expectations (fullstack)
- Expected: real FE↔BE E2E tests.
- Static evidence: backend HTTP E2E-style flows are present; browser-driven cross-layer E2E is not evident.
- Compensating factors: strong API coverage + frontend component render tests.

## Test Coverage Score (0–100)
- **91/100**

## Score Rationale
- + 100% endpoint HTTP coverage with true no-mock request path execution.
- + strong negative-path and security-path testing.
- + frontend unit tests now present with real component rendering.
- - frontend coverage still narrow relative to total page/component surface.
- - no explicit browser-to-backend end-to-end flow suite in evidence.
- - `run_tests.sh` not strictly container-only.

## Key Gaps
1. Frontend test breadth is limited relative to full UI surface (many pages/components untested).
2. No explicit browser FE↔BE E2E suite found.
3. Test runner still depends on local toolchain paths before Docker fallback.

## Confidence & Assumptions
- Confidence: high.
- Assumptions:
  - Endpoint source of truth is `repo/backend/src/main.rs`.
  - Coverage requires direct HTTP request evidence to exact method+path.

## Test Coverage Verdict
- **PASS (with gaps)**

---

# 2. README Audit

## README Location
- Required: `repo/README.md`
- Status: **FOUND**

## Hard Gate Checks
### Formatting
- PASS (clean, structured markdown).

### Startup Instructions (backend/fullstack)
- PASS.
- Required command `docker-compose up` appears exactly (`repo/README.md:60-63`).

### Access Method
- PASS.
- URL + ports provided (`repo/README.md:67-68`).

### Verification Method
- PASS.
- Includes API and UI verification steps (`repo/README.md:70-92`).

### Environment Rules (STRICT)
- PASS (no npm/pip/apt/runtime-install/manual-DB setup instructions).

### Demo Credentials (Conditional)
- **FAIL**.
- Auth exists (login/register and role guards in code: `repo/backend/src/main.rs:62-64,73-74`; `repo/frontend/src/main.rs:59-105`).
- README provides explicit credentials for only 2 roles (Administrator, Project Manager) (`repo/README.md:118-123`), not all roles.
- Missing explicit credential entries for Supporter and Finance Reviewer.

## High Priority Issues
1. Hard-gate failure: demo credentials do not enumerate username/email + password for **all roles**.

## Medium Priority Issues
1. Credential setup section is complex and partially shell-composed, increasing operator error risk (`repo/README.md:98-114`).

## Low Priority Issues
1. None significant under strict gate focus.

## Hard Gate Failures
1. Demo credentials for all roles missing (Supporter, Finance Reviewer not explicitly listed with credentials).

## README Verdict
- **FAIL**

---

## Final Combined Verdicts
- Test Coverage Audit: **PASS (with gaps)**
- README Audit: **FAIL**
