# 07: Convert a column offset to stagger, only when geometry is preserved

Status: resolved
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

## Outcome

Resolved 2026-10-09. Core: `MatrixPlacementChange::ColumnOffsetToStagger { column }` and
`pub fn convert_offset_available(&Matrix, u32) -> Result<(), String>` in `core/src/model.rs`
(`mod matrix` is private, and `placement.rs` calls the same function, so the rule has one source).
It moves offset Y into `stagger[c]`, compensates `stagger[c+1]`, keeps X, and refuses with the
ticket's text unless the splay is 0 or the origin is custom. Offset Y of 0 is a no-op, even on a
splayed column. Inspector: a "Convert to stagger" one-shot button next to Reset offset, shown only
for a nonzero offset Y, disabled while pending or when Core refuses (reason shown beside it); the
resolver re-checks availability and reports `Unchanged` once offset Y is 0.

Tests: Core 1 to 4 red first, then green (poses identical to 1e-9; refusal leaves the document
unchanged; both linked halves convert); browser 5 and 6 pass. The boundary golden is append-only
(2843 added lines, none removed; revisions 43 to 45). Review: no blocking findings from Standards
or Spec reviews. The extra public Core function is accepted; ADR-0007 now mentions it. A
scene-published per-column `convertible` flag would avoid it if a later ticket prefers that.
