# 07: Delete the unused BrowserStore save paths

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** Browser storage has one real project save path: the expected-token save that
Runtime uses, with conflict detection. Two further entry points, `save_document_with_assets` and
the test-harness `abort_document_save_for_test`, have no callers anywhere in the tree, including
tests, scripts and fixtures. The second one is also the only reason for the `force_abort` flag
threaded through the private save and persist functions. Delete both methods and the flag, so
anyone reading the most data-critical storage file sees only the save paths that actually run.

Behaviour does not change: conflict detection, missing-asset aborts, asset verification and
active-project bookkeeping stay exactly as they are.

## Acceptance criteria

- [x] `save_document_with_assets`, `abort_document_save_for_test` and the `force_abort` parameter are gone, and a whole-tree search finds no remaining reference.
- [x] The remaining save, conflict and missing-asset behaviour is unchanged; the existing storage lifetime browser tests pass unchanged.
- [x] `python3 scripts/check.py quick` passes, including the host crate's browser tests.

## Answer

Commit `2fbea117c` removes the two unused save entry points and the test-only abort flag, without changing the expected-token save, conflict and missing-asset handling, asset verification or active-project bookkeeping. Whole-tree search found no remaining references. The 20 storage-lifetime browser tests and all 45 host browser tests passed; `python3 scripts/check.py quick` passed every selected step. The combined Standards/Spec review found no implementation defect; its missing-tracker finding is closed by this separate ticket/map commit. Release, security and deny were not run for this slice.
