---
title: 'Prove populated membership retirement and remove redundant pick block'
type: chore
ticket: 4
created: '2026-11-16'
status: built
baseline_revision: '5aa4358e002716e884a673892ad945774030023c'
route: oneshot
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The membership-cache regression computes empty lists and ends its owner-change cases on an empty board, so its close assertion does not prove retirement of populated membership. The layout-source pick test contains a redundant duplicate-reference block rejected by source identity before duplicate resolution.

**Approach:** Repair only the membership test's local fixture and retirement/reopen assertions; remove only the authorized redundant pick block. Keep notification reuse, owner-change assertions, unique-reference selection and direct duplicate rejection intact. Seed an accepted one-cell matrix using the existing Runtime/Core edit seam and an independently expected generated key id. After all owner-change cases, restore the populated board/instance scope and cache concrete eligible and live ids immediately before close. Capture that full model for reopen: scope, token, session epoch, revision, document allocation and scene allocation must remain identical. Close must clear both lists without invoking compute; reopening must invoke compute again and recover the expected ids.

Only ticket 1.4 is in scope. Preserve existing assertions except the explicitly authorized duplicate block removal. No production behavior or shared-fixture changes, ignored tests, helper sweep, changes to built stories 1.1–1.3, or work on 1.5/the harness epic. A normal repaired-test failure against current production blocks this story for a separate production investigation; do not fix production here. Temporarily bypass the close act to demonstrate failure, immediately restore it, rerun and record exact evidence. No temporary mutation may be committed. Preserve unpublished history and unrelated work; keep necessist.db unread, untouched and excluded from artifacts and commits. No branch/checkout changes, stash, push or PR.

</frozen-after-approval>

## Implementation Notes

Oneshot: approximately 90 changed lines across two existing native-test bodies, with no implementation delegation. Risk is low because delivered changes are test-local and the only deletion is expressly authorized by epic R4. User approved proceeding past the dirty-tree gate with only untracked necessist.db excluded.

Investigation: `web/crates/layout/src/matrix_transform_lifecycle.rs`, `selection_retention_tests::accepted_membership_reuses_preview_notifications_and_retires_on_source_change`, opens `matrix_fixture()` with no matrix. Its compute closure filters for nonexistent `key`; the owner-change sequence switches to `other-board` then changes instance, token, session, document/scene allocations and revision. Reuse the adjacent `core_disable_then_undo_restores_scoped_key_selection` test's one-cell `SetMatrix` pattern locally, without modifying `matrix_fixture`, `fixed_commit`, `eligible_ids`, `live_ids` or production cache logic. Expected generated id: `matrix/matrix-main/r0c0`. Clone the pre-close ReadModel to preserve all owner identities for reopen, instead of fetching the original runtime model.

`web/crates/runtime/src/layout_viewer_source.rs`, `tests::picks_require_full_current_scope_source_generation_and_unique_board_reference`: remove only its final duplicate-document block (currently lines 1013–1028); keep every preceding unique-pick and stale-owner assertion. `web/crates/runtime/src/case_preview.rs`, `native_preview_pick_tests::picked_renderer_reference_resolves_to_board_part_and_not_asset_or_mesh_id` already tests ambiguous references directly; leave it unchanged and run it.

Evidence (2026-11-16): seeded the test-local matrix through real Core acceptance, changed only the compute closure's key literal, restored main/None scope after all existing owner-change checks, asserted both cached lists equal `["matrix/matrix-main/r0c0"]` immediately before close, and cloned that exact ReadModel for identical-full-owner reopen. Close retains the panic-on-compute closure and the original empty-lists assertion; reopen now checks recomputed ids as well as the existing call-count assertion. All existing notification/source-change assertions remain. The runtime edit removes only the authorized 17-line duplicate block, retaining unique selection and stale-owner guards. No production or shared-fixture changes.

