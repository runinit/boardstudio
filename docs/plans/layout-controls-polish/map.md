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
  [18](../split-board-editing/issues/18-spacing-change-ghost.md). Mostly presentation; the
  behaviour changes are the part rail (07), the new Bend/Shift handles (05), canvas Anchor pick
  (08) and re-anchor moves (04).
- Primary source: local branch `prototype/layout-controls-iconography` (e91865f43), which holds
  `web/crates/layout/prototype/layout-controls-iconography.prototype.html`. Open it with
  `?variant=D`.
- Tracker: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md), Wayfinding operations.

## Tickets

```text
01 Iconography prototype (resolved)
 ├─ 02 Screen-size labels and passive chrome ─────────────┐
 ├─ 03 Handle rail ─┬─ 05 Row/key Shift & Bend             │
 │                  ├─ 06 Value popup + toast ─┐           │
 │                  └──────────────────────────┴─ 07 Part rail ─┐
 └─ 04 Re-anchor moves (Core, combined review) ─────────────────┴─ 08 Canvas Anchor pick
```

Presentation only: 02, 03, 06. Behaviour: 04 (Core op semantics; combined Spec + Standards
review), 05, 07, 08 (web only, existing Core ops).

- [01: Layout controls iconography prototype](issues/01-layout-controls-iconography-prototype.md) — resolved
- [02: Screen-size labels and passive canvas chrome](issues/02-screen-space-canvas-chrome.md) — blocked by none
- [03: Handle rail for the existing tools](issues/03-handle-rail.md) — blocked by none
- [04: Re-anchoring moves the item](issues/04-re-anchor-moves.md) — blocked by none; combined review
- [05: Row and key Shift / Bend handles](issues/05-row-and-key-shift-bend-handles.md) — blocked by 03
- [06: Value popup and commit toast](issues/06-value-popup-and-commit-toast.md) — blocked by 03
- [07: Part rail](issues/07-part-rail.md) — blocked by 03, 06
- [08: Canvas Anchor pick mode](issues/08-canvas-anchor-pick.md) — blocked by 02, 04, 06, 07

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
  pose-preserving retarget and the prototype's "keeps its position" behaviour. Built in
  [04](issues/04-re-anchor-moves.md); matrices already move, so only part Pick changes.
- **Slicing (2026-10-26):** the polish was sliced into 02–08 above. Pure presentation (02, 03,
  06) is kept apart from behaviour (04, 05, 07, 08), and only 04 changes a public Core op (it
  gets the combined Spec + Standards review). [07](issues/07-part-rail.md) carries the parts
  decision, and [08](issues/08-canvas-anchor-pick.md) brings the prototype's Anchor pick mode to
  the canvas.

## Not yet specified

- The re-anchor Undo/status wording, and whether Follow changes or Detach should also move:
  carried in [04](issues/04-re-anchor-moves.md#carried-points-not-decided).
- Compact (≤980 px) token hit areas: carried in [03](issues/03-handle-rail.md#carried-point).
- Whether the row rail keeps the shipped Offset token next to Shift/Bend: carried in
  [05](issues/05-row-and-key-shift-bend-handles.md#carried-point).
- Whether the upper selection strip changes while a part is selected: carried in
  [07](issues/07-part-rail.md#carried-point).
