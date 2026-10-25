# 12: Parts in the half frame, split clamp, mirrored controller by default

Status: ready-for-agent
Type: build
Blocked by: [10](10-adaptive-spacing-on-commit.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

Parts on a split half live in that half's frame, so spacing changes never move them within
their half. Dragging a part stops at the split axis ("a part cannot cross to the other half"),
and the drop commits through 10's spacing rule. A controller added to a mirrored half is
**mirrored on both halves by default**, as a linked partner (a Part correspondence on the mirror
link). Either copy edits both, and the Inspector's "Mirror on both halves" makes it local.

## First RED / GREEN slice

Fail a Core edit test. Add a controller to the source half of a mirrored pair and assert that a
linked partner exists at the reflected pose. Then move the source copy toward the axis and assert
that spacing widens and both copies keep their half-frame values.

## Acceptance

- [ ] Dragging either copy edits both; neither crosses the axis.
- [ ] The "Mirror on both halves" toggle is one Undo entry each way; making a part local removes the partner.
- [ ] A local (unmirrored) part at the axis pushes the other half out by its overhang (07 carried point; note it in the ADR).
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.
