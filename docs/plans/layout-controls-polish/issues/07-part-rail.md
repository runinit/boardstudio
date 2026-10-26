# 07: Part rail: Move, Rotate, Spread and Stagger on the canvas

Status: ready-for-agent
Type: build
Blocked by: [03](03-handle-rail.md), [06](06-value-popup-and-commit-toast.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/layout-controls-iconography` (e91865f43), `?variant=D`.
Decision: [map.md](../map.md#decisions-so-far) "Parts' carried tools (2026-10-26)"
Review: web-only behaviour on existing Core ops; RED/GREEN evidence and a diff read.

## What to build

An independent part (the MCU or TRRS jack, on either half including the dimmed one) is selectable
on the canvas and gets its own rail. **Move** and **Rotate** edit the part's local Offset and
Rotation. **Spread** and **Stagger** edit the values carried from its anchor key, as in
split-board-editing [13](../../split-board-editing/issues/13-part-anchoring-column-vocabulary.md).
On an anchored part all four commit through `EditOperation::SetPartAnchorValues`. A detached
part shows only Move and Rotate, which commit through the shipped part-drag path, and members/companions keep their matrix's tools. Dragging a part shows the same
value popup and toast as a key. The selected part's outline turns accent, and unselected part
outlines stay violet ([02](02-screen-space-canvas-chrome.md)). The Anchor token is
[08](08-canvas-anchor-pick.md).

## Seam and first RED / GREEN slice

Seam: `transform_tools` gains a part context. A pure part-tool list depends on anchored versus
detached and member status.

RED: a native test that an anchored independent part gets `[Move, Rotate, Spread, Stagger]`, a
detached part gets `[Move, Rotate]` and a matrix member gets its matrix's tools. A mounted test
drags the part's Stagger token +2 mm and asserts one revision whose part anchor Stagger grew by
2, with Offset unchanged.

## Acceptance

- [ ] Native tool-list tests (anchored, detached, member, locked: locked parts show no tokens).
- [ ] Mounted tests: Move, Rotate and Stagger drags on an anchored part, one Undo entry each.
  The mirrored partner moves per ADR-0038 (Splay mirrors), and a drag on the dimmed half focuses
  that half as in split-board-editing 14.
- [ ] The existing part-drag browser smoke stays green.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser).
  `check.py test --update-native-baseline` only for deliberate test renames.

## Carried point

- The prototype leaves the upper Matrix / Row / Column / Key / Part strip unchanged while a part
  is selected. By default this ticket doesn't change that strip; confirm in the live pass.
