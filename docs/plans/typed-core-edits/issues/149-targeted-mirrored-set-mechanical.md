# 149: Give raw SetMechanical an explicit target for mirrored pairs

Status: ready-for-human
Type: build
Blocked by: none
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Design and implement raw `SetMechanical` support for native mirrored mechanical instances,
following the owner's 2026-10-13 choice to support mirror behavior. Ticket 114's refusal is safe
under its original payload but is not the final compatibility contract for parent 19.

## Source facts

- `EditOperation::SetMechanical` currently carries only
  `Option<Box<MechanicalConfiguration>>` (`core/src/model.rs`). Its apply arm replaces the one
  `ProjectDoc.mechanical` slot (`core/src/lib.rs`).
- A physical instance instead stores its own optional configuration, and a native pair stores
  shared construction plus linked mechanical features in `HardwareConfiguration`
  (`core/src/model.rs`, `core/src/mechanical/inputs.rs`).
- `MechanicalEdit` already distinguishes `MechanicalTarget::Board` from
  `MechanicalTarget::Instance` (`core/src/mechanical/change.rs`). The current Case settings caller
  uses `ApplyMechanicalEdit`; no production caller of raw `SetMechanical` was found.
- Therefore the current payload cannot identify which instance a `None` replacement clears, and
  assigning a mirror-side configuration to the single project slot would not update the pair's
  per-instance storage or linked feature records.

## Interface decision required before implementation

Choose how a caller identifies the target of a whole-configuration replacement:

1. **Extend `SetMechanical` with an optional `MechanicalTarget` (recommended).** Old payloads omit
   the field and keep today's board-slot behavior. New board-level calls specify `Board`; pair
   edits specify the exact `Instance`, including for `configuration: None`. Validate that a
   supplied configuration's `board_id` matches its explicit target. Keep the field optional for
   backward compatibility and do not change saved-project format.
2. **Add a separate target-aware replacement operation.** Keep legacy `SetMechanical` unchanged
   and add a clearly named operation carrying `{ target, configuration }`. This is more explicit
   at the operation name, but adds another public edit variant and dispatch path.

Keeping the payload unchanged cannot support a native `new-board` instance replacement safely:
there is no target for a mirror-side `None` and only one board-level storage slot. It would either
mutate the wrong slot or leave the physical pair untouched, so it is not a support option.

## Required behavior after the interface is chosen

- Prepare the entire replacement against the accepted document; mutate only a disposable draft.
  A refusal leaves the accepted document and history untouched.
- A targeted board replacement retains the existing board-slot semantics. Do not write its value
  into `hardware.shared_construction` or an instance slot.
- An instance replacement uses the existing mechanical ownership model: shared construction stays
  pair-owned; board/instance-local data stays local; linked feature values and membership changes
  follow the existing feature rules from either member; Local and Omitted states are preserved.
  Resolve references by identity, never vector position.
- `None` is scoped by the explicit target. For an instance with a linked pair, reuse the existing
  typed disable/unlink lifecycle so the surviving half keeps its visible mechanical data. An
  absent legacy Board target remains the existing no-op.
- A legacy pair without `halves` and a `reuse-board` pair retain existing semantics. A reuse-board
  pair is processed once and is not duplicated into two board configurations.
- Run existing downstream resolution, projection and final pair validation before publishing;
  return actual changed IDs. Keep production activation owned by 111.

## Suggested implementation seam

Use a private replacement plan (`between(accepted, target, requested)` then `apply(&mut draft)` or
one equivalent single entry point). It partitions a raw full replacement into the existing typed
mechanical lifecycle for linked features and construction, while making field ownership and
reference resolution explicit. This is pure in-process computation; no port/adapter is needed.
Do not duplicate geometry reflection or pair exception logic, widen public member visibility, or
route Open through this planner.

## Acceptance

- [ ] Owner chooses the target-aware operation shape above; old board-only serialized requests
      remain compatible.
- [ ] Source- and mirror-side full replacements update the same pair atomically, including shared
      construction and linked feature additions, edits, moves, and removals.
- [ ] Local features remain independent, Omitted features are never resurrected, and references
      resolve by identity or refuse without partial mutation.
- [ ] Explicit instance `None` follows the existing disable/unlink lifecycle; absent legacy `None`
      stays a no-op. Legacy and reuse-board cases are covered.
- [ ] No pair, configuration slot, or changed-ID record is partially updated on refusal.
- [ ] Independent Spec and Standards reviews pass. The preparation remains inert until 111.

## Related

- [114](114-mirrored-edit-compatibility-constraints.md) records the prior unsupported disposition.
- [115](115-prepare-mirrored-edit-transaction.md) owns inert transaction composition.
- [111](111-integrate-mirrored-half-reconciliation.md) owns production activation.
