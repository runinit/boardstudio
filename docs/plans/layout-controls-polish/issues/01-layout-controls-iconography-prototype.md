# 01: Layout controls iconography prototype

Status: resolved
Type: prototype
Blocked by: none
Map: [map.md](../map.md)

## Question

What should the Layout canvas controls look like so they read smaller and match the rest of
BoardStudio ([DESIGN.md](../../../../DESIGN.md))? Behaviour stays as shipped.

## Outcome

A throwaway prototype that compares renderings of the same behaviour: the shipped baseline (0),
Grips (A), Rail (B), Triad (C) and Rail refined (D).

## Answer

Approved by the user on 2026-10-26: **variant D, "Rail, refined"**. Primary source: local branch
`prototype/layout-controls-iconography` (e91865f43), file
`web/crates/layout/prototype/layout-controls-iconography.prototype.html`, opened with `?variant=D`.
The prototype's spec sheet is the reference for exact values.

**Controls covered:**

- Column: Spread, Stagger, Splay and Origin.
- Row: Shift and Bend. Key: Shift and Bend.
- Matrix (the anchored thumb cluster): Move, Rotate and Anchor.
- Parts (MCU and TRRS jack, on both halves including the dimmed half): selectable, with Move,
  Rotate and Anchor.
- The anchor badge and "Placed from" readout, the Anchor pick mode, the dimmed-half label, the
  spacing note, the value popup and the commit toast.

**Sizing:**

- **Tokens:** 18 × 18 px, radius 4, 12 px glyph, 20 × 24 px hit area. They sit in one rail 22 px
  tall (radius 6, 2 px padding) 10 px above the selection top, centred on it.
- **Origin:** stays a point on the canvas, a 6 px ring with a 12 px cross and a 24 px hit area.
- **Labels:** 11 px mono at every zoom, in screen space. Part names sit at the top of the part,
  clear of the tie dot.
- **Anchor pick:** each candidate key shows a 4.5 px ring (6 px on hover, 24 px hit area). Major
  targets (parts and the half origin) show a 7.5 px ring with a cross (9 px on hover, 36 px hit
  area), and their label is always on.
- **Value popup:** panel, 1 px rule-strong, radius 8. The value is 15 px mono 600 and the delta is
  in accent. The popup docks 8 px above the rail while a tool is chosen, and sits 18 px off the
  pointer while dragging.
- **Toast:** panel, radius 8, 12.5 px with the value in bold, at the top centre of the canvas for
  1.8 s. It confirms each commit as one Undo step.
- **Anchor popup:** pinned to the canvas's top-right corner.

**Colour (design tokens):**

- A token's glyph is muted at rest. On hover it gets a selection-surface fill and a selection-ink
  glyph. When active it gets an accent fill and a white glyph, and the other tokens drop to 35%
  opacity.
- Accent: the selection, key hover (1.5 px accent stroke, 60% selection fill) and the axis guide
  (1 px dashed).
- Violet is only for anchoring: the anchor ring (7 px), the tie, the pick rings (the current one
  is filled with a white dot), the "Placed from" readout at the bottom left, and part outlines.
  A selected part turns accent.
- Amber marks only the keys that move (hover preview and anchor preview, 1.5 px outline) and the
  spacing note.
- On the dimmed half, key hover is solid muted.

**Behaviour shown:**

- Dragging a part shows the same popup and toast as dragging a key.
- The Inspector shows Offset X/Y and Rotation for a part, plus the "Placed from" readout.
- Parts follow their anchor key.
- Re-anchoring a part keeps its position, and an anchor that would make a loop is refused.

**Carried forward (see the map's open fog):**

- **Parts' Spread and Stagger:** the prototype gives parts only Move, Rotate and Anchor, while
  [split-board-editing 13](../../split-board-editing/issues/13-part-anchoring-column-vocabulary.md)
  also carries Spread and Stagger from the anchor key. The Matrix / Row / Column / Key toolbar is
  also unchanged while a part is selected.
- **Matrix re-anchor:** re-anchoring the thumb matrix still moves it, while parts hold still.
  Should matrices also hold still?

## Comments
