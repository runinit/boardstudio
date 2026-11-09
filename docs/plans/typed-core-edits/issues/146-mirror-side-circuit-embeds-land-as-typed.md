# 146: Mirror-side circuit embeds land as typed

Status: resolved
Type: build
Blocked by: [112](112-inverse-map-hosted-electrical-edits.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [111](111-integrate-mirrored-half-reconciliation.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

112 refuses an embed made on a mirror half when a circuit member sits off the circuit's x axis or is
turned inside it. The user wants it accepted: embedding a circuit on either half must place its
members exactly as typed, and the other half must reflect them.

## Finding (why it was refused, and why it can be accepted)

- `EmbeddedCircuit` stores **no circuit pose**; members are ordinary parts with their own poses
  (`modules/circuit.rs::embed`). The circuit pose exists only at embed time.
- Forward derivation reflects each member as an independent part, so a mirror embed lands as typed
  exactly when the **source** members sit at the inverse-reflected world poses. 112 only chose the
  source circuit pose and let the definition's local member poses ride along, which is correct only
  for members on the x axis and unturned.
- Nothing about footprints or pads needs reflecting; only the members' **world poses** on the source.

## What this ticket owes

- Carry per-member source pose overrides in the planned `CircuitAddition`: for each member, the
  source world pose that the forward rule maps to the member's typed mirror pose (honour `side`,
  including the back-side x/rotation flip and member side flip `embed` applies).
- 111 applies the overrides atomically with the source embed (a member pose set inside the same
  prepared transaction); the module definition stays shared and registers once, unchanged.
- Remove 112's refusal; keep refusals for genuinely impossible cases (taken identities, Local/Omitted
  hosts).
- Remove `PLACEMENT_TOLERANCE` and its refusal test; replace with an exact-landing test.

## Acceptance

- [x] Red-first: a mirror-half embed with members at local `y != 0` and a nonzero heading, on both
      Front and Back sides and under translated/slanted/rotated maps, lands every member at the
      typed pose and the source half holds the inverse image.
- [x] A subsequent edit from either half round-trips (source move reaches the mirror as typed).
- [x] Shared definition registered once, unchanged; no pad or footprint reflection.
- [x] Mutation check on the override computation and the back-side branch.

## File ownership

`core/src/mirrored_half/inverse/hosted.rs` and its tests only; request ownership before touching
`modules/circuit.rs`.


## Outcome (2026-10-12)

Implemented in `core/src/mirrored_half/inverse/hosted.rs` (tests in `hosted_tests.rs`); no other
source file touched, no visibility widening, no call site.

- `CircuitAddition` gains `member_poses`: source member id to the world pose the source member
  takes after the source embed. It is the exact inverse image (`source_pose`) of the member's
  typed mirror pose, computed with `modules::embed`'s own placement (circuit pose, back-side x and
  heading negation). The member side needs no override: the source embed repeats the same `side`,
  so its flip equals the typed mirror member's, which the forward rule copies.
- 112's refusal and `PLACEMENT_TOLERANCE` are removed; its refusal test is replaced.
- `CircuitAddition` no longer derives `Eq` (`Pose2` has none); `PartialEq` remains.
- The source circuit's own pose is still the inverse of the typed circuit pose (unchanged);
  members ride on the overrides. Definition passes through unchanged and registers once.

Judgement calls and gaps:

- The oracle applies the prepared source embed plus overrides, then the forward pose rule
  (`mirrored_half::mirror_pose`, the `derive::pose` that forward AddCircuit's `project_part` uses),
  and compares with a real `modules::embed` on the mirror half. It does not drive the private
  `reconcile` function: that needs a `Projection` fixture only reconcile's own test module can
  reach, and 147 is editing that file concurrently.
- The round-trip acceptance (a later source or mirror edit) is covered by the exactness of the
  inverse image under the forward rule plus 112's existing member-move tests; no new test builds a
  fully reconciled document.
- Native test baseline: two 112 tests were deliberately replaced (the refusal test and the
  forward-rule landing test), so the baseline drops them and gains one.

Evidence (`.evidence/ticket146/`): red run observed (member_poses empty: 0 against 2). Mutations:
6 killed (back x, back heading, member y, circuit heading in position, member heading, override
not inverse); an empty-overrides mutant fails to compile and was observed red instead.
