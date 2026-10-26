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

- [x] Native tool-list tests (anchored, detached, member, locked: locked parts show no tokens).
- [x] Mounted tests: Move, Rotate and Stagger drags on an anchored part, one Undo entry each.
  The mirrored partner moves per ADR-0038 (Splay mirrors), and a drag on the dimmed half focuses
  that half as in split-board-editing 14.
- [ ] The existing part-drag browser smoke stays green.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser).
  `check.py test --update-native-baseline` only for deliberate test renames.

## Carried point

- The prototype leaves the upper Matrix / Row / Column / Key / Part strip unchanged while a part
  is selected. By default this ticket doesn't change that strip; confirm in the live pass.

## Answer

Independent parts get a canvas rail in part order `[Move, Rotate, Spread, Stagger]`
centred on the part. Move/Rotate edit the local Offset/Rotation and Spread/Stagger
edit the values carried from the anchor key, each drag committing one composed
`SetPartAnchorValues` (one Undo entry). Detached Move drives the shipped
`GestureBegin/Sample/End` path (Core `MoveParts`, one Undo); detached Rotate
commits one narrow `SetPartRotation` (one Undo entry, exact board rotation,
mirror fan-out, locked/member/companion/driven guards; anchored parts keep
using `SetPartAnchorValues::Rotation`). User approval keeps the token visible
and working. Members and companions pass no facts: on canvas they hit-test to
keys and keep the key tools, and a Component context for them draws no part rail.
The strip is unchanged (Position only for parts); outlines were already
accent/violet in CSS and are now pinned by a class test.

- `web/crates/layout/src/part_rail.rs`: native `PartRailFacts` + `part_rail_tools`
  (locked → none; detached → Move/Rotate; anchored → plus Spread/Stagger, minus
  Spread on anchor column 0).
- `layout_transform_toolbar.rs`: `From<PartRailTool>`, `LayoutPartToolOverlay`
  (own part-order mount set; reuses `handle_rail` geometry and the 06 feedback
  reducer/popup/toast; Escape clears preview/gesture; hover docks the current
  value; token press focuses the part's half for 14 parity).
- `layout_canvas.rs::transform_tools` gains the part facts (incl. hidden-handles
  arms); `presentation.rs` computes facts at the mount site and mounts the part
  overlays with anchor-pitch snap.
- DESIGN.md canvas paragraph records the part rail and the outline rule.

Tests: 4 native list tests + 4 Core `SetPartRotation` route tests (exact
commit + Undo/Redo/reload, guards, locked, mirror fan-out); mounted rail order
+ Spread hover value, anchored Stagger (+2 Alt, offset untouched, toast, Undo),
Move (offset, toast, Undo), Rotate (commit matches live popup, Undo), detached
tokens/member-guard, detached Move (gesture engaged, one revision, toast,
Undo), detached Rotate (commit matches live popup, toast, Undo), mirror pair
fan-out + Undo on both halves, half-focus on
press, locked draws nothing, Part strip keeps Position, selected outline accent
vs unselected violet. Full toolbar module 53/53 and the layout_canvas page-bin
14/14 pass; the full layout lib run SIGKILLs mid-suite in this environment
(pre-existing pattern, scoped suites green).
