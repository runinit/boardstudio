# 04: Generate and inspect the committed case in workers

**What to build:** Case-setting edits on either fixture durably commit, then produce current exact CAD displayed by the existing 3D renderer.

**Blocked by:** 02

**Status:** CAD worker and renderer composition implemented; current fixture/browser geometry acceptance in progress

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [ ] Compare prepared inputs/readiness and meshes/body identities with independent React provider lifetimes.
- [ ] Preview/exact/stale/blocked/failed/cancelled states remain distinct.
- [ ] Supersession, board/project replacement and worker failure settle jobs without corrupting caches.
- [ ] Real renderer DPR/resize/context-loss/late-import/teardown gates pass on the integrated web package.
