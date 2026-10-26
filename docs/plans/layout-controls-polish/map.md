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

## Open fog

- **Parts' carried tools:** should a selected part's rail also carry Spread and Stagger from its
  anchor key, as [ticket 13](../split-board-editing/issues/13-part-anchoring-column-vocabulary.md)
  specifies? The prototype gives parts only Move / Rotate / Anchor. Related: should the
  Matrix / Row / Column / Key mode toolbar change when a part is selected?
- **Matrix re-anchor:** re-anchoring a part keeps its pose. Should re-anchoring a matrix (the
  thumb cluster) also hold still, or keep moving onto the new anchor as it does in the prototype?
