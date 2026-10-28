---
status: accepted
---

# Split halves share one stored gap and each half keeps its own frame

A mirrored pair on one board (a same-board `Layout` mirror link) carries a stored,
editable **Split gap** and each half's matrices are placed in that half's own frame,
offset from the split axis by the gap. Changing the gap moves both halves apart
symmetrically about the axis; matrix origins keep their half-frame values and the
partner still reflects exactly. This is the saved-document contract that tickets
10–13 build on; it decides storage and evaluation only, not spacing policy.

On 2026-10-25 the user approved the split editing prototype
(ticket 07, variant B) whose document model stores parts and keys in half-local coordinates
while a separate display offset positions each half. This ADR records the Core
half of that model for matrices; independent parts stay board-absolute until
ticket 12 moves them into the half frame.

- **Stored gap.** `LayoutMirrorLink` gains an optional `splitGap`: the key-edge to
  key-edge distance between the halves, measured at the half-origin column
  (`2 * (axisX - origin.x) - (pitch.x - edgeGap.x)`). It is set at mirrored-pair
  creation to `max(requested placement, 40 mm default, ticket-10 minimum)`
  (ticket 11)
  and changes through one typed `set-split-gap` edit (or an equivalent
  `SetLayout`), each a single Undo entry.
  The edit refuses a non-finite or negative gap; storage itself only requires a
  finite number so an adopted touching board keeps its honest implied value.
- **Half frame.** A stored gap switches both halves' unanchored matrix origins to
  half-frame values: the board-space half offset (`axisX - gap/2` for the source,
  `axisX + gap/2` for the mirror) is added back at evaluation. Anchored matrices
  already store relative origins and inherit their anchor's offset, so they are
  never rewritten. The mirror keeps the negated half-frame Origin/Rotation, and
  the axis reflection of every evaluated world pose is preserved for any gap.
- **No migration on open.** A document saved without the stored gap keeps its
  board-coordinate placement: it opens, evaluates and exports byte-identically to
  before, and the gap stays absent. The first committed matrix-geometry edit
  adopts the half frame — it stores the implied gap and re-expresses unanchored
  origins without moving any evaluated pose. Fully anchored pairs stay legacy
  until spacing needs them (ticket 10). Unlinking, or clearing a stored gap
  through `SetLayout`, restores board-coordinate origins the same pose-preserving
  way; one mirrored half is unlinked at a time.
- **Compatibility.** The new fields are optional and the format version does not
  change: older documents open, newer gapped documents are refused by older
  builds ([ADR-0004](0004-document-format-version-and-id-rename.md)). PCB, case
  and export consumers read evaluated world geometry, so identical gaps give
  identical output. Outline island grouping compares board-space positions. Web
  adapters round-trip stored origins and submit placement deltas, which are
  frame-invariant; absolute board-space origins are only authored at creation
  and detach, where Core converts. The Inspector keeps showing stored values:
  a gapped half's origin reads as its within-half placement, which ticket 10's
  split section (Stored gap, Current spacing, Minimum) will contextualise.

## Considered Options

- **Symmetric rewrite of board coordinates on every gap change.** Keep storing
  board-space origins and translate both on a gap edit. This keeps evaluation
  untouched but every gap change rewrites stored coordinates, against the
  prototype's rule that spacing never rewrites within-half coordinates.
- **Gap-derived evaluation with board storage and no conversion.** Shift
  evaluated worlds by the difference between the stored and implied gaps.
  Matrix edits then change the implied gap under a fixed stored gap, so an
  unchanged gap would still move the halves — the stored value would lie.
