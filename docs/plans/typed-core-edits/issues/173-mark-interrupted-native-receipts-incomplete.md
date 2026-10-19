# 173: Mark interrupted native test binaries incomplete in receipts

Status: ready-for-agent
Type: build
Priority: P2
Blocked by: None (can start immediately)
Related: [169: Verification receipts](169-generate-scoped-verification-receipts.md); [Research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

Reviewers see incomplete native test evidence when a launched test binary never
finishes, even if every test name observed so far has passed. Preserve all actual
observations and the check's existing failure status; do not invent outcomes for
tests whose names were never printed.

The audit supplied output announcing 100 tests, one passing result, no terminal
binary summary and exit 130. The receipt marked the command complete with no
problems. The overall invocation correctly remained failed; the defect is its
misleading completeness metadata.

## Acceptance criteria

- [ ] First reproduce that interrupted-output case yielding complete evidence.
  After the fix it is incomplete and failed, retains the one observed pass and
  exit 130, and explains the missing summary/count mismatch.
- [ ] Track each launched native binary independently, require its terminal
  result summary, and reconcile announced executed-test count, named terminal
  outcomes and summary totals. Filtered-out tests are not executed tests.
- [ ] Missing, truncated, inconsistent or malformed binary output cannot become
  complete through another binary's summary or the command exit code. A launch
  interrupted before announcing tests also remains incomplete.
- [ ] Valid zero-test, filtered, ignored and multi-binary cases stay supported.
  A fully observed failing suite may be complete while remaining failed; keep
  terminal completeness distinct from pass/fail and baseline retention policy.
- [ ] Keep observed incomplete test names separate from any unnamed/unobserved
  shortfall. JSON and its derived reviewer summary agree, without fabricating
  missing test identities or changing known-exclusion and reused-evidence rules.
- [ ] Run affected receipt, native-output parser and check-runner tests plus a
  bounded invocation smoke check. Preserve default check output/exit behavior and
  the checked-in native name guard. Full browser/CAD rebuilds are unnecessary.

## Comments

Created from the user's audit findings after primary-source research. A focused
follow-up to resolved 169, which remains unchanged. Independent of the other four active
tickets; no replacement runner, external receipt service or new manifest scheme.
