# 04: Generate and inspect the committed case in workers

**What to build:** Case-setting edits on either fixture durably commit, then produce current exact CAD displayed by the existing 3D renderer.

**Blocked by:** 02

**Status:** Implementation complete; actual provider projection, both final fixtures and focused renderer checks pass; full teardown allocation accounting remains open.

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [x] Compare prepared inputs/readiness and meshes/body identities with independent React provider lifetimes.
- [x] Preview/exact/stale/blocked/failed/cancelled states remain distinct.
- [x] Supersession, board/project replacement and worker failure settle jobs without corrupting caches.
- [ ] Real renderer DPR/resize/context-loss/late-import/teardown gates pass on the integrated web package.

See [current acceptance evidence](../ACCEPTANCE.md#current-evidence--final-release-8f509433) for artifact scope and remaining gates. Checked items record bounded completed checks; ticket closure follows the task graph and full acceptance requirements.
