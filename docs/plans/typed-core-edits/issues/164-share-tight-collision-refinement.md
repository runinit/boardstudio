# 164: Share tight collision refinement across repeated model placements

Status: claimed
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

- [ ] A regression with repeated placements fails because the current tight-check
  path imports the same source/settings more than once. Count preparation requests
  rather than asserting wall-clock timing.
- [ ] Within one current check, reuse refinement by verified source digest and the
  complete import settings, including repair budget and target deflection. Different
  digests or settings never share a refined asset accidentally.
- [ ] Preserve placement IDs, final transforms, face provenance and pair results;
  sharing geometry must not collapse distinct component occurrences.
- [ ] Preserve existing cancellation, stale-snapshot, worker-generation and evidence
  recovery rules. Reuse cannot activate retired evidence or bypass the existing
  bounded retry/refusal policy.
- [ ] A failed shared preparation settles affected placements with the existing
  explicit uncertainty/error behavior. It does not leave waiters pending or cause
  another import for every row in the same check.
- [ ] A focused Case/Runtime test exercises explicit tight checking with repeated
  rows. Verify the real provider/worker boundary and existing default/tight behavior
  using the established focused browser checks.
- [ ] Run affected lifecycle and collision tests. Apply the CAD closure review policy
  once for the completed slice and record preparation counts and validation limits.

## Comments

Created from audit finding 3 at the user's request. Reuse existing asset delivery
and evidence ownership mechanisms; this ticket does not introduce a new scheduler
or broaden collision qualification.
