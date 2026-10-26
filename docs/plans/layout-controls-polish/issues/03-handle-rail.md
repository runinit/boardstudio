# 03: Handle rail for the existing tools

Status: ready-for-agent
Type: build
Blocked by: none (decision in [01](01-layout-controls-iconography-prototype.md))
Map: [map.md](../map.md)
Prototype: branch `prototype/layout-controls-iconography` (e91865f43), `?variant=D`; its spec sheet holds the exact values.
Review: presentation only; RED/GREEN evidence and a diff read, no combined review.

## What to build

The canvas drag handles that ship today become labelled tokens in one rail above the selection.
Each drag still edits the same field through the same op. The handles are rendered in
`LayoutTransformToolOverlay`
(`web/crates/layout/src/objects/layout_transform_toolbar.rs`, the `m1-transform-handle` /
`m1-spread-handle` / `m1-stagger-handle` / `m1-splay-*` / `m1-matrix-*-handle` nodes), and
`transform_tools` (`web/src/presentation/layout_canvas.rs`) picks them.

- **Tokens:** 18 × 18 px, radius 4, 12 px glyph, a 20 × 24 px hit area, gathered in one rail
  22 px tall (radius 6, 2 px padding, panel fill, 1 px `rule-strong`, soft shadow). The rail sits
  10 px above the selection's top and is centred on it, at screen size at every zoom.
- **States:** at rest the glyph is muted. Hover gives a `selection-surface` fill, a
  `selection-ink` glyph, amber on the keys that would move, and a faint axis guide. Active gives
  an accent fill and a white glyph, the other tokens drop to 35% opacity, and the axis guide
  becomes 1 px accent dashed.
- **Covered here:** Column Spread, Stagger and Splay; Row Offset (`PlacementParameter::RowOffset`) as it ships today;
  Matrix Move and Rotate. Origin stays a point on the canvas: a 6 px ring with a 12 px cross and
  a 24 px hit area. Ctrl-drag quick Offset, keyboard nudge (`aria-label` lookup) and
  hidden-handles behaviour keep working.
- New handles (Row Bend, Key Shift/Bend) are [05](05-row-and-key-shift-bend-handles.md). Parts
  are [07](07-part-rail.md), and the Anchor token is [08](08-canvas-anchor-pick.md).

## Seam and first RED / GREEN slice

Seam: a pure rail layout in `web/crates/layout`. It takes the selection's screen bounds and the
ordered tool list and returns token rectangles plus the rail rectangle. It is native-tested, and
the overlay only draws what it returns.

RED: a native test that three column tools over a known bounding box produce a rail
`3·18 + 2·2 + 4` px wide, centred on the box, with its bottom edge 10 px above the box top. It
also checks that the layout is the same at two zooms in screen space.

## Acceptance

- [ ] Native rail-layout tests: width, centring, offset, zoom invariance and an empty tool list.
- [ ] The existing mounted handle tests in `layout_transform_toolbar.rs` (the `aria-label`
  queries for "Drag to splay", "Drag to move matrix" and the others) still pass against the
  tokens. They may change only in their geometry assertions, and each such change is listed.
- [ ] A mounted test covers hover (selection-surface, amber movers), active (accent, others at
  35%) and leave, with no document revision.
- [ ] Live-app drag pass for each covered tool, including Ctrl-drag quick Offset and the
  keyboard nudge.
- [ ] DESIGN.md's "Stagger, Offset, Splay, and Origin activate explicit canvas tools" paragraph
  describes the rail.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser), including
  the drag smoke. `check.py test --update-native-baseline` only for deliberate test renames.

## Carried point

- The prototype is desktop only. DESIGN.md asks for 44 px touch targets in the compact
  layout. Whether tokens grow their hit area at ≤980 px is undecided; ask before building a
  compact variant, and by default leave compact sizing to a follow-up.
