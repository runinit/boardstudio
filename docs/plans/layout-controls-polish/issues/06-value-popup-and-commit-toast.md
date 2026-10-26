# 06: Value popup and commit toast

Status: ready-for-agent
Type: build
Blocked by: [03](03-handle-rail.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/layout-controls-iconography` (e91865f43), `?variant=D` (`renderHud`, `toast`).
Review: view-only presentation; RED/GREEN evidence and a diff read.

## What to build

- **Value popup:** a panel with a 1 px `rule-strong` border, radius 8 and the canvas toolbar
  shadow. It shows the field name, the value in 15 px mono 600 and the delta in accent. It docks
  8 px above the rail while a tool is chosen, and sits 18 px off the pointer while dragging.
  Its footer reads "Drag the handle, or type in the Inspector · Esc: close". Esc clears the
  chosen tool.
- **Commit toast:** a panel with radius 8, 12.5 px text and the value in bold, at the top centre
  of the canvas for 1.8 s. It confirms each committed drag as one Undo step: "<b>Spread 19.00
  mm</b> (+1.00 mm) · 1 Undo step". A zero delta raises no toast. It follows the
  `motion-reveal` token, and reduced motion drops the travel.
- Both are view-only: they write nothing to the document or history. `pointer-events: none`.

## Seam and first RED / GREEN slice

Seam: a pure view-state reducer in `web/crates/ui-model` (pattern: `spacing_ghost.rs`). It takes
tool chosen, drag delta, commit and timeout events and returns the popup and toast content. It
is native-tested, and the canvas only renders it.

RED: a native test where choosing Spread and then committing a +1.00 mm drag gives the toast
text "Spread 19.00 mm (+1.00 mm) · 1 Undo step". A zero-delta commit gives no toast.

## Acceptance

- [ ] Native reducer tests: choose, drag, commit, zero delta, Esc and expiry.
- [ ] A mounted test: the popup docks above the rail, follows the pointer during a drag and
  shows the toast after commit with no extra revision. The toast is gone after its timeout.
- [ ] The toast is announced politely (`role="status"`) and never steals focus.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser).
  `check.py test --update-native-baseline` only for deliberate test renames.
