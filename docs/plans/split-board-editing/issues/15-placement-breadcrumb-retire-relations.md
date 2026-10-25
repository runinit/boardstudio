# 15: Placement breadcrumb and Relationships; retire the Relations tab

Status: ready-for-agent
Type: build
Blocked by: none (decisions in [06](06-decide-relationship-highlighting-home.md), presentation from [07](07-prototype-split-editing-view.md))
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

Above the Inspector fields, the selection shows "How it is placed" as a clickable breadcrumb:
half › anchor reference › matrix › columns › rows › the selection, then dashed "Carries …" chips
for what it carries. Clicking a chip selects that item. A Relationships section beside the
fields keeps the Relations facts (mirror partner, follows, anchors). The Relations Inspector tab
is removed, but its facts and data stay. Hovering a reference marks its parts with the violet
dashed outline, which stays distinct from amber edit previews.

## Acceptance

- [ ] Every fact the Relations tab showed is reachable beside the selection.
- [ ] Chip and fact navigation follows normal selection rules and never moves split focus.
- [ ] Mounted browser coverage for chip navigation and the violet hover; the Relations tab is gone.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.
