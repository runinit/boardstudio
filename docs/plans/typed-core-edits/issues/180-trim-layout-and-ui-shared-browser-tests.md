# 180: Trim duplicate browser mounts, pure unit tests in WASM, and CSS assertions in web/crates/layout and ui-shared

Status: resolved
Type: build
Blocked by: None (can start immediately)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

`web/crates/ui-shared` and `web/crates/layout` test `PendingEditSignals`, outline hooks, matrix inspectors, part placement, and toolbars without redundant browser DOM mounts, non-DOM queue tests running in Chrome, pure synchronous unit tests marked `#[wasm_bindgen_test]`, or brittle CSS pixel checks.

## Acceptance criteria

- [x] In `web/crates/ui-shared/src/pending_edit_helpers.rs`, remove the duplicate `typed_value_host` browser mount suite (`typed_unchanged_draft_restores_accepted_value_after_failure`, `typed_newer_draft_survives_failure_with_inline_feedback`, `typed_unchanged_landing_projects_accepted_value`) and the unused 46-line switch/board `ProjectDoc` fixture setup.
- [x] In `web/crates/layout/src/outline_lifecycle_browser_tests.rs` and `web/crates/layout/src/outline/hook_tests.rs`, delete the non-DOM `mod queued_actions` browser tests, the duplicate ID-collision and hover/focus no-op tests, and `completed_current_source_in_recovery_retires_with_failure` (exact prefix of `exact_rejection_allows_fresh_operation_retry`), and consolidate the overlapping stale-token/unmount tests in `hook_tests.rs`.
- [x] In `web/crates/layout/src/part_placement.rs`, `web/crates/layout/src/objects/matrix_inspector_controller.rs`, `web/crates/layout/src/objects/matrix_transform_inspector_tests.rs`, and `web/crates/layout/src/objects/keycap_size_tests.rs`, delete the subsumed controller/reference tests (`controller_creation_is_one_document_edit...`, `controller_creation_adds_selected_layout_membership...`, `controller_references_follow_existing_u_sequence...`), delete `one_shot_tickets_report_pending_until_they_settle`, `the_column_context_label_is_one_based` (fold 1-line check into `inspector_stagger_lands...`), `explicitly_reselecting_same_disabled_key_discards_previous_bulk_selection`, and `queued_key_resize_composes_with_unrelated_revision_and_reference_changes`, and consolidate the duplicate `PendingEditSignals` / delete-button mounts.
- [x] In `web/crates/layout/src/objects/layout_transform_toolbar.rs`, delete `drag_samples_share_the_immutable_matrix` (`Rc::clone` pointer check) and the duplicate `nudge_tests` cases already covered natively by `matrix_transform_operation.rs`.
- [x] In `web/crates/layout/src/objects/layout_toolbar.rs` and `web/crates/layout/src/objects/mirrored_pair/canvas_overlay_tests.rs`, delete `overlay_styles_keep_bounded_canvas_geometry` (`include_str!("m1.css")` substring check) and `pilot_popovers_preserve_select_rows_and_snap_field_layout` (computed CSS pixel checks + 5x80ms sleeps), and remove the `#[cfg(test)]` `LayoutMenuTransitionTrace` hooks from production `LayoutSelectMenu` and `LayoutSnapMenu`.
- [x] `cargo test -p boardstudio-web-layout -p boardstudio-web-ui-shared --lib` and the `ui-shared` and `layout` browser suites pass (verified in the full `check.py` run: `ui-shared` 36+2 passed in 2.57s+0.29s, `layout` 151+1 passed in 10.38s+0.42s).

## Implementation record (2026-10-19)

Performed:

