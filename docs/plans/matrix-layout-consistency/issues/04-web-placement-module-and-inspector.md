# 04: One web module builds placement edits; Inspector uses it

Status: claimed
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
