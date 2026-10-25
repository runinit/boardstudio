# 10: Spacing adapts on every committed edit

Status: ready-for-agent
Type: build
Blocked by: [09](09-split-half-frame-contract.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

After every committed geometry edit (part drop, move or rotation, matrix growth or shrink, any
field edit), spacing becomes **max(stored gap, minimum)**. It changes symmetrically about the
split axis, in the same Undo entry, and the status names the new spacing. Spacing narrows back
as well as widens, never below the stored gap, and the halves' outlines never touch. A board
that already touches on open is left alone until its first committed edit. Spacing changes never
change the editing focus.

The minimum, as decided in the prototype (outline air gap = 2 × outline margin; each outline
stays on its own side of the axis):

```js
// prototype: envelope = keys + parts per half; need = 4 * margin
deficit = max(need - (minRight - maxLeft), 2 * (maxLeft + margin), 2 * (margin - minRight))
spacing = max(storedGap, smallest gap whose deficit <= 0)
```

The Inspector's split section shows Stored gap (editable), Current spacing, Minimum now,
Outline air gap and Required air gap. Typing a gap below the minimum keeps the stored value, and
the spacing stays at the minimum.

## First RED / GREEN slice

Fail a Core edit test. Grow a mirrored matrix by one inner column, then assert that spacing
widens to exactly the minimum in one Undo entry. Remove the column and assert that spacing
returns to the stored gap.

## Acceptance

- [ ] Widening and narrowing both happen; spacing never goes below the stored gap; outlines never touch.
- [ ] Undo restores the edit and the spacing together.
- [ ] The status names the spacing change ("spacing widened 40.0 → 50.1 mm …").
- [ ] Opening a touching saved board changes nothing until the next committed edit.
- [ ] Confirm with the user, or record in the ADR: the 2 × margin outline air gap.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.
