# Spec: Matrix transforms and visible relationships

Label: wayfinder:spec
Map: [map.md](map.md) · Handoff: [handoff-opus.md](handoff-opus.md)

## Problem

A matrix already places every key parametrically (pitch, row and column offsets,
cumulative column stagger, splay, mirror, rotation, origin), but the product does not
show or edit that structure consistently:

- Inspector **Stagger** edits the relative column stagger. Toolbar
  **Transform → Stagger** writes the column's independent offset instead. The same
  word changes different parameters, so after a toolbar stagger the Inspector shows
  `Stagger 0` and `Offset Y −9.525`.
- **Relations** on a new matrix, row, column or key says "No saved placement
  relationship on this selection." The structure that actually places the selection
  (inherited spacing, stagger, attached components) is invisible.

## Source-confirmed behaviour (at `f65ed01a6`)

Each point was read from source for this plan. None was re-run as a test or browser
reproduction.

1. **Placement.** `core/src/matrix.rs::location`: local
   `x = c·pitch.x + row_offset[r].x + column_offset[c].x + cell.offset.x`,
   `y = r·pitch.y + row_offset[r].y + column_offset[c].y + cell.offset.y + Σ stagger[0..=c]`.
   Splays then rotate the point, from column `c` down to `0`. Each column's pivot is its
   custom origin if one exists, otherwise `(j·pitch.x, Σ stagger[0..=j])`. Mirror and
   matrix rotation are applied next, and the origin last.
2. **Inspector.** `matrix_transform_operation.rs::build_operation`: `ColumnStagger`
   sets `column_staggers[c]` (an absolute value) and leaves the other columns alone, so
   the change carries to every later column. `ColumnOffsetX/Y` and `RowOffsetX/Y` set
   one `column_offsets`/`row_offsets` entry. `transform_edit_resolver`
   (`matrix_transform_controller.rs`) rebuilds the fields from the accepted document
   when the edit runs: `Unchanged` if the value is equal, `Retire` if the target has
   gone, otherwise a `SetMatrix` submit with an empty transaction ID, so Session gives
   it its own ID and its own Undo step.
3. **Toolbar.** In `layout_transform_toolbar.rs`, the Stagger gesture is
   `{row_axis, index}`. `sample_transform` (preview), `transform_drag_result`
   (net delta at drop), `transform_drag_resolver` (commit) and
   `transform_nudge_resolver` (keyboard) all read or write `row_offsets` or
   `column_offsets` through `stagger_by`. The drag converts screen to local with
   `local_matrix_delta`, which uses splay through the column itself. Keyboard deltas
   are added in local coordinates with no conversion, so ArrowUp on a Y-mirrored
   matrix moves the column down on screen.
4. **Tool activation** (`web/src/presentation.rs`, `on_transform_tool`): Stagger keeps
   a Row selection and turns any other matrix selection into a Column selection.
5. **Gesture settlement.** A drag starts only when the lifecycle is `Ready`, durability
   is `Saved` at the owner's revision, no preview or gesture is active, and the accepted
   matrix equals the drag's start matrix. Each move submits `Event::PreviewEdit` under
   one transaction ID. On drop, the net delta goes through a resolver that applies it
   to the matrix accepted when it runs, under the same transaction ID: one Undo step.
   Cancel, Escape, lost capture, an owner change, or a drop that matches the start send
   `ClearPreview` and commit nothing. Each key press is a delta resolver with an empty
   transaction ID, so each press is its own Undo step. Presses queue while the
   lifecycle is `Applying` or `Saving`.
6. **Linked halves.** `core/src/matrix/layout.rs::reflected` copies `pitch`,
   `row_offsets`, `column_offsets`, `column_staggers`, `column_splays`,
   `column_origins` and the cell offsets verbatim. It flips the mirror between
   None and X, negates the rotation, and reflects the origin about the link axis.
   `SetMatrix` on either half syncs the other. Matrix-local parameters are therefore
   identical on both halves. Y mirror is rejected on a linked pair.
