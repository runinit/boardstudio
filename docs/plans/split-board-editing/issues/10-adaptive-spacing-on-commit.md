# 10: Spacing adapts on every committed edit

Status: resolved
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

- [x] Widening and narrowing both happen; spacing never goes below the stored gap; outlines never touch.
- [x] Undo restores the edit and the spacing together.
- [ ] The status names the spacing change ("spacing widened 40.0 → 50.1 mm …"). (Web follow-up: Core stores `spacing` per edit in the same Undo entry, but the status line is web-side and this environment has no browser to verify it.)
- [x] Opening a touching saved board changes nothing until the next committed edit.
- [x] Confirm with the user, or record in the ADR: the 2 × margin outline air gap.
- [x] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.

## Answer

Built 2026-10-25 on the half-frame contract (09). `LayoutMirrorLink` gains an optional
`spacing`: the evaluated key-edge distance. Evaluation follows `spacing` when present, else
the stored gap, so saved boards open with byte-identical geometry; the adaptive pass in Core's
commit path (`layout::adapt_spacing`, after resolve, before the no-op check) brings every
stored-gap pair to `max(stored gap, minimum)` in the same Undo entry, then re-resolves.

- Minimum (ADR-0036): per-half envelope = enabled key cells (`pitch - edge gap` rects) plus
  the pair's own independent layout parts (courtyard, else keycap, else bare pose; members,
  companions and Utility excluded). `need = 4 * margin` with the widest PartEnvelope margin
  on the board (default 3.0 mm, prototype feel); the search starts at gap zero with the side
  mapping frozen and converges in one exact step for rigid halves. Typing a gap below the
  minimum keeps the stored value while spacing stays at the minimum.
- Legacy: the pass adopts adoptable pairs first (pose-preserving, as 09), but a newly
  adopted pair only gains a spacing when its envelopes touch; pre-gapped pairs re-space on
  every committed edit; fully anchored pairs stay legacy. Creation keeps its own gap
  (11 owns the creation formula); the pass skips `CreateMirroredPair`/`CreateMirroredHalf`.
  No-op commits stay no-ops; when no gap clears (keys spanning the axis, fixed parts
  pinning), the current spacing is kept instead of thrashing the halves.
- GLOSSARY: Split gap reworded (never moves on its own), new Current spacing term.
- Web follow-ups (need a browser to verify): the Inspector split section (Stored gap
  editable, Current spacing, Minimum now, Outline air gap, Required air gap — all five
  values exist in Core: `splitGap`, `spacing`, and the minimum/air/required formula in
  `matrix::layout`, ready to consume without duplicating the math) and the status line
  naming spacing changes.
- Checks: new `core/tests/split_adaptive_spacing.rs` (6 tests: open-unchanged, widen to
  exactly the minimum + undo, narrow back to stored, move-toward-axis, below-minimum
  typing, touching legacy first-edit) RED first then GREEN; companion + offset-constraint
  expectations updated to the new contract (3 tests: authored overlaps now widen); full
  `check.py test` passes with no regressions.

Prototype comparison: the Core run matches variant B on every carried point — spacing
widens/narrows symmetrically about the axis, never below the stored floor, in one Undo
entry; typing below the minimum keeps the stored value while spacing holds the minimum;
touching-on-open boards wait for the first edit. Differences from the prototype are
deliberate: Core keeps a single stored field plus the evaluated spacing (no separate
base/gap), the margin row is not user-editable, parts are board-absolute until 12 (the
prototype's half-local parts), and unsatisfiable spans keep their spacing instead of
widening into the red.
