# Re-Inspection Fix Status (Rerun, Static-Only)

Date: 2026-04-09
Method: Fresh file reads from current repository state (not prior report cache).

## Summary
- Fixed: 7
- Remaining from listed defects: 0
- Informational/strength: 1

## Issue Status

1. CSV export includes unapproved reversal records
- Status: **Fixed**
- Evidence: `repo/backend/src/db.rs:1370-1372` (accounting filter now applied in export); test `repo/backend/tests/integration.rs:1287-1314`.

2. Sensitive-action second confirmation not enforced server-side
- Status: **Fixed**
- Evidence:
  - Refund approval two-step token: `repo/backend/src/routes/donations.rs:185-203`
  - Unpublish two-step token: `repo/backend/src/routes/admin.rs:84-100`
  - Comment delete two-step token: `repo/backend/src/routes/comments.rs:90-106`
  - Token persistence/consume: `repo/backend/src/db.rs:233-241`, `repo/backend/src/db.rs:305-332`

3. Immutable ops log not technically immutable
- Status: **Fixed**
- Evidence:
  - Trigger enforcement in schema init: `repo/backend/src/db.rs:260-270`
  - Trigger enforcement in SQL schema: `repo/database/init.sql:250-260`
  - Immutability test: `repo/backend/tests/integration.rs:1351-1376`

4. CORS policy fully permissive
- Status: **Fixed**
- Evidence: CORS allowlist + explicit methods/headers in `repo/backend/src/main.rs:38-56`.

5. Encryption key loading can panic on malformed key file
- Status: **Fixed**
- Evidence:
  - Key loader now returns `Result` and malformed key returns `Err`: `repo/backend/src/crypto.rs:15-33`
  - Startup handles error with explicit fatal message/exit path: `repo/backend/src/main.rs:25-28`
  - Tests assert error behavior (not panic): `repo/backend/tests/integration.rs:1440-1450`

6. Missing regression tests for export/confirmation semantics
- Status: **Fixed**
- Evidence:
  - Export regression test: `repo/backend/tests/integration.rs:1287-1314`
  - Confirmation token behavior tests: `repo/backend/tests/integration.rs:1316-1349`
  - Route-level unpublish two-step test: `repo/backend/tests/integration.rs:1378-1437`
  - Route-level refund two-step + token reuse tests: `repo/backend/tests/integration.rs:1486-1555`, `repo/backend/tests/integration.rs:1557-1595`
  - Route-level comment-delete two-step test: `repo/backend/tests/integration.rs:1601-1662`

7. Role guards broadly applied via middleware and route checks
- Status: **Confirmed (Informational)**
- Evidence: route guard usage remains broad across modules (`repo/backend/src/routes/*.rs`), e.g. `repo/backend/src/routes/admin.rs:66-69`, `repo/backend/src/routes/donations.rs:167-170`, `repo/backend/src/routes/comments.rs:72-75`.

## Final Conclusion
All defect items from your listed findings are now fixed in current static code, and the previous remaining encryption-key reliability defect has also been resolved in the latest code state.
