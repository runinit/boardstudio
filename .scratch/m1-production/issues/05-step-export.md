# 05: Export committed STEP with exact snapshot guards

**What to build:** Editors download a STEP case built from the durably accepted revision, independently of preview cache, with stale downloads prevented.

**Blocked by:** 04

**Status:** ready-for-agent

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [ ] Archive and STEP delivery require current immutable snapshot/session/board/instance identity.
- [ ] Reopened STEP matches existing bounds/material/volume and cached/uncached geometry oracles on both fixtures.
- [ ] Cancellation, changed scope, save recovery and worker failure prevent delivery and settle callers.
- [ ] Characterize CAD wide revisions and reject unsupported identity loss; complete integer parity requires explicit compatibility resolution.
