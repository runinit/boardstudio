# 17b: Validation and Layout ownership

Status: resolved
Type: build
Blocked by: [17a](17a-grouped-model-and-core-callers.md)
Parent: [17](17-cut-over-format-and-model.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Validate the grouped records and enforce Layout ownership, without silently accepting unresolved anchors.

## Acceptance

- Apply finite/default/list-bound checks, sparse defaults, negative/zero step policy, growth/truncation and nonempty logical Layout ownership. Preserve unrelated data.
- No silent acceptance of an unresolved anchor as world origin.

## Checks

Source anchors: `core/src/validate.rs`, `model.rs`, `matrix.rs`. Use focused public Core tests (document-integrity module) and `check.py quick --files`.

## Progress

- [x] Finite/default/list-bound, sparse defaults, growth/truncation, step policy: `grouped_layout_validation_refuses_bad_lists_and_unresolved_anchors_atomically`, `zero_and_negative_steps_are_accepted_while_axis_zero_steps_are_refused`, `explicit_steps_survive_pitch_edits_and_compaction_drops_only_removed_records`, `non_finite_evaluated_layouts_and_empty_ownership_are_refused`, `an_anchor_without_follow_follows_all_three_components`.
- [x] Nonempty logical Layout ownership, unrelated data preserved: `non_finite_evaluated_layouts_and_empty_ownership_are_refused`, `edits_that_orphan_an_anchor_or_empty_a_layout_are_refused_atomically`, `omitted_matrix_membership_keeps_a_live_anchor_without_materialized_parts`.
- [x] No unresolved anchor as world origin: `anchors_without_any_layout_are_refused_not_read_as_world_origin`, `anchors_must_resolve_to_an_in_range_key_in_the_same_layout`, `anchor_keys_stay_free_of_constraints_and_self_anchors_are_refused`.
- [x] Dimension bound (`MAX_MATRIX_PARTS` = 4096 rows x columns, `core/src/matrix.rs`) runs before any row/column walk: `valid_dimensions` is called first by `matrix::layout::validate`. `validate.rs` now calls `valid_dimensions` (same finding text, `Matrix dimensions or transform are invalid`). `kernel.rs` keeps its own `usize` check on already-parsed `LayoutInput` (comment added); `valid_dimensions` stays `pub(crate)` because `validate.rs` uses it.
- [x] Dimension refusals pin the exact message `Matrix dimensions are invalid or exceed the part limit` and unchanged state: `a_matrix_of_u32_max_rows_is_refused_on_open_before_any_row_walk`, `a_matrix_of_u32_max_columns_is_refused_on_open_before_any_column_walk`, `a_matrix_one_over_the_part_limit_is_refused_on_open`, `oversized_matrices_are_refused_through_set_matrix_and_project_matrices_with_state_unchanged` (u32::MAX rows, u32::MAX columns, 4097 via `set-matrix` and `project-matrices`). The capped run found no unbounded path through `ProjectMatrices`/`SetMatrix` (no hang, immediate refusal). `project-matrices` refuses with the kernel's `invalid matrix dimensions`, which the boundary golden transcript (`3:invalid-draft`) pins, so it was left as is and asserted as such.
- Every other 17b refusal test now asserts the specific refusal text (`assert_refused`).
- RED-first: `rows = u32::MAX` on open (red4 kill and capped 120 s timeout). Characterization (passed on pre-existing behaviour): the 4097 case, u32::MAX columns, `cells_beyond_the_matrix_dimensions_are_refused_by_set_matrix` (`Matrix cells exceed dimensions`), and the anchor/growth/ownership tests.
- New tests in `core/tests/document_integrity.rs` (not added to any baseline JSON): `zero_and_negative_steps_are_accepted_while_axis_zero_steps_are_refused`, `anchors_must_resolve_to_an_in_range_key_in_the_same_layout`, `anchors_without_any_layout_are_refused_not_read_as_world_origin`, `non_finite_evaluated_layouts_and_empty_ownership_are_refused`, `edits_that_orphan_an_anchor_or_empty_a_layout_are_refused_atomically`, `an_anchor_without_follow_follows_all_three_components`, `anchor_keys_stay_free_of_constraints_and_self_anchors_are_refused`, `a_matrix_of_u32_max_rows_is_refused_on_open_before_any_row_walk`, `a_matrix_of_u32_max_columns_is_refused_on_open_before_any_column_walk`, `a_matrix_one_over_the_part_limit_is_refused_on_open`, `oversized_matrices_are_refused_through_set_matrix_and_project_matrices_with_state_unchanged`, `cells_beyond_the_matrix_dimensions_are_refused_by_set_matrix`.

## Answer

Merged commits: `ff25e26d3`, `433544050` (rustfmt only), `e1434a7d2`, merge commit `b70ac24f5`.

Delivered:

- The row/column bound `MAX_MATRIX_PARTS` = 4096 now runs first through `valid_dimensions`, before any row/column walk. This fixes the unbounded allocation that OOM-killed two agents.
- `validate.rs` shares `valid_dimensions`; `kernel.rs` keeps its own `usize` check.
- Refusal reasons are pinned by exact text in tests.
- `ProjectMatrices` and `SetMatrix` with rows/columns = `u32::MAX` are refused immediately, with state unchanged.
- The kernel text for `project-matrices` is kept: unifying it broke `boundary_parity::native_transport_reproduces_the_golden_transcript`.

Evidence:

- Standards and Spec reviews of the branch passed.
- The worktree quick check ended "all steps passed" under a 22G memory cap.
- Core tests: lib 802 passed, 4 ignored; integration 753 passed.

Labelling: only the `rows = u32::MAX` case was RED-first. The 4097 case, cells-over-dimensions, and the anchor/growth tests are characterization tests.

Carried forward:

- `MAX_MATRIX_PARTS` = 4096 is a pre-existing constant not stated in `spec.md`.
- The new tests listed in Progress are for the integrator's single baseline update.
- Unfixed lesson: failing-first tests with huge values must run under a memory cap.
