# 35: Cut version-literal and golden churn (audit phase 2, part 3)

Status: ready-for-agent
Type: build
Blocked by: [34](34-test-speed-browser.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Audit: [test-audit.md](../audit/test-audit.md)

## Request

The audit's maintenance items, which do not save time but remove churn at every format bump (we have bumped the version four times
on this branch). User, 2026-10-10: implement them.

## What to do

- D5: replace the roughly 14 format-version literals with `CURRENT_VERSION` where the assertion is not about the specific version
  (`current_schema`, `migrate.rs` tests, `archive.rs`, `storage_lifetime_tests`, `demo_projects`); keep literals only in migration-step tests.
- D7: cut the boundary transcript from 1.1 MB to about 0.2 MB: full bodies for a small set of anchor replies, digests elsewhere. Native and
  wasm parity both pass; the re-bless procedure is documented in `core/tests/support/boundary.rs`.
- D8: drop the recorded `hardware`, `parts`, `definitions` and `boards` for the ten paired splits from the demo baseline (re-recorded three
  times, not frozen by the file's own header), keeping keys and matrices at 1e-9; replace them with the targeted assertions already in
  `included_keyboards`.
- D6: remove the v4-to-v5 migration helper inside `demo_projects`.
- Close the audit: update `test-audit.md` with a phase 2 results section (counts, times, what was removed or merged), set ticket 24 resolved.

## Acceptance

- [ ] Baseline and transcript sizes before/after; a deliberate version bump now touches one constant in the fixtures that are not migration tests.
- [ ] The full `check.py` before and after (this closes the audit batch); `release` not run.

## Verification

Full `check.py` (default steps) with the sandbox lifted.
