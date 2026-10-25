# 13: Part anchoring with the column vocabulary

Status: ready-for-agent
Type: build
Blocked by: [12](12-half-frame-parts-and-mirrored-controller.md), [17](17-canvas-anchor-badge.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

A part is placed with the same layout vocabulary as a column. It has an anchor key with Follow
X/Y/rotation, plus **Spread, Stagger, Splay** carried from that anchor and **Offset, Rotation**
for the part alone. Pick anchor key and Detach keep the part's pose; Detach places it from the
half origin. A dropped part anchors to the nearest key of its half and keeps the drop point,
upright. Dragging edits Spread/Stagger. Editing the anchor column or key moves the part. This
extends ADR-0034's matrix anchors to parts, which needs an ADR update and a GLOSSARY entry.

Placement math decided in the prototype:

```js
// prototype: A = anchor key pose filtered by follow (or half origin when detached)
base = A + rot({x: spread, y: stagger}, A.r)
pose = step({x: base.x, y: base.y, r: A.r + splay}, offset, rotation)
```

The Inspector groups "Carried from the anchor" and "This part only", then Anchor, Relationships
and the placement breadcrumb. Selecting the anchor key lists "Anchors <part>". The canvas anchor
badge from 17 covers parts.

## First RED / GREEN slice

Fail a Core edit test. Anchor a part to Main R3C6, change that column's Stagger, and assert that
the part moves by the transported amount. Then Detach and assert that the pose is unchanged.

## Acceptance

- [ ] Follow subset, Pick and Detach each keep the pose and are one Undo entry each.
- [ ] Mirrored partners use the same half-frame values; splay mirrors.
- [ ] Confirm with the user: dropping anchors to the nearest key (07 carried point).
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.
