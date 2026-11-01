# 08: Canvas Anchor pick mode

Status: ready-for-agent
Type: build
Blocked by: [02](02-screen-space-canvas-chrome.md), [04](04-re-anchor-moves.md), [06](06-value-popup-and-commit-toast.md), [07](07-part-rail.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/layout-controls-iconography` (e91865f43), `?variant=D` (anchor pick, `data-anchor`).
Review: web-only behaviour on existing Core ops; RED/GREEN evidence and a diff read.

## What to build

An **Anchor** token on the rail of an anchored matrix (Move / Rotate / Anchor) and of a part
(after its tools) enters pick mode.

- Each candidate key shows a violet ring 4.5 px across (6 px on hover, 24 px hit area). Major
  targets (parts and the half origin) show a 7.5 px ring with a cross (9 px on hover, 36 px hit
  area), and their label is always on. The current anchor is filled with a white dot.
- Hovering a candidate previews, in amber, where the item would **move**, per the rule in
  [04](04-re-anchor-moves.md): the same carried values placed from the hovered anchor.
- Click commits `SetPartAnchor` or `SetMatrixAnchor` as one Undo entry and raises the toast
  ([06](06-value-popup-and-commit-toast.md)). An anchor that would make a loop shows no ring, or
  is refused with Core's message. Esc or a second press of the token leaves pick mode.
- The anchor popup is pinned to the canvas's top-right corner.

## Seam and first RED / GREEN slice

Seam: a pure candidate projection in `web/crates/layout` (next to `anchor_badge.rs`). It takes
the document, scene and selection and returns candidate keys and major targets with their kind
and current flag, excluding loop-forming targets. The amber preview reuses Core's evaluation of
the proposed op. It does not re-implement placement math in the web layer.

RED: a native test that a thumb-matrix selection lists the main matrix's keys plus the half
origin as candidates, marks its current anchor key, and excludes the thumb matrix's own keys
(loop).

## Acceptance

- [ ] Native candidate tests for an anchored matrix, an anchored part and a detached part (half
  origin current), plus loop exclusion.
- [ ] Mounted tests: enter pick mode, hover (amber preview matching the post-commit pose), click
  (one revision, the item moved, toast shown), Undo, and Esc (no revision).
- [ ] Live-app pass on a split board, on both halves including the dimmed one.
- [ ] DESIGN.md records Anchor pick in the Layout canvas paragraph.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser).
  `check.py test --update-native-baseline` only for deliberate test renames.

## Carried point

- The toast wording follows whatever [04](04-re-anchor-moves.md)'s carried Undo/status point
  settles. Until then, use the prototype's default.
