# 01: Cache-branch readiness

Status: resolved
Type: research
Parent: [CI wall time map](../map.md)

## Question

Is `ci/improve-cache-speed` landable as this effort's cache fix, and what warm-wall gain does it
predict against the `d650e1ddf` cold baseline (web 33m37s, native 30m19s)?

## Pointers

- Branch diff versus `dev`: PR-scoped rust-cache saves, `RUSTUP_HOME` pinning,
  `cad/monstertruck-qualification` target coverage, `ImageOS` env-vars, PR-run cancellation,
  Chrome-for-Testing cache, native pinned to `ubuntu-24.04`.
- Read-only audit (lives on that branch, not on `dev`):
  `git show ci/improve-cache-speed:docs/research/2026-10-28-ci-build-time-audit.md`.
- Current workflow: `.github/workflows/check.yaml` on `dev` (save-if main/dev only).
- Constraints: never let PR code become a trusted cache writer; preserve provenance; respect the
  10 GB cache quota.

## Answer

Do **not** land `ci/improve-cache-speed` as-is: it is 41 commits / ~473 files of
whole-stack product code on top of `dev`, not a cache fix. Land **with changes** — extract only
the five cache commits (`2874034e2` stable key plus qualification target, `281b7a286` per-PR
cancellation, `8e647283a` same-repo PR-scope saves, `b5b964a80` dependabot exclusion, `3505f294b`
audit doc) as a standalone CI-only change; `scripts/test-ci-workflow.py` hermetically pins the
save matrix, trust boundaries, key stability, and quota guards. Trust/quota risks are bounded
(PRs write only their own `refs/pull/N/merge` scope; forks/dependabot never save;
`cache-on-failure` stays off), with two caveats: landing day invalidates existing `dev` caches
(one cold transition run), and several concurrent heavy PRs could LRU-evict `dev` scopes under
the 10 GB quota.

Predicted warm PR wall: **~22–27 min** (~7–12 min saved vs the cold baseline), anchored on PR 7's
warm web 22m52s; the bottleneck may flip to native since no warm native run has ever been
observed. Ticket 03 must run a same-head cold/warm A/B with hit/miss bytes, scope proof, and a
quota inventory before scoring steady state.

Full evidence and the ticket-03 checklist: `docs/research/2026-10-29-cache-readiness.md` on
branch `research/cache-readiness` (pushed, no PR).

## Comments

- Claimed by the charting session for read-only research delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3A0afe12f3-6dd6-4c7f-8677-88abb3fe5409`
  (findings go to `docs/research/2026-10-29-cache-readiness.md` on branch
  `research/cache-readiness`, pushed, no PR).
