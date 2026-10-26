---
status: accepted
---

# Parts anchor with the column vocabulary; mirrors share values through a mapped target

Ticket 13 ([13-part-anchoring-column-vocabulary](../plans/split-board-editing/issues/13-part-anchoring-column-vocabulary.md))
gives independent parts the ticket-07 column vocabulary: an anchor key with a
Follow subset, Spread/Stagger/Splay carried from that anchor, and Offset/
Rotation for the part alone. A part whose anchor key belongs to its own
layout derives its evaluated pose from that key, so editing the anchor column
or key moves the part without rewriting its stored values; a dropped part
anchors to the nearest key of its half and keeps the drop point, upright.
Detached parts keep ticket-12 behavior exactly.

- **One optional anchor.** `Part.anchor` is `None` for a detached part, which
  evaluates from the half origin as before. `Some(PartAnchor)` names the
  anchor key (`matrixId`, `row`, `column`), the Follow subset, the carried
  Spread/Stagger/Splay and the local Offset/Rotation. While anchored, the
  stored pose is only refreshed by part ops; anchor edits move the part, so
  the stored pose goes stale and readers must keep using
  `evaluated_part_pose`. An anchor that stops resolving (removed key or
  matrix) holds the stored pose instead of failing evaluation.
- **Retargets (Pick moves; Follow and Detach hold the pose).** Follow changes
  (`SetPartAnchorFollow`) and Detach (`DetachPartAnchor`) re-solve the carried
  and local values from the current evaluated pose, each in one Undo entry.
  Pick (`SetPartAnchor`) instead keeps the stored carried and local values and
  places the part freshly from the new anchor key (target and Follow subset
  come from the Pick), so the part moves — see the layout-controls-polish
  [re-anchor decision](../plans/layout-controls-polish/issues/04-re-anchor-moves.md).
  Typed value edits (`SetPartAnchorValues`), drops and drags move the part by
  design: a drag re-solves Spread/Stagger from the clamped drop, keeping
  rotation; typed X/Y edits start from the evaluated pose for anchored parts.
  All four refuse locked parts; retargets additionally refuse members,
  companions, driven parts, cross-layout targets, missing keys and
  constraint-driven keys, naming the missing counterpart instead of silently
  retargeting when a mirrored partner has no counterpart matrix.
- **Mirrors share half-frame values.** Both copies carry the same anchor
  values; the partner's target maps to the counterpart key in its own layout
  with the matrix relative-negate rule (Spread, Splay, Offset.x and Rotation
  negate; Stagger and Offset.y stay; non-controllers keep the shared mirrored
  half turn, so anchored copies agree with ticket-12 reflection). Either copy
  edits both through the source-layout authority, which converts a
  mirror-side edit back before fanning out. A part whose anchor matrix has no
  counterpart in the partner layout cannot mirror: the partner stays detached
  at the reflected pose. A separately driven part keeps its solver-owned pose:
  constraints win over the anchor.
- **Search-safe key poses.** The anchor follows the evaluated cell, which
  rides its half through spacing — member stored poses go stale inside the
  adaptive minimum search, where only the temporary gap moves — except for a
  Local or explicitly overridden member, which holds its final pose and is
  followed frozen. Drops validate a preset anchor like a Pick and refuse a
  stale key instead of falling back silently.
