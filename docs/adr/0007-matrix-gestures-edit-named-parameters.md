---
status: accepted
---

> Amended by [ADR-0036](0036-matrices-follow-ergogen-point-layout.md): saved projects are no
> longer migrated, so "saved offsets keep their meaning" no longer applies.

# Matrix gestures edit named placement parameters through Core

The Inspector's **Stagger** set the relative column stagger. The toolbar's
**Transform → Stagger** changed the column's independent offset. Its preview,
drop, commit and keyboard routes each did their own screen-to-matrix conversion
in the web layer. One word named two parameters, and the web layer copied part of
Core's placement maths to do it
([matrix layout consistency](../plans/matrix-layout-consistency/spec.md)).

On 2026-10-08 the user decided that every matrix placement adjustment names the
parameter it changes, and that Core owns the rules:

- **Stagger** always means the relative column stagger, which later columns
  inherit. **Offset** means an independent row or column adjustment. The toolbar
  has a separate Offset tool. Holding Shift while dragging the Stagger handle
  gives a quick column offset for that drag.
- Inspector fields, toolbar previews, drag commits and keyboard nudges reach Core
  through one typed edit, `EditOperation::SetMatrixPlacement`. It names the
  parameter and its new value, following the `SetMatrixSplay` pattern and the
  direction of [ADR-0006](0006-replace-document-for-import-and-recovery.md).
  Resolvers compute that value from the accepted matrix when they run, as
  [ADR-0005](0005-resolve-queued-edits-at-execution.md) requires.
- Core publishes the world direction of each matrix parameter in its scene,
  including a column's stagger axis. Gestures project the pointer delta onto
  those axes. The web layer does no splay, mirror or rotation maths.
- Core classifies how each attached companion is placed (follows its key,
  authored position, or explicit constraint first) and publishes that in the
  scene. Relations displays it and does not re-derive Core's precedence.
- Saved offsets keep their meaning. Opening a project never converts or
  normalizes them. Converting a column offset into stagger is an explicit
  one-shot action, which Core offers only when the geometry stays exactly the
  same.

## Consequences

- `EditOperation` gains `SetMatrixPlacement`, and the scene gains `staggerAxis`
  and companion placement modes. These are public Core interface additions. The
  saved document format does not change. `MatrixPlacementChange::ColumnOffsetToStagger`
  and the pure helper `convert_offset_available` are the only extra public surface.
- The boundary transcript fixture is regenerated once the scene grows.
- Toolbar Stagger behaves differently from before. Rows use the Offset tool.

## Considered Options

- **A web-only semantic module** that keeps `SetMatrix` and duplicates the axis
  maths, with tests that check it against Core: no interface change, but two
  copies of the placement rules that can drift apart.
- **Core computes relationships in full** (a complete relations projection): the
  most Core ownership, but the panel's grouping and wording would sit in Core
  too.

## Amendment: carried chains and anchors in Relations (2026-10-23)

[The accepted layout-parity Relations decision](../plans/ergogen-layout-parity/issues/12-relations-and-highlight.md#answer)
extends scene facts to carried contributions, cumulative values and matrix anchors.
Core owns the geometry facts; the presentation owns their grouping, wording and editor
navigation. Reference highlighting is not an edit preview. This preserves the existing
seam rather than moving the formatted Relations tree into Core.
