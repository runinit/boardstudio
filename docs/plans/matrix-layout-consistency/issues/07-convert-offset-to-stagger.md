# 07: Convert a column offset to stagger, only when geometry is preserved

Status: claimed
Type: build
Blocked by: 04
Spec: [spec.md](../spec.md#core-owns-the-parameter-rules) · Map: [map.md](../map.md)

## What to build

- **Core.** `MatrixPlacementChange::ColumnOffsetToStagger { column }`:
  `stagger[c] += offset[c].y`, `stagger[c+1] −= offset[c].y` (when column `c+1`
  exists), `offset[c].y = 0`. Errors, leaving the document unchanged, unless column
  `c` has splay 0 or a custom splay origin. The error text is "Converting would move
  this column: its splay pivots with the stagger." Also export
  `pub fn convert_offset_available(matrix, column) -> Result<(), String>` for the UI,
  so the rule has one source. **Public Core function: confirm in review** (ADR-0007
  covers the operation; this helper is the only extra surface).
- **Inspector.** On a column with a nonzero offset Y, show a **Convert to stagger**
  one-shot button next to Reset offset. Disable it while pending. When
  `convert_offset_available` errs, disable it and show the reason. Submit through a
  resolver that retires if the offset is already 0 or the column has gone.
- Relations reuses this action in ticket 09.

## Tests

Core (native):

1. With an unsplayed column and an offset Y of −9.525 (the handoff's screenshot
   case), Convert leaves every pose identical (1e-9). The stagger holds the value and
   the next column compensates.
2. The same on the last column (no compensation needed) and on a column with a
   custom origin and splay.
3. A splayed column with a default origin errors and the document is unchanged.
4. Linked pair: both halves convert, and the poses are identical.

Browser (layout batch):

5. The button appears only for a nonzero offset Y. Convert is one Undo step, and the
   Inspector shows Stagger −9.525 and Offset Y 0.
6. On a splayed column it is disabled with the reason.

## Acceptance

`lint`, `test`, `typecheck` and `browser` pass.

## Files

The Core change sits in `matrix/placement.rs` and `model.rs`; rebase with 03 if it
is still open. The Inspector change sits in `matrix_transform_inspector.rs` and its
controller (after 04).
