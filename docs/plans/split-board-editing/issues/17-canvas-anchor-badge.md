# 17: Canvas anchor badge for the selection

Status: ready-for-agent
Type: build
Blocked by: [15](15-placement-breadcrumb-retire-relations.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

On the Layout canvas, the selection shows what it is placed from. An anchor badge (authored
icon) sits at the anchor point with a short label, its target gets the violet dashed highlight,
and a dashed tie runs to the selection:
- Column N → Column N−1 ("spread, stagger, splay carry"); Column 1 → the matrix origin.
- A row or key → the row or key below it, or the column base.
- An anchored matrix → its anchor key ("follows X, Y, rotation").
- An unanchored matrix → the half origin (default).

It draws on the selected half, including a dimmed one. Parts are added in 13.

## Acceptance

- [ ] Each selection kind draws the right badge, label and target; the badge clears when the selection clears.
- [ ] Mounted browser coverage for column, anchored matrix and key.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.