The initial plan write was refused twice by Gortex as `view_building`, including the required retry after 30 seconds; native patch creation used AGENTS.md's fallback. Subsequent plan edits, every source edit and both mutation/restoration writes succeeded through Gortex. Exact overlay graph routing subsequently recovered; change.detect mapped only the two intended test functions and their test modules. Graph test discovery returned no coverage targets, guards had no configured rules, and contract allowed the change; explicit repository test commands provide execution evidence.

Observed verification (commands below, logs under `.evidence/`):
- Normal membership: 1 passed, 0 failed, 0 ignored (`story-1-4-membership-normal.log`).
- Counter-example: replaced only `model.accepted = None;` with a bypass comment after cloning the pre-close owner. Same membership command exited 101, 0 passed, 1 failed, 0 ignored: `web/crates/layout/src/matrix_transform_lifecycle.rs:371:9: assertion failed: empty.eligible.is_empty() && empty.live.is_empty()` (`story-1-4-close-bypass.log`). Immediately restored the exact close line through Gortex; restored file blob matched its pre-mutation Git blob. Restored rerun: 1 passed, 0 failed, 0 ignored (`story-1-4-membership-restored.log`).
- Unique pick and direct duplicate rejection: 1 passed each, 0 failed/ignored (`story-1-4-unique-pick.log`, `story-1-4-direct-duplicate.log`).
- Neighbors: matrix_transform_lifecycle 4 passed; layout_viewer_source 12 passed; case_preview 15 passed; all 0 failed/ignored (`story-1-4-retention-neighbors.log`, `story-1-4-layout-source-neighbors.log`, `story-1-4-case-preview-neighbors.log`).
- Full project check exited 0: repo, tooling, build, lint, test, browser all passed, total 434.4s; final page/browser batch executed 263, failed 0 (`story-1-4-full-check.log`). No unexpected test failure occurred.
- Target resolved inside this worktree. All test/check invocations used the prescribed build lock and existing build settings. Git diff reviewed before normal, counter-example and restored tests; changes are restricted to the two intended test bodies and this plan. No counter-example remains; necessist.db is unread and untouched.

## Plan Change Log

## Review Triage Log

2026-11-16 — Quick inspection review completed on the story-only diff and plan using the same model/reasoning level (GPT-6.1 Sol, high). Result: no findings; high/medium/low/false/maybe-false counts all 0. No patches, deferrals or unresolved items. Production code and shared fixtures remain unchanged. Final Git scope is the two test files and this plan; necessist.db remains unread, untouched and excluded.

## Verification

Run serially, rechecking `readlink -f target` and using the prescribed build lock if shared. Preserve supported runner/concurrency settings. Long output goes only to `.evidence/` logs.

- `cargo test -p boardstudio-web-layout --lib accepted_membership_reuses_preview_notifications_and_retires_on_source_change` — normal pass; temporary bypass of `model.accepted = None` must fail the unchanged empty-membership assertion; restore immediately and rerun to pass.
- `cargo test -p boardstudio-web-runtime --lib picks_require_full_current_scope_source_generation_and_unique_board_reference` — unique-reference selection and retained guards pass after block removal.
- `cargo test -p boardstudio-web-runtime --lib picked_renderer_reference_resolves_to_board_part_and_not_asset_or_mesh_id` — direct duplicate rejection passes unchanged.
- `cargo test -p boardstudio-web-layout --lib matrix_transform_lifecycle` — neighboring retention tests pass, none ignored.
- `cargo test -p boardstudio-web-runtime --lib layout_viewer_source` and `cargo test -p boardstudio-web-runtime --lib case_preview` — neighboring source/preview tests pass, none ignored.
- `python3 scripts/check.py` — all full-project steps pass; quote any failures without inventing their causes.
- Inspect Git diff before tests and after restoration: only the two test bodies and this plan may change; production code and shared fixtures remain unchanged. Record commands, observed counts, mutation failure, restored result and review outcome here. Commit only this story's files if the rendered workflow directs it.
