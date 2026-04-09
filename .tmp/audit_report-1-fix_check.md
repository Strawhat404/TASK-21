# Follow-up Issue Recheck (2026-04-09)

Scope: Static-only recheck of the exact issues you listed.

## Summary
- Items rechecked: 9
- Fixed: 9
- Not Fixed: 0

## Detailed Status

1) Prior conclusion: Partial Pass (core coverage had DND/schema inconsistencies)
- Status: **Fixed**
- Evidence:
  - Search/filter still implemented: `repo/frontend/src/pages/project_list.rs:13-74`, `repo/backend/src/db.rs:432-505`
  - Notification schema consistency now present (`is_deferred` in both runtime and SQL artifact): `repo/backend/src/db.rs:124-131`, `repo/database/init.sql:115-121`
  - DND now uses user-local time with timezone: `repo/backend/src/db.rs:1059-1068`

2) Prior conclusion: Partial Pass (duplicate schema authority + timezone assumptions)
- Status: **Fixed (for listed risk points)**
- Evidence:
  - Runtime schema includes timezone field: `repo/backend/src/db.rs:24-27`
  - SQL schema artifact also includes timezone field: `repo/database/init.sql:15-17`
  - Runtime and SQL notifications schema both include `is_deferred`: `repo/backend/src/db.rs:124-131`, `repo/database/init.sql:115-121`

3) Prior conclusion: Partial Pass (fit risk from UTC-only DND + stale schema)
- Status: **Fixed**
- Evidence:
  - DND uses stored user timezone and local conversion: `repo/backend/src/db.rs:1061-1068`, `repo/backend/src/db.rs:1082-1087`
  - User model now carries timezone: `repo/common/src/lib.rs:258-260`, `repo/common/src/lib.rs:517-521`

4) Issue: Security documentation said receipt at-rest encryption, but payload stored raw
- Status: **Fixed**
- Evidence:
  - README now states receipt file data encrypted with AES-256-GCM: `repo/README.md:47-49`
  - Upload path now encrypts file bytes before DB insert: `repo/backend/src/routes/receipts.rs:81-94`
  - Byte encryption helper exists: `repo/backend/src/crypto.rs:50-63`
  - DB insert receives encrypted bytes from route: `repo/backend/src/db.rs:1372-1386`

5) Issue: `database/init.sql` out of sync with runtime schema (`is_deferred`)
- Status: **Fixed**
- Evidence:
  - Runtime notifications table includes `is_deferred`: `repo/backend/src/db.rs:124-131`
  - `init.sql` notifications table includes `is_deferred`: `repo/database/init.sql:115-121`
  - Runtime write path uses `is_deferred`: `repo/backend/src/db.rs:1072-1075`

6) Issue: DND logic UTC-only with no user timezone model
- Status: **Fixed**
- Evidence:
  - Users table includes `timezone`: `repo/backend/src/db.rs:26`, `repo/database/init.sql:17`
  - DND update accepts timezone: `repo/backend/src/routes/auth_routes.rs:128-129`, `repo/backend/src/db.rs:370-379`
  - DND evaluation uses user-local time: `repo/backend/src/db.rs:1061-1068`, `repo/backend/src/db.rs:1082-1105`

7) Issue: Security-critical route-level coverage incomplete
- Status: **Fixed (for the previously named paths)**
- Evidence:
  - Route-level receipt IDOR tests added: `repo/backend/tests/integration.rs:980-1015`, `repo/backend/tests/integration.rs:1017-1051`
  - Route-level moderation scope tests added: `repo/backend/tests/integration.rs:1053-1102`, `repo/backend/tests/integration.rs:1104-1136`
  - Route-level webhook URL policy tests added: `repo/backend/tests/integration.rs:1138-1166`, `repo/backend/tests/integration.rs:1168-1196`

8) Issue: Anonymous rate-limit bucket shared globally
- Status: **Fixed**
- Evidence:
  - Anonymous rate-limit key now uses client IP headers (`X-Forwarded-For` / `X-Real-IP`) with `anon:<ip>` keying: `repo/backend/src/middleware.rs:56-74`

9) Issue: Route/object-level auth coverage concerns (tests helper-heavy)
- Status: **Fixed (for listed receipt/moderation/webhook concerns)**
- Evidence:
  - Direct route-level authz assertions now exist for these object/route controls: `repo/backend/tests/integration.rs:980-1196`

## Final Verdict
- Based on current static evidence, the previously listed findings are **resolved**.
