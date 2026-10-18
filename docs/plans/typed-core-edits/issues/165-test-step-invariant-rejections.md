# 165: Test STEP invariant rejection with corrupt imports

Status: claimed
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

- [ ] Replace the duplicate happy-path claim with focused negative cases using
  private test access. Do not widen member or API visibility for the tests.
- [ ] Cover NaN and infinity beyond the first ten position entries, including a
  finite prefix, so truncating the scan by position cannot pass unnoticed.
- [ ] Cover an inconsistent triangle-to-face metadata count and the other existing
  invariant rejection branches with controlled corrupt values and explicit reasons.
  Keep assertions tied to the current contract; any newly discovered production
  defect is reported explicitly before broadening this test-only slice.
- [ ] Retain a valid-import control. Rename or rewrite the successful-import test
  so its stated guarantee matches what it actually observes.
- [ ] Establish a meaningful RED/GREEN result for the late nonfinite-coordinate
  regression. Use the project's mutation checker to demonstrate that bypassing the
  relevant invariant condition is detected, and restore source exactly afterward.
- [ ] Check that the production import boundary still invokes validation. Claim
  integration enforcement coverage only if a test actually exercises that boundary
  with a failing invariant; record the limit otherwise.
- [ ] Run the affected native invariant/import tests in release mode. For this
  test-only slice, retain RED/GREEN and mutation evidence plus a final diff read
  under the CAD review policy.

## Comments

Created from audit finding 4 at the user's request. This adds rejection evidence
for existing invariants and does not reopen the broader STEP validation plan.
