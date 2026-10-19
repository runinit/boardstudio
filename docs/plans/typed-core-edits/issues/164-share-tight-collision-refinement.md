# 164: Share tight collision refinement across repeated model placements

Status: resolved
Type: build
Priority: P2
Blocked by: None (can start immediately)
Related: [CAD 010](../../cad-backend-replacement/issues/010-production-cache-worker-browser.md), [CAD 014](../../cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md)

## What to build

An explicit tight collision check should refine each distinct vendor STEP asset
once for its import settings and reuse that analysis mesh for all its placements.
Each placed component must keep its own identity, transform and source witness.

The 2026-10-18 audit at `f955050c3` traced the Case loop awaiting a fresh import for
every delivered model row. Sixty identical switch placements therefore issue sixty
serial imports of the same bytes and settings before pair checks begin. This was
established from the production call path; whole-keyboard latency was not measured.

## Acceptance criteria

- [x] A regression with repeated placements fails because the current tight-check
  path imports the same source/settings more than once. Count preparation requests
  rather than asserting wall-clock timing.
- [x] Within one current check, reuse refinement by verified source digest and the
  complete import settings, including repair budget and target deflection. Different
  digests or settings never share a refined asset accidentally.
- [x] Preserve placement IDs, final transforms, face provenance and pair results;
  sharing geometry must not collapse distinct component occurrences.
- [x] Preserve existing cancellation, stale-snapshot, worker-generation and evidence
  recovery rules. Reuse cannot activate retired evidence or bypass the existing
  bounded retry/refusal policy.
- [x] A failed shared preparation settles affected placements with the existing
  explicit uncertainty/error behavior. It does not leave waiters pending or cause
  another import for every row in the same check.
- [x] A focused Case/Runtime test exercises explicit tight checking with repeated
  rows. Verify the real provider/worker boundary and existing default/tight behavior
  using the established focused browser checks.
- [x] Run affected lifecycle and collision tests. Apply the CAD closure review policy
  once for the completed slice and record preparation counts and validation limits.

## Comments

Created from audit finding 3 at the user's request. Reuse existing asset delivery
and evidence ownership mechanisms; this ticket does not introduce a new scheduler
or broaden collision qualification.

## Answer — 2026-10-18

Implemented in `ed7fe9d5f`: a private Runtime adapter shares tight preparation
by verified SHA and complete repair/deflection settings within the current
scope/token/revision/check batch. Each placement keeps its ID, transform and
source witness. A separate adapter preserves missing-evidence recovery ownership.

RED observed **2 preparations instead of 1** for repeated placements. GREEN:
five new Runtime regressions; Runtime collision browser 12 passed; Case collision
browser 3 passed; model delivery 25, lifecycle 5 and native CAD collision 10 passed.
Fresh real-provider page observed **21 default pairs, 21 tight pairs, 1 tight
preparation**. Source precision still correctly leaves the 0.01 mm request uncertain;
no latency benchmark or broader collision qualification is claimed.

Capacity remains 80 cached assets. Once an asset is evicted, its same-batch attempt
marker remains and another preparation is refused; a new explicit check permits
recovery. This intentionally preserves the existing bounded retry/refusal policy
and may leave later placements uncertain under cache pressure. Removing the marker
would violate that policy and the existing LRU-pressure regression.

Integrated verification at `ed7fe9d5f`: `python3 scripts/check.py` passed
**repo, tooling, lint, build, test and browser** in **1005.7 s**, including native
name guards and freshly built provider/page artifacts. Logs: `.evidence/tickets162-165/full-check.log`;
stream RED/GREEN and mutation receipts: `.evidence/tickets162-165/streams/`.

Claude Opus medium combined closure review: **Standards zero findings; Spec
zero blocking findings**. The initial low eviction concern was withdrawn after
reviewing the existing bounded retry policy and LRU regression. Its optional
error-wording suggestion remains outside this slice; the capacity behavior is
recorded in ticket 164. Reports: `.evidence/tickets162-165/opus-final-review-round1.md`
and `opus-final-review-round2.md`. Review used source inspection and parent test
evidence. Broader CAD migration and performance acceptance remain outside these
four tickets.
