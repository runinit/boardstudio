# 177: Prune duplicate and no-op Core unit tests in mirrored_half and mechanical::edit

Status: resolved
Type: build
Blocked by: None (can start immediately)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

Core's unit test suite exercises mirrored-half reconciliation, lifecycle transitions, inverse mapping, and mechanical edits without dead no-op tests, 22 repeated compaction-fixture validations, copy-pasted 200-line fixture builders, or inline `mechanical::edit` tests that duplicate the integration suite.

## Acceptance criteria

- [x] Remove the no-op test `repair_fresh_chain_has_nonempty_both_local_child_dependencies` (`core/src/mirrored_half/reconcile.rs`), which mutates `before` and asserts on `before` without calling `prepare_membership_transition` or `reconcile`.
- [x] Trim `repair_fresh_coherent_compaction_fixture` to its 11-line offset normalization (removing the 65 lines of repeated `before` assertions), and remove `repair_fresh_compaction_before_matches_approved_literals`, `repair_fresh_corrected_literal_control_compacts_and_repeats`, `consolidated_fresh_refuses_nonfinite_historical_constraint`, `independent_transition_accepts_inherited_local_child_provenance`, `consolidated_fresh_valid_identity_control`, `repair_fresh_refuses_surviving_linked_key_constraint`, `valid_pair_fixture_materializes_both_matrices_and_passes_validators`, and dead `if let Ok` blocks after `assert!(matches!(result, Err(...)))` in `core/src/mirrored_half/reconcile.rs`.
- [x] Extract a shared `membership_expansion_fixture()` helper in `core/src/mirrored_half/reconcile.rs` for the four expansion tests (`repair_fresh_refuses_unbound_planned_source_reference`, `repair_fresh_refuses_missing_planned_child_parent`, `repair_fresh_refuses_duplicate_planned_mirror_references`, `repair_fresh_final_addition_copies_nondefault_source_parameters`) and deduplicate the overlapping hosted-addition tests (`adding_a_hosted_module_uses_the_selected_module_and_connector_ids`, `hosted_add_circuit_replay_binds_all_selected_identity_maps`, `adding_an_embedded_circuit_uses_the_selected_namespaced_id_maps`).
- [x] Share the duplicated 209-line pair fixture builders between `core/src/mirrored_half/reconcile.rs` and `core/src/mirrored_half/lifecycle/tests.rs`, delete the subsumed `prepared_unlink_reuse_board_keeps_one_nonempty_closure_set_and_flipped_geometry`, `whole_matrix_source_deletion_commits_retirement_and_detaches_local_parts`, and `matrixless_connector_omit_keeps_module_and_clears_only_connector_reference` tests in `lifecycle/tests.rs`, and reuse `commit_matrix_step` in `parent_kernel_commits_local_source_edit_and_rejoin_through_real_lifecycle_context`.
- [x] Remove the redundant sweep tests in `core/src/mirrored_half/inverse/tests.rs` (`every_in_scope_variant_runs_source_and_local_requests_as_written_and_inverts_mirror_ones`, `the_ledger_names_this_slice_and_what_becomes_of_every_other_sampled_family`) and `core/src/mirrored_half/inverse/hosted_tests.rs` (`every_family_of_this_slice_has_a_ledger_entry_and_linked_requests_are_rewritten`).
- [x] Remove the 20 inline settings unit tests and duplicated helper block in `core/src/mechanical/edit.rs` that are already covered by `core/tests/mechanical_settings_edits.rs`, retaining the replacement and mirror-pair lifecycle unit tests in `core/src/mechanical/edit.rs`.
- [x] `cargo test -p boardstudio_core --lib` and `cargo clippy -p boardstudio_core --all-targets --no-deps -- -D warnings` pass.

## Implementation notes

- `cargo test -p boardstudio_core --lib`: **825 passed / 3 ignored before → 788 passed / 3 ignored after** (37 duplicate, no-op or subsumed tests removed).
- `cargo clippy -p boardstudio_core --lib --tests --no-deps -- -D warnings`: clean.
- Net **3,649 lines removed** across the touched files (`4,053` deleted, `404` inserted): `reconcile.rs` −1,610, `lifecycle/tests.rs` −660, `mechanical/edit.rs` −1,127, `inverse/tests.rs` −193, `inverse/hosted_tests.rs` −62. `reconcile/prepare/tests.rs` and `reconcile/outlines/tests.rs` needed no change (see below).
- `repair_fresh_coherent_compaction_fixture` was reduced to an 11-line helper instead of deleted: `membership_compaction_fixture()` already applies the diode offsets, but its `retained-second-key-offset` pose normalization is not otherwise present, so the minimal mutation was kept without the 65 lines of repeated assertions.
- Shared `membership_expansion_fixture()` now holds the 105-line expansion setup used by the four expansion tests.
- The seven `reconcile::tests` fixture builders are `pub(in crate::mirrored_half)`; `lifecycle/tests.rs` imports `fixture_document` and its 209-line duplicate block is gone.
- `parent_kernel_commits_local_source_edit_and_rejoin_through_real_lifecycle_context` now drives Local/expansion/Rejoin/repeat through `commit_matrix_step`.

### Stale ticket references (absent from this checkout)

These named tests/helpers do not exist in the current tree, so no deletion was possible:

- `reconcile/prepare/tests.rs`: `prepare_preview_matches_committed_reconcile_across_matrix_edit_families`, `prepare_preview_projects_each_mechanical_edit_family_without_mutating_live_state`, `prepare_entry_captures_two_matrix_baseline_and_projects_targeted_matrix_only`, `reconcile_keeps_linked_companions_aligned_across_supported_modes`.
- `inverse/tests.rs`: `the_ledger_names_this_slice_and_keeps_unrelated_operations_out_of_it` (the existing ledger sweep is `the_ledger_names_this_slice_and_what_becomes_of_every_other_sampled_family`, which was removed), `mirror_transform_and_offset_edits_match_source_reconcile_in_every_companion_mode` / `fixture_in_mode`.
- `inverse/hosted_tests.rs`: `every_family_of_this_slice_runs_source_and_local_requests_as_written_and_inverts_mirror_ones` (the existing sweep `every_family_of_this_slice_has_a_ledger_entry_and_linked_requests_are_rewritten` was removed).
- `reconcile/outlines/tests.rs`: `remove_outline_element_retires_mirrored_board_binding_and_pair_record` (and `remove_outline_element_retires_only_the_targeted_mirrored_outline_record`).
- `reconcile.rs`: `independent_transition_refuses_linked_child_under_local_parent`; the dead `if let Ok(output) = &result` block lived in `independent_transition_rejects_wrong_board_final_owner` and was removed there.
