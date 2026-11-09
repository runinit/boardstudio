# 165: Test STEP invariant rejection with corrupt imports

Status: resolved
Type: build
Priority: P2
Blocked by: None (can start immediately)
Related: [STEP validation 03](../../step-validation-simplification/issues/03-post-import-invariants.md), [CAD 007](../../cad-backend-replacement/issues/007-strict-step-import.md)

## What to build

The post-import invariant suite must have tests that detect broken rejection
behavior. A successful tetrahedron import remains a smoke check; it must not be
presented as proof that corrupt imports are rejected or that the import-time guard
is enforced.

The 2026-10-18 audit at `f955050c3` found two new tests using only a valid
tetrahedron. Neither exercises rejection, including the recently corrected scan
for a nonfinite coordinate beyond the first ten positions.

## Acceptance criteria

- [x] Replace the duplicate happy-path claim with focused negative cases using
  private test access. Do not widen member or API visibility for the tests.
- [x] Cover NaN and infinity beyond the first ten position entries, including a
  finite prefix, so truncating the scan by position cannot pass unnoticed.
- [x] Cover an inconsistent triangle-to-face metadata count and the other existing
  invariant rejection branches with controlled corrupt values and explicit reasons.
  Keep assertions tied to the current contract; any newly discovered production
  defect is reported explicitly before broadening this test-only slice.
- [x] Retain a valid-import control. Rename or rewrite the successful-import test
  so its stated guarantee matches what it actually observes.
- [x] Establish a meaningful RED/GREEN result for the late nonfinite-coordinate
  regression. Use the project's mutation checker to demonstrate that bypassing the
  relevant invariant condition is detected, and restore source exactly afterward.
- [x] Check that the production import boundary still invokes validation. Claim
  integration enforcement coverage only if a test actually exercises that boundary
  with a failing invariant; record the limit otherwise.
- [x] Run the affected native invariant/import tests in release mode. For this
  test-only slice, retain RED/GREEN and mutation evidence plus a final diff read
  under the CAD review policy.

## Comments

Created from audit finding 4 at the user's request. This adds rejection evidence
for existing invariants and does not reopen the broader STEP validation plan.

## Answer — 2026-10-18

Implemented in `f46e09b9a`: eight private validator tests cover late
NaN/+Inf/-Inf at position 35 after a finite prefix, empty mesh, missing source faces,
inconsistent nonempty triangle-face tags, every nonfinite bounds coordinate, and
equal/reversed degenerate bounds. Kept the valid-import control and removed the
misleading duplicate happy-path test. No production behavior or visibility changed.

Project mutation checker baseline: **8 passed**. Truncating the coordinate scan
with `take(10)` compiled and was killed by all three late-coordinate tests (exit
101); exact source restoration was verified, then restored GREEN **8 passed**.
Affected STEP import tests passed in release mode. This is mutation RED, not an
unmodified-baseline failure. Source inspection confirms production still invokes
validation before success. The tests corrupt an already imported model and prove
validator rejection; they do not prove import-boundary rejection of injected
corruption. No injectable hook or broader acceptance claim was introduced.

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
