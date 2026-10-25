# 18: Ghost of the previous spacing

Status: ready-for-agent
Type: build
Blocked by: [10](10-adaptive-spacing-on-commit.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

When a committed edit changes the spacing, Layout draws a dashed ghost of the keys at the
previous spacing, with a short "+N mm" or "−N mm" note on the axis, until the next edit or
selection change. The view refits to the content in both directions after commits, undo and
redo, and stays still while dragging.

## Acceptance

- [ ] The ghost appears after both widening and narrowing, and clears on the next action.
- [ ] The view grows and shrinks with the content; it does not move during a drag.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.
