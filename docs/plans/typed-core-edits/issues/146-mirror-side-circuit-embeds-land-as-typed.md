# 146: Mirror-side circuit embeds land as typed

Status: ready-for-agent
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

- [ ] Red-first: a mirror-half embed with members at local `y != 0` and a nonzero heading, on both
      Front and Back sides and under translated/slanted/rotated maps, lands every member at the
      typed pose and the source half holds the inverse image.
- [ ] A subsequent edit from either half round-trips (source move reaches the mirror as typed).
- [ ] Shared definition registered once, unchanged; no pad or footprint reflection.
- [ ] Mutation check on the override computation and the back-side branch.

## File ownership

`core/src/mirrored_half/inverse/hosted.rs` and its tests only; request ownership before touching
`modules/circuit.rs`.