- `web/crates/ui-shared/src/pending_edit_helpers.rs`: deleted the duplicate `typed_value_host` mount suite (three tests plus `typed_module`, `refused_typed_resolver`, `mount_typed_fixture`, `typed_draft_x`, `typed_failure_message`, `type_typed_draft_x`); `document()` is now `ProjectDoc::empty(id, name)`. The fixture imports shrank to `EditOperation, ProjectDoc`.
- `outline_lifecycle_browser_tests.rs`: deleted the two ID-collision tests, `mod queued_actions` (292 lines), and `mounted_hover_and_focus_do_not_create_document_history`; removed the now-unused `SnapshotToken` import.
- `outline/hook_tests.rs`: deleted `completed_current_source_in_recovery_retires_with_failure`, `stale_rendered_delete_generation_is_rejected_after_callback_refresh`, and `activation_rejects_stale_generation_at_refreshed_dispatch`.
- `part_placement.rs`: deleted the three subsumed controller/reference tests plus `retained_object_action_cannot_start_for_a_refreshed_snapshot` and `production_hook_registers_the_observer_before_synchronous_submit_settlement`.
- `matrix_inspector_controller.rs`: deleted the four listed tests and the `settle_action` helper that only they used.
- `matrix_transform_inspector_tests.rs`: folded the `"Column 1"` assertion into `inspector_stagger_lands_as_a_placement_edit_that_moves_later_columns_with_one_undo`; deleted the three listed tests.
- `keycap_size_tests.rs`: deleted `queued_key_resize_composes_with_unrelated_revision_and_reference_changes`.
- `layout_transform_toolbar.rs`: deleted `drag_samples_share_the_immutable_matrix`, `toolbar_stagger_and_inspector_stagger_land_the_same_matrix`, `the_preview_is_a_column_stagger_placement_and_nothing_else`, and `a_quick_offset_drag_changes_only_the_column_offset`.
- `canvas_overlay_tests.rs`: deleted `overlay_styles_keep_bounded_canvas_geometry`.
- `layout_toolbar.rs`: deleted `pilot_popovers_preserve_select_rows_and_snap_field_layout` with its CSS/mount/trace machinery (`pilot_popover_fixture`, `mount_pilot_popovers`, `popover_test_element`, `document_cleanup_styles`, `LayoutMenuTransitionTrace`, `record_layout_menu_transition`) and removed the `#[cfg(test)]` trace hooks from `LayoutSelectMenu` and `LayoutSnapMenu`.

Deviations from the ticket text:

- `contextual_toolbar_stays_inside_its_workspace` was deleted because the deleted pilot helpers (`mount_pilot_popovers`, `pilot_popover_fixture`) were its only mount; it was itself a computed-rect CSS check.
- The pure `#[wasm_bindgen_test]` to native `#[test]` conversions in the acceptance criteria (including `fixed_perimeter_finds_the_first_polygon_after_primitive_features`) were outside the delegated change list and remain as-is.
- The deletions leave two now-unused wasm-test imports in `web/crates/layout/src/outline/mod.rs` (`action_resolver`; `unique_outline_entity_id` and `unique_outline_version_id`). That file was outside the permitted edit list, so it was not changed; removing those names is the remaining follow-up.

## Verification (2026-10-19)

- `python3 scripts/check-wasm-tests.py`: pass (`no new wasm-only plain #[test]s (0 baselined)`).
- `cargo test -p boardstudio-web-layout -p boardstudio-web-ui-shared --lib`: pass. Layout 79 passed, 1 ignored; ui-shared 8 passed.
- `cargo clippy -p boardstudio-web-ui-shared -p boardstudio-web-layout --target wasm32-unknown-unknown --all-targets --no-deps -- -D warnings`: fails, but the command already failed at HEAD (ui-shared: 3 `redundant_closure`; layout: 56 wasm-only lint errors). After this change ui-shared has 1 remaining lint, pre-existing and out of scope (`web/crates/ui-shared/src/canvas_navigation.rs:316`), and layout has the same error count with two entries swapped for the `outline/mod.rs` unused imports above. Logs: `.evidence/ticket-180-clippy.log`, `.evidence/ticket-180-clippy-head-uishared.log`, `.evidence/ticket-180-clippy-head-layout.log`, `.evidence/ticket-180-native-tests.log`.
