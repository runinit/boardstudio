# 06: Offset tool for row and column offsets

Status: resolved
Type: build
Blocked by: 05
Spec: [spec.md](../spec.md#edits-in-the-web-layer) · Map: [map.md](../map.md)

## What to build

Add `LayoutTransformTool::Offset`, the old 2-D drag under its accurate name, built
by ticket 04's module.

- Menu: Stagger, Offset, Splay, Origin.
- Activation (`web/src/presentation.rs`, `on_transform_tool`): Offset keeps a Row
  selection and turns other matrix selections into their Column. Stagger turns a Row
  selection into the selected key's column if there is one, and otherwise into
  column 1. Record the choice in the Outcome.
- Handle: four-way glyph. `aria-label` is "Drag to offset row" or "Drag to offset
  column". Arrows step in screen directions through the scene frame.
- DESIGN.md: the Layout sentence "Stagger, Splay, and Origin activate explicit
  canvas tools" names Offset and the Shift quick offset. DESIGN.md holds unrelated
  uncommitted edits: stage only this hunk (AGENTS.md partial-staging method).

## Tests (layout browser batch)

1. The old 2-D drag test returns as Offset: `column_offsets[0] == (2, 1)`.
2. A Row Offset drag and nudge change only `row_offsets[r]`.
3. A Column Offset on a splayed, rotated matrix: the keys follow the pointer's world
   delta (1e-6). Staggers are unchanged.
4. Menu availability and selection-kind changes for Row, Column and Key.

## Acceptance

Stagger and Offset never write each other's parameter. `rg` finds no web writer of
`column_staggers`, `column_offsets` or `row_offsets` outside Core-bound operation
building. `lint`, `test`, `typecheck` and `browser` pass.

## Files

`layout_transform_toolbar.rs`, and the tool-activation block of `presentation.rs`.
Ticket 09 edits a different region of `presentation.rs`; rebase before committing.

## Outcome

Resolved 2026-10-09. `LayoutTransformTool::Offset` is the old 2-D drag under its accurate name,
built by `matrix_transform_operation.rs`; the menu is Stagger, Offset, Splay, Origin. It writes
`RowOffset` or `ColumnOffset` only. Handle: four-way glyph, `aria-label` "Drag to offset row" or
"Drag to offset column", arrows step in screen directions through the scene frame.

Activation choice (`activate_transform_tool`, called from `on_transform_tool`): Offset keeps a Row
selection and turns other matrix selections into their Column. Stagger turns a Row selection into a
Column through the existing selection conversion (the retained key's column, otherwise column 1).
Splay and Origin turn any selection into their Column. Choosing the active tool again turns it off.

Closed from 05's gaps: the drag-commit and nudge resolvers retire when the column or row has gone;
stagger snaps to the grid step of its axis' dominant component (scope beyond the ticket, with its
own test); the handle carries `aria-description` instead of `aria-valuenow` (support is uneven; a
later design pass may choose again); one browser test dispatches real pointer events for the Shift
latch (pointer capture is stubbed because a synthetic pointer id cannot be captured). DESIGN.md
Layout sentence updated.

Tests were written with the implementation; red evidence is a mutation (Offset resolved to Stagger,
Shift read dropped: four tests failed, then passed on restore). Known limits: a Row Offset uses
column 0's frame, so on a splayed matrix other columns may move in a different world direction from
the arrow; `layout_align_controller.rs:822` is still a second `column_offsets` writer for 11.
