---
status: accepted
---

# Parts live in the half frame; mirrored by default, local by pin

Ticket 12 (12-half-frame-parts-and-mirrored-controller)
extends the ticket-09 half frame from matrix origins to independent parts: a
part on a stored-gap half stores a within-half pose, while the canvas,
constraints and spacing all work board-space. A new controller on a mirrored
half is mirrored on both halves by default, as a Part correspondence on the
mirror link; either copy edits both. The Inspector's "Mirror on both halves"
toggle makes the part local in one Undo entry each way.

- **Storage.** `part_half_offset` is the layout's half offset (`spacing` else
  stored gap, sided by the link); members, companions and legacy pairs have no
  offset. `evaluated_part_pose` adds it for readers (canvas transforms, snap
  and viewport poses, envelopes, outline and export geometry, the constraint
  solver, drag deltas and retunes); `stored_part_pose` converts board-space
  writes back. Rotations are frame-free and pass through untouched.
- **Exactly-once conversion.** Poses convert only at layout-membership
  transitions, never in steady-state sync: `AddPart` converts drops onto a
  gapped half, `SetLayout` reframes entering parts board-to-half (and leaving
  parts half-to-board, except on the unlink transition, where unadopt already
  restored them), and gap adoption converts the pair once. The sync only
  records and ensures; converting there would double-convert.
- **Correspondence, not properties.** Stored-gap pairs mirror independent
  parts through `MirroredItemKind::Part` records, which always name the
  source-layout copy as the source. Legacy property-linked copies are adopted
  into records on first sync; unrecorded source parts materialize a default
  partner (the legacy `right-layout::mirror::left-part` id shape is kept).
  Members stay owned by the matrix slot sync: their stored poses are
  board-space, so the part sync skips them.
- **Either copy edits both.** Drags arrive board-space and clamp so the
  evaluated extent stops at the split axis, then commit through ticket 10's
  spacing rule. The dragged copy converts to storage; the partner reflects
  the half-frame pose (`-x`, mirrored heading). A constraint-driven partner
  back-solves the authority from the partner, like the legacy sync did;
  both copies retune held offsets before the write, so one drag never shifts
  an offset twice.
- **Local by pin.** Making a part local removes the partner, retires the
  record and pins the kept copy with the legacy substitute flag, so the
  default-mirror sync passes it over. Re-mirroring clears the pin and records
  a fresh partner at the reflected pose. The flag — not the matrix exception
  framework, which does not cover independent parts — is reused deliberately
  so legacy and stored-gap pairs pin locals the same way.
- **Local overhang (ticket 07 carried point).** A local part has no partner to
  absorb its extent, so its overhang past the split axis still counts in the
  ticket-10 minimum: the other half moves out symmetrically. A local part at
  the axis pushes the mirror half out by its overhang.

## Alternatives considered

- **Converting in steady-state sync.** Rejected: entry points already store
  half-frame poses, so converting again would double-convert. The sync
  assumes the storage invariant and only records, ensures and materializes.
- **Board-space storage with evaluation-time reflection.** Rejected: spacing
  changes would rewrite every part pose, violating the ticket-09 promise that
  spacing never moves parts within their half.
- **Matrix-exception locals for parts.** Rejected for this ticket: the
  exception framework scopes Part items to members, while the substitute pin
  already means "local" to both the legacy sync and the Inspector vocabulary.
  Ticket 13 may unify the vocabularies.
- **Kind gate (controllers only).** Rejected: the legacy sync mirrors every
  source part with no kind check, so gating the default to controllers would
  silently unmirror diodes and passives on upgrade. Any independent part
  mirrors by default; the local pin is the opt-out.
