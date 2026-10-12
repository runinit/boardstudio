# 143: An Offset constraint can be measured in its source part's frame

Status: ready-for-human
Type: build
Blocked by: none for design; implementation shares a schema window with [139](139-generalized-mirrored-constraint-model.md)
Related: [139](139-generalized-mirrored-constraint-model.md), [111](111-integrate-mirrored-half-reconciliation.md)
Decision: [ADR-0004](../../../adr/0004-document-format-version-and-id-rename.md), [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

A user constrains a companion (a diode or an LED) to its key with an Offset constraint, then
splays the key's column. The companion turns with the key but its position does not pivot around
it, so it drifts off the key. Let an Offset be measured in the source part's own frame, so a
companion held to a key stays where it was relative to that key whenever the key turns.

## Why this is its own ticket

The drift is not a defect in the splay edit. Today an Offset is a **world** translation plus a
rotation that is **relative** to the source:

```
target.at       = source.at + offset            (offset is not rotated by the source)
target.rotation = source.rotation + rotation
```

`constraint_pose` in `core/src/constraints.rs` and `ConstraintRelation::apply` in
`core/src/constraints/relation.rs` implement this. Ticket 139's accepted design keeps it:
"Offset.offset is a world vector, matching existing behavior", with legacy Offset JSON preserved
byte-for-byte. A companion that follows its key, by contrast, is placed by `follow_pose`, which
rotates the offset by the key's rotation. The world frame is right when the source never rotates
(a mounting anchor) and wrong when the source can be splayed (a key).

Changing the meaning of every stored Offset would silently move parts in saved documents, so the
frame becomes an explicit property instead.

## Behavior

- An Offset carries a frame: `world` (today's meaning) or `source` (the offset is rotated by the
  source part's rotation before it is added to the source position).
- A document without the field means `world`. No stored Offset changes meaning.
- Under `source`, the target's position is
  `source.at + R(source.rotation) * offset` and its rotation stays `source.rotation + rotation`.
- Dragging a constrained target retunes the stored offset in the frame the constraint uses: the
  world displacement is rotated by `-source.rotation` for a `source` Offset.
- The inspector shows the frame with the Offset fields and defaults a new constraint whose source
  is a matrix key to `source`; any other source defaults to `world`.
- The constraint contract for a `source` Offset holds after a column splay, a matrix rotation, a
  key move and a mirrored-half reconcile.

## Evidence this ticket starts from

- `core/tests/splay_companions.rs` splays column 1 by 30 degrees with a companion held to its key
  by Offset `(1, -4)`. Its stored contract is world-frame: the companion sits at `key.at + (1, -4)`
  and its position in the key's frame moves from `(1, -4)` to `(-1.13, -3.96)`, about 2.1 mm.
  Drift is `2 * d * sin(angle / 2)`. That assertion is the one this ticket changes for `source`
  Offsets.
- Authored and following companions already keep their pose relative to the key through a splay
  (the carry in `core/src/matrix/placement.rs`). They are not part of this ticket.
- Not verified: the same drift for a matrix `rotation` change. Keys turn there too, so it is
  likely, but no test covers it yet.

## Constraints on the design

- **Schema window.** 139's format 6 is activated only with 111. An optional `frame` that defaults
  to `world` can ride in that same version. Decide with ADR-0004 whether an older build must
  refuse a document that uses `source`, since reading it as `world` would misplace parts.
- **139's framed relations.** 139's endpoint views and this frame are different things: a view
  maps an endpoint across a mirror, the frame says which axes the offset is measured on. The
  design must say how a `source` Offset composes with a mixed linked/Local endpoint, and how a
  reflection across a slanted axis transports it (a reflected frame flips handedness).
- **One lookup.** 139 routes retuning through one driving-constraint helper in `lib.rs` and
  `matrix/layout.rs`. The frame has to be applied there, not in each caller.
- **Mirror reconcile.** `mirrored_half/reconcile.rs` reflects Offset constraints between halves;
  its reflection of a `source` Offset needs literal geometry oracles for the vertical axis, the
  horizontal axis and a slanted axis.

## Acceptance

- [ ] A key-sourced `source` Offset keeps the companion's position and turn in the key's frame
      through a column splay (both affect modes), a matrix rotation and a key move.
- [ ] A `world` Offset and a document with no `frame` field resolve exactly as before, and
      existing fixtures and the golden transcript are unchanged.
- [ ] Dragging a constrained companion retunes the stored offset in the constraint's frame.
- [ ] A mirrored half reflects a `source` Offset to the geometry its literal oracle states, from
      either half.
- [ ] The inspector shows the frame and sets the default described above.
- [ ] Serialization round-trips the frame, and the version and migration choice is tested.

## Alternatives considered

- **B: every Offset becomes source-frame.** Cheapest to build (one meaning, no field, no
  inspector control) and it makes all three companion modes behave alike. Rejected as the first
  step for three reasons.
  1. *It cannot be undone.* A stored world-frame Offset and a source-frame Offset are the same
     numbers. Once rewritten, nothing records which a user meant, so a later return to C is
     impossible. C to B stays possible: deprecate `world` for key sources once it is shown unused.
  2. *Geometry changes only where the product's core use is.* A source with rotation 0 resolves
     the same under both frames, so only rotated sources differ: splayed columns and rotated
     matrices. A geometry-preserving rewrite would need solved poses, and 139's accepted design
     forbids re-expressing offsets during raw migration.
  3. *It removes a capability.* A world-frame Offset is the only way to say "this far along the
     board's axes from that part", for example to keep diodes in an axis-aligned row down a
     splayed column. Under B that needs a separate anchor part, or a new relation.
- **B': source-frame only when the source is a matrix key.** No field, but the frame becomes
  hidden state: it flips if the part leaves its matrix, and it still changes the meaning of every
  stored key-sourced Offset. Rejected.
- Costs C adds over B: the `frame` field, one branch at each retune site (`lib.rs` MoveParts,
  `matrix/layout.rs` set_component_pose), `ConstraintRelation::apply` and `inverse`, an inspector
  control and a version decision.

## Where this should end up

The `frame` field is the first step toward two named relations, so the model should not grow a
third frame flag later:

- **Attach**: the target is rigidly fixed in the source's frame (offset, turn and, once poses can
  carry it, handedness). A generated companion, an authored companion carried with its key and a
  constrained companion are all this relation, differing only in who authored it.
- **Align**: the target is placed along the board's axes from the source (today's `world`
  Offset), for axis-aligned rows and datum placement.

When the model is next opened for change, `frame: source` becomes Attach and `frame: world`
becomes Align, which is a mechanical migration. A general geometric constraint solver is not part
of this direction: the dependency graph stays a source-first DAG with atomic cycle refusal.
`Pose2` records direction only and cannot represent reflection (`constraint_pose` says so), so
exact mirrored attachments need a handedness-aware transform; that belongs with 139.

## Not in this ticket

- Changing the meaning of any existing Offset.
- Carrying authored companions (done) or following companions (already correct).
- Mirror constraints and the slanted-axis model, which stay with 139.
