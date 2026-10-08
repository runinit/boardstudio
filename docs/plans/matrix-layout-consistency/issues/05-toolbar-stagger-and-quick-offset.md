# 05: Toolbar Stagger edits the column stagger; Shift-drag gives a quick offset

Status: ready-for-agent
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
