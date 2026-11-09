# 04: One web module builds placement edits; Inspector uses it

Status: resolved
Type: build
Blocked by: 02
Spec: [spec.md](../spec.md#edits-in-the-web-layer) · Map: [map.md](../map.md)

## What to build

Make `web/crates/layout/src/matrix_transform_operation.rs` the only web code that
builds a `SetMatrixPlacement`:

- `fn placement_value(parameter, Set(value))` gives an Inspector value.
- `fn gesture_value(scene_basis, parameter, start, world_delta, snap)` gives a drag
  sample. It projects the world delta onto Core's `staggerAxis` (stagger) or onto
  `axisX`/`axisY` (a column offset in its column's frame; a row offset in column 0's
  frame). It snaps the resulting value to the grid step (spec: snapping), with Alt
  skipping the snap via `snap: None`, and returns the absolute value.
- `fn nudge_value(scene_basis, parameter, current, key, big)`: the stagger arrow
  pair aligned with the on-screen axis, offset arrows in screen directions, and
  `None` for the perpendicular stagger pair.

`build_operation`'s `ColumnStagger`, `ColumnOffsetX/Y/Reset` and
`RowOffsetX/Y/Reset` arms return `SetMatrixPlacement`. Delete `set_matrix_offset`.
Key fields keep their current operations. In
`objects/matrix_transform_inspector.rs`, make the Column context label 1-based. Make
no other wording change; the `impeccable` pass owns it.

## Impact

`build_operation` (this file): **HIGH**, 23 impacted symbols. Callers are
`transform_edit_resolver`, `selected_keys_enabled_resolver`, the preflight check at
`matrix_transform_controller.rs:762`, and tests. Keep its signature; non-placement
arms are unchanged. Rerun `impact` first.

## Tests

Native (`cargo test -p boardstudio-web-layout`), RED first:

1. `inspector_and_gesture_send_the_same_operation`: for each parameter, a typed
   value `v` and a gesture whose delta reaches `v` produce equal `EditOperation`s.
   Use `CoreEngine` for the scene of a rotated, mirrored, splayed (custom origin)
   matrix.
2. `gesture_value_snaps_the_resulting_value`: start 0.3 with a step of 4.7625 lands
   on 4.7625 or 0. A start already on the grid moves in whole steps. Alt gives the
   raw value.
3. `nudge_pairs_follow_the_axis`: an axis rotated 90° on screen consumes
   Left/Right, not Up/Down. A mirror-Y matrix's ArrowUp increases the on-screen Y.
4. Existing `row_offset_changes_only_the_selected_row…` and
   `remaining_numeric_fields_map_to_their_existing_matrix_properties` (~941, ~1449):
   assert the `SetMatrixPlacement` payloads, which mean the same thing.

Browser (layout batch; `matrix_transform_inspector_tests.rs`):

5. The Inspector Stagger field lands through the real Session and Core as
   `SetMatrixPlacement` and moves later columns. Its Undo is one step.
6. The Column label is 1-based. Update any existing assertion on the 0-based label
   and record it in the Outcome.

## Acceptance

Tests pass. `lint`, `test`, `typecheck` and `browser` (layout batch) pass. No other
web code writes `column_staggers`, `column_offsets` or `row_offsets`. Check with
`rg` after ticket 06; the toolbar still does until 05/06.

## Files

Owner of `matrix_transform_operation.rs`, `objects/matrix_transform_controller.rs`,
`objects/matrix_transform_inspector.rs` and `matrix_transform_inspector_tests.rs`.

## Outcome

Resolved 2026-10-09. `matrix_transform_operation.rs` now builds every placement edit through one
function, `placement_operation(matrix_id, PlacementParameter, PlacementValue)`, and offers
`placement_value` (an Inspector field replaces one component), `gesture_value` (world delta
projected onto Core's `staggerAxis`, or solved in the column's `axisX`/`axisY` frame, then snapped
to the grid step, `None` skipping the snap) and `nudge_value` (0.1 mm, Shift 1 mm, exact; stagger
consumes the arrow pair closest to its on-screen axis and returns `None` for the perpendicular pair).
`SceneBasis::for_parameter` reads the frame from the scene; a row offset uses column 0's. The
`ColumnStagger`, `ColumnOffsetX/Y/Reset` and `RowOffsetX/Y/Reset` arms of `build_operation` return
`SetMatrixPlacement`; `set_matrix_offset` is deleted. The Inspector's Column context label is
1-based (no existing assertion used the old label; Row and Key labels are unchanged, as the ticket
says).

Tests: native `inspector_and_gesture_send_the_same_operation` (rotated, mirrored, splayed,
custom-origin matrix from `CoreEngine`; the world delta is worked from the scene axes, equality to
1e-9 because the two paths round differently), `gesture_value_snaps_the_resulting_value`,
`nudge_pairs_follow_the_axis`; the two existing row and column tests now assert
`SetMatrixPlacement` payloads. Mounted: `inspector_stagger_lands_as_a_placement_edit_that_moves_later_columns_with_one_undo`
(real Session and Core; the first edit syncs the fixture's unlaid poses, so the test measures the
second) and `the_column_context_label_is_one_based`. Gates: `check.py quick` ran lint, typecheck,
native tests (layout 75) and the layout browser batch (147 passed, 1 failed: the Stagger test's own
baseline, since fixed; the 15 inspector tests then passed on a targeted rerun; the full layout batch
was not rerun after that test-only fix). GitNexus: `build_operation` HIGH, 23 symbols, signature
unchanged; change analysis low risk.

Differences from the ticket sketch: `placement_value` takes the current value and a `PlacementSet`
(field and number), because an offset field changes one component. The toolbar still writes offsets
itself (`layout_transform_toolbar.rs`, tickets 05 and 06), and `layout_align_controller.rs` also adds
to `column_offsets` through `SetMatrix`; that second writer is not named in the plan, so 11's
verification should either route it through `SetMatrixPlacement` or record it as an accepted
exception. `ColumnOffsetToStagger` does not exist yet (ticket 07).

