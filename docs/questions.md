# Project Task Questions & Decisions Record

This record tracks all questions, clarifications, and major architectural decisions made during the Fund Transparency project lifecycle.

## Initial Setup Questions (Phase 1)

**Q: Should the project structure follow a flattened layout or a modular sub-internal layout?**
**A**: A modular layout was chosen: `repo/backend/`, `repo/frontend/`, and `repo/database/` for better separation of concerns.

**Q: Where should shared logic and models (common crate) be placed?**
**A**: It was placed in `repo/common/` as a peer to avoid circular dependencies and ensure it remains accessible to both layers.

## Audit Remediation Decisions (Phase 2)

**Q: How should self-registration role assignment be addressed?**
**A**: Modified the `register` route to strictly ignore client-provided roles and default all new accounts to the `Supporter` role.

**Q: Should existing unencrypted reviewer notes be migrated to AES-256?**
**A**: (Awaiting decision) Currently, all new notes are encrypted. Existing notes should be migrated during the first deployment cycle.

**Q: What is the preferred TTL for the replay protection nonces?**
**A**: Five minutes was selected as a balance between security and occasional network latency in offline-first scenarios.

---
*Created by Antigravity AI.*
