# Library mounted-test harness repair — 2026-10-04

## Red evidence

Input: retained `library-mounted-repair-20261004/red.log` and `library-mounted-repair-20261004/red.json`. The isolated run executed 11 tests: 6 passed, 5 failed. The three delete failures (`current_project_delete_cancel_and_confirm_use_the_observed_replacement_path`, `current_project_delete_sorts_remaining_records_by_stored_name`, `failed_current_project_replacement_keeps_the_record_and_dialog_available`) and `project_menu_name_draft_survives_unrelated_accepted_revision_and_commits_latest_document` all panic at `wait_for_saved_cards` with expected counts 1/3/2/4. The helper queried `.m1-keyboard-card`, which includes demo cards; `DemoKeyboardCard` explicitly renders `.m1-keyboard-card.m1-demo-keyboard-card`. This is a stale test selector, not a product assertion failure.

`project_menu_exposes_the_whole_project_copy_action` fails later because the name test panicked before removing `#project-name-mounted-regression`. `close_project_menu()` selects the first document-wide `.m1-project-menu`; with the leaked earlier test root, it closes that one and the copy test's menu stays open. This is cross-test DOM contamination after the helper failure, not evidence of a production defect in the single-menu UI.

## Repair

Only `web/src/presentation/library.rs` test-module selectors changed. `wait_for_saved_cards` now counts `.m1-keyboard-card:not(.m1-demo-keyboard-card)`. The name/search test's result counts and title lookups now use the same saved-only selector; its separate demo-action assertion remains intact. No production source or allowlist changed.

No compilation or rerun was performed (coordinator owns the test slot). The source diff is a harness correction only; the prior raw red remains the execution evidence until coordinator records a fresh isolated green run.

## Follow-up raw result and structural assertion repair

Coordinator's next actual module run is retained at `parts-library-gates-20261004/presentation_library.log`: 11 executed, 6 passed, 5 failed. The name-draft test advanced past the repaired card wait and now fails at the exact-child assertion (`unexpected direct menu child`, line 1897). Current `Library` markup always renders `.m1-project-delete-dialog` as the final direct child after `.m1-project-menu-footer`; it was omitted from the test's expected ordered children. Added that expected child, preserving the existing assertions for heading, New/Open actions, current-project section, scroll content, footer/setup guide, and ordering. The copy-action failure is downstream of the name test abort leaving its root mounted; subsequent saved-card waits likewise run after that abort and can observe contaminated mounted/storage fixtures. No product behavior failure is shown by this trace.

The current worktree source now contains both test-only repairs: saved-card selectors exclude demo cards, and the menu structure assertion includes its mounted delete dialog. No tests/build were run; coordinator owns the next isolated execution and any required fixture cleanup. The source was settled after `git diff --check`.
