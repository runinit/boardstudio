# 11: Adaptive gap at Mirrored pair creation

Status: ready-for-agent
Type: build
Blocked by: [10](10-adaptive-spacing-on-commit.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

Mirrored pair setup proposes and stores a Split gap of max(40 mm default, minimum from the
projected key envelopes), and the setup preview shows it before placement. A small 4×6 board
keeps 40 mm; a long thumb cluster that would reach past the axis gets a larger gap (54 mm in the
prototype sample).

## Acceptance

- [ ] The setup preview and the placed pair show the same adapted gap, and it is stored and editable.
- [ ] Existing boards are untouched.
- [ ] Confirm with the user: the max(40 mm, minimum) formula (07 carried point).
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.