7. **Companions** (`assembly_placements`, `constraints::pose`). An attached assembly
   follows its key at `assembly.offset` and rotation. A companion with the
   `layoutOverride` property keeps its authored pose until the key moves, then follows
   it at its authored relative pose. An explicit `Constraint` targeting a companion
   takes precedence over the attachment. Linked keys cannot be constraint targets.
8. **Relations** (`presentation.rs::layout_relationship_summary`): for a matrix
   context, the panel shows one sentence. That is the linked-half message, "This layout
   is independent…", the first constraint driving the active part, or the fallback. A
   static note, "Matrix rows and columns share pitch, stagger and splay", follows, plus
   an "Edit placement relationship" button for a constraint.
9. **Labels.** The Column Inspector shows `Column {column}` 0-based
   (`matrix_transform_inspector.rs:251`). The tree and the context title use 1-based
   labels.

## Derived geometry (from point 1, to be confirmed by ticket 02)

Write `R(θ)` for rotation by θ, `M` for the matrix mirror, `Q` for the matrix rotation,
and `s_j` for column `j`'s splay. Changing a parameter by Δ moves the keys of column
`c` in world coordinates as follows:

| Parameter Δ | World displacement of column `c` |
| --- | --- |
| `column_offsets[c]` by local `(dx, dy)` | `Q·M·R(Σ_{j≤c} s_j)·(dx, dy)`, this column only |
| `row_offsets[r]` by local `(dx, dy)` | same rotation as the column-offset row, applied to each key of row `r` in its own column `c` |
| `column_staggers[k]` by Δ, for `c ≥ k` | `Q·M·R(Σ_{j<k} s_j + Σ_{j∈[k,c], custom origin} s_j)·(0, Δ)` |
| cell offset (key local) | same rotation as the column-offset row, this key only; attached companions follow |

Stagger moves the default splay pivots of columns `k..` with the keys, so their own
splays do not rotate the displacement. A custom origin is a fixed pivot, so its
column's splay does rotate it. The **stagger axis** of column `k` is the unit vector
`a_k = Q·M·R(Σ_{j<k} s_j + [custom_k]·s_k)·(0, 1)`, the direction column `k`'s own keys
move per +1 mm of stagger. For column 0 the sum over earlier columns is empty: the
reference is the matrix base frame, not an earlier column.

Worked example (unsplayed): staggers `[0, 2, −1]` give column Y contributions
`[0, 2, 1]`. Raising stagger 1 by 3 gives `[0, 5, −1]`, so the contributions become
`[0, 5, 4]` and columns 1 and 2 both move +3. Column 0's stagger is measured from the
matrix origin's row-0 line. Changing it moves every column.

## Behaviour (decided in [ticket 01](issues/01-decide-matrix-transform-contract.md#answer); [ADR-0007](../../adr/0007-matrix-gestures-edit-named-parameters.md))

### Terms

| Term | Meaning | Parameter |
| --- | --- | --- |
| **Column stagger** | Relative Y step of a column from the previous column (column 0: from the matrix base), in matrix coordinates; that column and all later columns inherit it | `column_staggers[c]` |
| **Column offset** | Independent X/Y adjustment of one column only; nothing inherits it | `column_offsets[c]` |
| **Row offset** | Independent X/Y adjustment of one row only | `row_offsets[r]` |
| **Key local offset** | Adjustment of one key inside the matrix; its attached companions follow | `cell.offset`, `cell.rotation` |
| **Structural relationship** | Placement inherited from the matrix and its assemblies, projected from the accepted document; no record of its own | — |
| **Explicit constraint** | A separately authored `Constraint` between parts, shown with its source, target and precedence | `doc.constraints` |


