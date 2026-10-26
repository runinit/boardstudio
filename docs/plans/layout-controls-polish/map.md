# Map: Layout controls polish

Label: wayfinder:map

## Destination

This planning map is finished when the Layout canvas controls (handles, anchor badge, value
feedback) are smaller and refined, and they fit [DESIGN.md](../../../DESIGN.md): the house toolbar
style, fixed screen-size labels, and accent, violet and amber used only for their documented roles.

## Notes

- Polishes the controls from ergogen-layout-parity
  [09](../ergogen-layout-parity/issues/09-inspector-and-toolbar-prototype.md) and split-board-editing
  [13](../split-board-editing/issues/13-part-anchoring-column-vocabulary.md) /
  [17](../split-board-editing/issues/17-canvas-anchor-badge.md) /
  [18](../split-board-editing/issues/18-spacing-change-ghost.md). Behaviour is unchanged; only the
  presentation changes.
- Primary source: local branch `prototype/layout-controls-iconography` (e91865f43), which holds
  `web/crates/layout/prototype/layout-controls-iconography.prototype.html`. Open it with
  `?variant=D`.
- Tracker: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md), Wayfinding operations.

## Tickets

```text
01 Layout controls iconography prototype (prototype) ── resolved
```

- [01: Layout controls iconography prototype](issues/01-layout-controls-iconography-prototype.md)

## Decisions so far

- **01:** variant D ("Rail, refined") approved. Handles become 18 px labelled tokens in one rail
  above the selection, with an Anchor pick mode, a value popup and a commit toast. Parts are
  selectable and use Move / Rotate / Anchor. See
  [01](issues/01-layout-controls-iconography-prototype.md#answer).
- **Parts' carried tools (2026-10-26):** YES — a selected part's toolbar also offers Spread and
  Stagger carried from its anchor key, matching
  [split-board-editing 13](../split-board-editing/issues/13-part-anchoring-column-vocabulary.md).
- **Re-anchor moves (2026-10-26):** NO hold-still on re-anchor — re-anchoring a part, matrix,
  or anything else MOVES it: it is placed from the new anchor using its current settings
  (Spread/Stagger/Splay/Offset/Rotation as applicable). This supersedes ticket 13's
  pose-preserving retarget and the prototype's "keeps its position" behaviour; flag as a
  behaviour change needing its own build ticket when slicing.

## Not yet specified

- Whether the re-anchor move is one Undo entry with status naming the new anchor (default
  assumption to confirm at slicing; not decided).
