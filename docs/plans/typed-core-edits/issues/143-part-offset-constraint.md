# 143: A part-offset constraint holds a part along its source part's axes

Status: claimed
Type: build
Blocked by: none
Related: [139](139-generalized-mirrored-constraint-model.md), [144](144-set-matrix-carries-authored-companions.md)
Decision: [ADR-0011](../../../adr/0011-part-offset-is-its-own-constraint-kind.md)

## Request

A user constrains a companion (a diode or an LED) to its key with an Offset constraint, then
splays the key's column. The companion turns with the key but does not swing around it, so it
drifts off the key. Add a **part-offset constraint**: it holds its target at a fixed distance and
angle along the source part's own axes, so a companion held to a key stays where it was relative
to that key whenever the key turns.

## Why this is not a defect in Offset

An Offset constraint is a translation on the board's axes plus a rotation relative to the source:

```
target.at       = source.at + offset            (offset is not rotated by the source)
target.rotation = source.rotation + rotation
```

`constraint_pose` in `core/src/constraints.rs` implements it, [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)
keeps it ("the existing world-plane Offset semantics"), and ticket 139's accepted design says the
same. It is the right meaning for a source that never turns (an anchor) and the only way to say
"this far along the board's axes". A following companion is placed differently: `follow_pose`
rotates its offset by the key's rotation. A part-offset constraint gives a constrained companion
that same behaviour.

## Settled design

Decided in the 2026-10-12 design interview; the reasoning for the representation is in
[ADR-0011](../../../adr/0011-part-offset-is-its-own-constraint-kind.md).

- **A new persisted kind, `part-offset`.** Same fields as `offset` (`id`, `sourcePartId`,
  `targetPartId`, `offset`, `rotation`); `offset` is untouched. There is no format bump:
  `CURRENT_VERSION` stays 5. An older build cannot read the unknown kind and refuses that
  document, where a property on Offset would be ignored and the part misplaced silently.
- **Meaning.** `target.at = source.at + R(source.rotation) * offset` and
  `target.rotation = source.rotation + rotation`. The axes are the source's stored rotation, the
  same axes `follow_pose` uses, with no handedness correction, so following, authored and
  constrained companions agree on a key's axes. Handedness for mirrored matrices stays with 139.
- **Only Offset has a part-axes form.** A Mirror constraint reflects across a board coordinate
  and is unchanged.
- **Defaults.** A constraint authored in the inspector starts as `part-offset` when its source is
  a matrix key and as `offset` for any other source.
- **Switching preserves geometry.** One Core operation (working name
  `ConvertOffsetConstraint { constraintId, to }`) re-expresses the numbers from the source part's
  current pose so the target does not move, keeps the constraint's id and refuses atomically if
  an endpoint is missing. The inspector sends it and does no geometry itself.
- **Inspector.** The constraint section always shows the choice, "Offset on board axes" or
  "Offset along part axes", next to Offset X / Offset Y.
- **No bulk conversion.** Each constraint is converted individually.
- **Mirrored halves: retune only.** A linked-layout edit that moves a constrained companion
  retunes the stored offset correctly for either kind. Reflecting constraints across halves is
  139's.
- **One retune helper.** The two independent retune sites move to one helper in `constraints.rs`
  that knows each kind's axes.
- **139's files stay untouched.** `core/src/constraints/relation.rs` and `endpoints.rs` are not
  edited here (see "Handoff to 139").

## Where the change lands

Verified on `dev` at `32c59a58f`. The compiler lists every exhaustive `match` on `Constraint`;
these are the sites that read an Offset's numbers or construct one:

