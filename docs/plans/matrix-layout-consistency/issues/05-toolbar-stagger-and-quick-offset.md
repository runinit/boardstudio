# 05: Toolbar Stagger edits the column stagger; Shift-drag gives a quick offset

Status: resolved
Type: build
Blocked by: 04
Spec: [spec.md](../spec.md#edits-in-the-web-layer) · Map: [map.md](../map.md)

## What to build

In `web/crates/layout/src/objects/layout_transform_toolbar.rs`, move every Stagger
route onto ticket 04's module:

- `sample_transform` for the preview;
- `transform_drag_result` and `transform_drag_resolver` for the commit (the net
  change from start to snapped value, applied to the accepted value when it runs,
  under the drag's transaction ID);
- `on_key_down` and `transform_nudge_resolver` for the keyboard;
- `same_transform_as_start` for the no-op check.

The gesture becomes `Stagger { column, quick_offset: bool }`. `quick_offset` is
latched from `shiftKey` at pointer-down. When it is true, the drag runs as a
column-offset gesture (2-D, column frame), and the handle shows the offset glyph
until release. Draw the stagger handle's arrow along `staggerAxis`, and set
`aria-valuenow` to the stagger.

Delete `stagger_by` and `local_matrix_delta`, which Core's scene axes replace.
Until 06, a Row selection renders no Stagger handle. Ship 05 and 06 together if
that gap is unacceptable.

## Impact

`stagger_by`, `sample_transform`, `transform_drag_result` and `local_matrix_delta`
(toolbar): LOW, the `LayoutTransformToolOverlay` process. Rerun first.

## Tests (layout browser batch, real Session and Core via `matrix_edit_test_support`)

RED first, before changing routes. This is the regression the handoff asks for:

1. `toolbar_stagger_and_inspector_stagger_land_the_same_matrix`. Land an Inspector
   Stagger of 2 on column 1, then Undo. Land a toolbar drag result of +2 on column 1.
   The accepted `Matrix` and the key poses are equal. It fails today, because the
   toolbar changes `column_offsets`.

Reassign existing tests:

- `three_handle_nudges_queued_behind_a_gated_reply_move_the_matrix_three_steps`:
  stagger +0.3 and offsets unchanged.
- `a_drag_commit_applies_its_net_transform_to_the_accepted_matrix`: as stagger
  (+1 landed, then a drag of +2, gives 3). Its 2-D form moves to 06.

New tests:

2. The preview operation is `SetMatrixPlacement::ColumnStagger` only.
3. Escape and cancel land nothing; the revision and Undo depth are unchanged.
4. One drag is one Undo step; Redo restores it.
5. A held Core reply on an unrelated edit: the drag composes and both land.
6. A quick-offset drag (Shift latched) changes only `column_offsets[c]`. Releasing
   Shift mid-drag does not switch the parameter.
7. Keyboard on a mirror-Y matrix: ArrowUp moves the column up on screen.
8. Linked pair: a drag on the right half updates both halves equally.

## Acceptance

Tests pass. `lint`, `test`, `typecheck` and `browser` pass.

## Files

Owner of `layout_transform_toolbar.rs` until 06 resolves.

## Outcome

Resolved 2026-10-09. Every Stagger route in `layout_transform_toolbar.rs` goes through
`matrix_transform_operation.rs`: preview (`gesture_value` + `placement_operation`), commit (net
change added to the accepted value under the drag's transaction ID), keyboard (`nudge_value`
against the accepted scene's stagger axis; the perpendicular pair is not consumed) and the no-op
check. The gesture is `Stagger { column, quick_offset }`, latched from `shiftKey` at pointer-down;
a Shift drag edits `column_offsets[c]` in the column frame and shows an offset glyph until
release. The arrow follows `staggerAxis`; `aria-valuenow` is the stagger. `stagger_by` and the
toolbar's `local_matrix_delta` are deleted. A Row selection renders no Stagger handle until 06.

Tests: `toolbar_stagger_and_inspector_stagger_land_the_same_matrix` was red first (the toolbar
changed `column_offsets`), plus the two reassigned tests and tests 2 to 8, all in the layout
browser batch.

Known gaps, for 06 and 11: no test dispatches real pointer events, so the Shift latch at
pointer-down and the glyph are not exercised end to end; the drag-commit resolver does not retire
when the column has gone (Core refuses cleanly, nothing mutates); stagger snapping uses the grid's
Y step whatever the axis; `aria-valuenow` on `role="button"` is not valid ARIA (the ticket asked
for it; consider `slider` in the design pass); `RowOffset` arms in the toolbar are unused until 06.
`layout_align_controller.rs:822` remains a second `column_offsets` writer for 11 to decide.