The terms are recorded in [GLOSSARY.md](../../../GLOSSARY.md#matrix-layout).

### Core owns the parameter rules

- **Typed edit.** `EditOperation::SetMatrixPlacement { matrix_id, change }`, shaped
  like `SetMatrixSplay`, with
  `MatrixPlacementChange::{ColumnStagger { column, value: f64 }, ColumnOffset { column, value: Vec2 }, RowOffset { row, value: Vec2 }, ColumnOffsetToStagger { column }}`.
  Values are absolute and in matrix coordinates. Core validates the index and that
  the value is finite, pads the vectors, applies the change, syncs the linked half
  (`layout::sync`), resolves constraints, and reports precise changed IDs (the
  matrix plus moved members and companions). Applying the current value is a valid
  no-op.
- **Convert offset to stagger** (`ColumnOffsetToStagger`) is a one-shot action. It
  moves `column_offsets[c].y` into `column_staggers[c]`, subtracts the same amount
  from `column_staggers[c+1]` if that column exists, and zeroes the Y offset. X
  stays. Core accepts it only when the geometry is exactly preserved: column `c`
  has splay 0 or a custom splay origin, per the derived geometry. Otherwise it
  errors with "Converting would move this column: its splay pivots with the
  stagger." The UI disables the action with the same reason.
- **Scene axes.** `MatrixColumnBasis` gains `staggerAxis` (`a_k`, computed by Core
  by differencing `location` with stagger +1). The existing `axisX`/`axisY` already
  give the column-offset frame `R(Σ_{j≤c} s_j)`. The row-offset frame is column 0's.
- **Companion placement modes.** `MatrixScene` gains `companions`: one entry per
  generated companion with `partId`, `keyId` and
  `mode: follows | authored | constrained { constraintId, sourcePartId }`. It is
  derived from the same inputs as `assembly_placements` and `constraints::pose`,
  so the classification has one source.
- Key local offsets keep their existing routes (canvas moves from `f65ed01a6` and
  the Inspector cell transform). Moving them to a typed edit belongs to the
  typed Core edits effort.

### Edits in the web layer

- **One web module.** `matrix_transform_operation.rs` is the only place that builds
  a `SetMatrixPlacement` from an intent. It takes an absolute value (Inspector), or
  a gesture's world delta, which it projects onto Core's scene axes. Inspector,
  toolbar preview, drag commit and keyboard nudge all call it. A typed value and a
  gesture that reach the same value send the identical operation.
- **Stagger tool.** A Column, Key or matrix Component selection becomes its column.
  The drag is one-dimensional: the world delta is projected onto `staggerAxis`, and
  motion across the axis is ignored. The handle is drawn along the axis.
  **Shift held at pointer-down** turns that drag into a free 2-D column offset
  (quick offset), shown with the Offset handle glyph for the duration of the drag.
  The modifier is read only at pointer-down, so a drag never switches parameter
  part-way. A Row selection does not offer Stagger.
- **Offset tool.** A Row selection edits `row_offsets[r]` in column 0's frame.
  Column, Key and Component selections edit `column_offsets[c]` in the column's
  frame. The drag is free 2-D.
- **Snapping (change and value).** While dragging, the *resulting value* snaps to
  the grid step (`snap_fraction` × pitch: pitch.y for stagger, each axis for
  offsets). When the start value is on the grid, the change is therefore a whole
  number of steps too. When it is off the grid, the first snap brings it onto the
  grid. Alt disables snapping. The commit sends the snapped value minus the start
  value as a change, resolved against the accepted matrix. Keyboard steps are exact
  (±0.1 mm, Shift ±1 mm) and are not grid-snapped.
- **Keyboard.** On the stagger handle, the arrow pair closest to the on-screen
  direction of `staggerAxis` changes Stagger in the direction the column moves on
  screen. The perpendicular pair is not consumed. On the offset handle, arrows step
  in screen directions, converted through the scene frame. This also fixes today's
  unconverted keyboard delta.
- **Inspector.** Labels stay **Stagger** and **Offset X/Y** with **Reset offset**.
  An `impeccable` pass later handles hints and layout (outside this effort). The
  Column label becomes 1-based. Column 1's Stagger stays editable. Its reference is
  the matrix origin, and Relations says so.
- **Linked halves.** Scene axes are per half, so a gesture on either half projects
  to the same matrix-local value, and Core syncs both halves.
- **No migration.** Saved offsets keep their meaning and geometry. Opening a project
  never normalizes them. Convert is explicit and only offered when exact.

### Settlement (unchanged contract)

Field edits remain absolute value intents resolved against the accepted matrix: the
latest value wins, an equal value is `Unchanged`, and a vanished target retires
silently or with the existing inline message. A drag still requires the start
conditions in point 5. It previews under one transaction, commits its net change
through one resolver under that transaction (one Undo step, composing with edits
accepted meanwhile), and Cancel, Escape or a no-op drop clears the preview. Key
presses stay per-press change intents with their own Undo steps. Pending drafts in other
fields are untouched. Only the parameter each intent names changes.

All of them now land as `SetMatrixPlacement`. Convert is a one-shot action: its
control is disabled while it is pending.

### Relations

Relations is a web projection (`web/crates/layout/src/matrix_relations.rs`) of the
accepted document and scene. It has no store of its own, and it takes companion
modes from Core's scene instead of re-deriving them. It has two sections:
**Structure** (inherited) and **Authored constraints** (explicit). Local
adjustments are **departures**: items inside Structure that mark where a value
overrides what is inherited, each with Reset (and Convert, for a column offset).

| Selection | Structure shows | Departures | Constraints |
| --- | --- | --- | --- |
| Matrix | rows × columns, enabled/disabled/removed key counts; shared pitch and edge gap; origin, rotation, mirror; per-column stagger and splay when nonzero; linked half (partner name) or independent | counts of rows/columns with offsets, keys with local offset/rotation, authored companions | every constraint whose source or target is a member or companion |
| Row | membership (keys in row); spacing from the previous row by pitch Y; columns' stagger applies to each key | row offset, if nonzero, with Reset | constraints on its keys |
| Column | stagger value and its reference ("from Column N" / "from the matrix origin"), cumulative Y, columns that inherit it (list); splay angle and pivot (default/custom), columns it pivots | column offset, if nonzero, with Reset and Convert | constraints on its keys |
| Key | what places it: pitch cell, row offset, column offset, cumulative stagger, splay, matrix frame; linked partner key; attached companions, each with its Core mode | key local offset/rotation | constraints where it is source or target |
| Matrix component | attached to key (navigable); Core mode: follows key / authored position relative to key / **explicit constraint takes priority** | authored position | constraint on it, with its source |

- **Navigation.** Every referenced object is a button that selects it (Escape
  returns, as today).
- **Highlight.** Hovering or focusing a reference highlights its objects on the
  Layout canvas: the parts, or all members of a row, column or matrix. The
  highlight is transient view state in `ui-model`, the same kind as finding focus.
  It never enters the document or Undo, and it clears on blur, on selection change
  and when the target leaves the accepted document.
- **Departed targets.** A target that no longer exists when activated does nothing.
  The next accepted revision re-projects and drops it. A newly created blank matrix
  shows non-empty Structure at once.

## Out of scope

PCB and electrical remap; unrelated Inspector work; the Inspector wording and visual
pass (a later `impeccable` session); typed edits for key local offsets; row stagger.

## Acceptance (effort)

- An Inspector value and an equivalent toolbar gesture (preview, commit, keyboard)
  send the identical `SetMatrixPlacement` and produce equal `Matrix` parameters and
  Core poses, for plain, rotated, mirrored, splayed (default and custom origin) and
  linked matrices.
- Toolbar Stagger changes `column_staggers[k]` and moves columns `k..` together.
  Shift-drag and the Offset tool change only offsets.
- Convert preserves every pose exactly, or is unavailable with its reason.
- Undo/Redo, drag cancel, queued unrelated edits and save/reopen behave as in
  Settlement. Save/reopen leaves geometry and saved offsets unchanged.
- Relations is meaningful on a blank matrix and on every selection kind, separates
  structure from constraints, highlights references on the canvas, and handles
  selection changes and departed targets.
