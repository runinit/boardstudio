---
status: accepted
---

# Adaptive spacing keeps split outlines apart after every committed edit

Ticket 10 (10-adaptive-spacing-on-commit)
decides the spacing policy ADR-0037 deferred: after every committed edit that
changes the document, each stored-gap pair's current spacing becomes
`max(stored gap, minimum)`, symmetric about the split axis, in the same Undo entry.

- **Two values.** `LayoutMirrorLink` keeps the stored Split gap as the user's floor
  and gains an optional `spacing`: the evaluated key-edge distance. Evaluation
  follows `spacing` when present, else the stored gap, so boards saved before
  this ticket open with byte-identical geometry (no migration on open) and gain
  a spacing on their first adaptive edit. The adaptive pass never rewrites the
  stored gap, so narrowing back always stops at the stored floor. Typing a gap
  below the minimum keeps the stored value; the spacing stays at the minimum.
- **Minimum.** Per half, the envelope is the half's matrix keys (enabled cells,
  `pitch - edge gap` rects) plus that half's independent pair-layout parts
  (courtyard, else keycap, else the bare pose; matrix members, their companions
  and Utility parts excluded). With `margin` the widest PartEnvelope margin on
  the pair's board (default 3.0 mm when the board carries none — the prototype's
  clearance feel), `need = 4 * margin` and
  `deficit = max(need - clear, 2 * (maxLeft + margin - axis),
  2 * (axis + margin - minRight))`; the minimum is the smallest gap whose
  deficit clears. Outline air gap (`clear - 2 * margin`) then always meets the
  required `2 * margin`, and each outline stays on its own side of the axis.
- **Legacy boards.** The pass adopts adoptable legacy pairs first
  (pose-preserving, as ticket 09), but a newly adopted pair only gains a
  spacing when its envelopes touch; stored-gap pairs re-space on every edit.
  Fully anchored pairs stay legacy: with no unanchored matrix there is no
  implied gap to adopt. Creation keeps its own gap (ticket 11 owns the
  creation formula); the pass never re-spaces a new pair.
- **Limits.** The search starts at gap zero with the side mapping frozen from
  the current geometry, converges in one exact step for rigid halves, and keeps
  the current spacing when no gap clears — keys that span the axis, or fixed
  parts pinning an extent, cannot be separated by symmetric widening (parts
  move into the half frame with ticket 12/13). No-op commits stay no-ops:
  the pass is skipped when the edit changed nothing.
- **The 2 × margin rule** (outline air gap ≥ twice the outline margin) is
  recorded here per the ticket's acceptance: comfortable clearance, matching
  the approved prototype. The Inspector's split section (Stored gap, Current
  spacing, Minimum now, Outline air gap, Required air gap) and the status
  line naming spacing changes build on these values on the web side.

## Considered options

- **Rewrite the stored gap to `max(stored, minimum)` (one field).** Rejected:
  the narrowing case then stops at the widened value instead of the user's
  floor — removing a column would never return to the stored gap.
- **Clamp at evaluation (`max` in `half_offset`, no stored change).** Rejected:
  opening a touching board would re-space it immediately (no-migration rule),
  and matrix edits would move the halves under an unchanged stored gap.
- **Adapt legacy pairs on every edit.** Rejected: adoption is pose-preserving
  but spacing is not, so unrelated edits on legacy scaffolding would shift
  halves. Newly adopted pairs gain a spacing only when their envelopes touch.