| Site | Today | Change |
|---|---|---|
| `constraints.rs` `constraint_pose` | board-axes translation | add the part-axes arm |
| `constraints.rs` validation (finite check) | Offset and Mirror | add the new kind |
| `lib.rs` `MoveParts` retune | adds the world delta to `offset` | call the shared helper |
| `matrix/layout.rs` `set_component_pose` | adds world position and rotation deltas | call the shared helper |
| `matrix/layout.rs` `move_keys`, `has_linked_positions` | feeds the above; `matches!` on Offset | include the new kind |
| `matrix/layout.rs` linked-layout back-solve | reflects poses, calls `set_component_pose` | no change beyond the helper |
| `matrix/layout.rs` `companion_mode`, `assembly_placements` | a constraint on the companion wins | recognise the new kind |
| `matrix/deletion.rs` | id remap | add the new kind |
| `mirrored_half/reconcile.rs` `validate_constraints` | existence and finiteness only | add the new kind |
| `web/.../layout_component_edits.rs`, inspector | builds and shows an Offset | choice control, default by source |

## Evidence this ticket starts from

- `core/tests/splay_companions.rs` splays column 1 by 30 degrees with a companion held to its key
  by Offset `(1, -4)` and pins the board-axes contract: its position in the key's frame moves from
  `(1, -4)` to `(-1.13, -3.96)`, about 2.1 mm. The drift is `2 * d * sin(angle / 2)`.
- `Constraint` has no `deny_unknown_fields`, so an extra property would be ignored by an older
  build. No format 6 exists on `dev`.
- Mirrored halves only validate constraints today; nothing reflects an Offset between halves.
- Not verified: the same drift for a constrained companion under a matrix `rotation` change.
  Keys turn there too, so it is likely. Authored companions under a whole-matrix edit are
  [144](144-set-matrix-carries-authored-companions.md).

## Suggested slices

1. **Kind, solver and retune.** The `part-offset` kind and serde shape, `constraint_pose`, the
   shared retune helper at both sites, and tests that a key-held companion keeps its key-frame
   pose through a splay and a drag.
2. **Conversion.** The Core operation, with a test that a part does not move on conversion in
   either direction and that a missing endpoint changes nothing.
3. **Inspector.** The choice control, the default by source, and the conversion wiring.

## Acceptance

- [ ] A key-sourced `part-offset` keeps the companion's position and turn in the key's frame
      through a column splay (both affect modes), a matrix rotation and a key move.
- [ ] `offset` constraints and existing documents resolve exactly as before; the golden transcript
      and existing fixtures are unchanged.
- [ ] Dragging a constrained companion retunes the stored offset in the constraint's own axes for
      both kinds, including through a linked-layout edit.
- [ ] Converting between the kinds leaves every part where it was, keeps the id, and is atomic
      when an endpoint is missing.
- [ ] A new inspector constraint defaults to `part-offset` for a matrix-key source and `offset`
      otherwise, and the choice is always visible.
- [ ] A document using `part-offset` round-trips through serialization, and the unknown-kind
      refusal of an older reader is covered by a test of the wire shape.

## Handoff to 139

139's private relation math (`ConstraintRelation::Offset`, `FramedConstraint`) is already on
`dev`, unreferenced. When 139 compiles legacy records into framed relations it must compile
`part-offset` too: a relation whose offset is rotated by the source's rotation before it is
added. 143 records this and does not edit 139's files or its claimed ticket.

## Alternatives considered

- **A property on Offset, no version bump.** Cheapest, but an older build silently reads it as
  board axes, moves the part and drops the property when it saves.
- **A property on Offset plus format 6.** Loud, but refuses every document the new build saves,
  including ones that never use it, and claims the version step 139 planned to own.
- **Every Offset becomes part-axes.** Cannot be undone (the numbers are identical, so intent is
  lost), changes geometry exactly where the product's core use is, and removes the board-axis
  capability.
- **Part axes only when the source is a matrix key.** Hidden state: the meaning flips if the part
  leaves its matrix.

## Where this should end up

`part-offset` is the Attach relation (rigid in the source's axes) and `offset` is the Align
relation (along the board's axes). A later rename is mechanical. A general geometric constraint
solver is not part of this direction: the graph stays a source-first DAG with atomic cycle
refusal. `Pose2` records direction only, so exact mirrored attachments need a handedness-aware
transform, which belongs with 139.

## Not in this ticket

- Changing the meaning of any existing Offset or Mirror constraint.
- Carrying authored companions through whole-matrix edits ([144](144-set-matrix-carries-authored-companions.md)).
- Reflecting constraints across mirrored halves and slanted axes (139).
