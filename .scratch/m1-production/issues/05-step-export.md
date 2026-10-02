# 05: Export committed STEP with exact snapshot guards

**What to build:** Editors download a STEP case built from the durably accepted revision, independently of preview cache, with stale downloads prevented.

**Blocked by:** 04

**Status:** Implementation complete; real STEP-only reply regression repaired and reviewed, both final downloads and stale-scope suppression pass; complete independent semantic/resource acceptance remains open.

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [x] Archive and STEP delivery require current immutable snapshot/session/board/instance identity.
- [ ] Reopened STEP matches existing bounds/material/volume and cached/uncached geometry oracles on both fixtures.
- [ ] Cancellation, changed scope, save recovery and worker failure prevent delivery and settle callers.
- [x] Characterize CAD wide revisions and reject unsupported identity loss; complete integer parity requires explicit compatibility resolution.

See [current acceptance evidence](../ACCEPTANCE.md#retained-evidence--release-8f509433-and-reviewed-overlays) for artifact scope and remaining gates. Checked items record bounded completed checks; ticket closure follows the task graph and full acceptance requirements.
