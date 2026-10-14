# 35: Cut version-literal and golden churn (audit phase 2, part 3)

Status: claimed
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

- [x] Baseline and transcript sizes before/after; a deliberate version bump now touches one constant in the fixtures that are not migration tests.
- [x] The full `check.py` before and after (this closes the audit batch); `release` not run.

## Verification

Full `check.py` (default steps) with the sandbox lifted.

## Outcome (D5 and D7 implemented; D6/D8 disposition)

- D5/D7 source and fixture changes were committed to `dev` as `84930356`.
- D5 replaces current-version test assertions with `CURRENT_VERSION`, keeps literals where a test
  deliberately exercises migration input, and removes version pins from the synthetic-outline
  and paired-rejoin fixtures. The part-offset persistence test now serializes a document and checks
  its actual `formatVersion`; it no longer asserts that the current version happens to equal 6.
- D6's `demo_projects` migration helper and the old demo-project baselines named by the audit are
  absent from this checkout, so there was no helper to remove. The D8 built-in-board baseline
  reduction is deferred under the user's instruction to remove built-in-board work from scope.
- D7 retains full bodies for the `open`, `commit`, `invalid-draft`, and `draft` Core replies and the
  first successful archive-unpack anchor. Other replies keep identifying fields and whole-reply
  digests. Native and WASM transcript parity passed.
- Transcript size: **1,143,772 bytes before; 108,125 bytes after** (90.5% smaller).
- Full baseline `python3 scripts/check.py` in the disk-backed pre-D5/D7 snapshot passed all steps
  in **696.5 s**: repo 0.4, tooling 3.8, lint 24.4, build 221.4, test 125.4, browser 321.2.
  Full after-change `python3 scripts/check.py` on `dev` also passed all steps in **932.2 s**:
  repo 0.4, tooling 50.0, lint 22.2, build 404.8, test 123.6, browser 331.3. The build and
  tooling times differ substantially, so these totals are recorded as separate observations, not
  a speed comparison. `release` was not run. The final D5 fixture/test refinements passed focused
  Core tests (1 + 1 + 7) and the two affected mounted layout tests; `cargo fmt --all --check`
  passed.

This ticket remains open: D8 is deliberately deferred and the dependent browser-maintenance tickets
37 and 38 have not completed. The full audit is not closed by this partial outcome.
