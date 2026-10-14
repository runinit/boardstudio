# 109b: Mirror-only replacement parts receive pins automatically

Status: claimed
Type: build
Blocked by: [109](109-mirrored-half-exceptions-unlink.md), [111](111-integrate-mirrored-half-reconciliation.md), [151](151-explicit-mirror-only-part-placement.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

A user can Omit an item on the mirror half and place a different part where it was, such as a
different display. That replacement exists only on the mirror half, so nothing derives its MCU
pins. Give it pins at placement, keep every mirrored item on the source's pins, and re-pin the
replacement when a Rejoin takes its pins back.

## Behavior

- **Assignment happens when the part is placed.** Run the existing `electrical::resolve` for the
  mirror board with every mirrored item's assignment locked to the source's pin. The resolver can
  then hand out only pins that are free: those an Omit released and those no item uses.
- **Rejoin releases, then re-pins.** When a Rejoin reclaims a pin that a replacement holds,
  reconciliation already releases that entry (ticket 109). This ticket runs the resolver again so
  the replacement is automatically given another free pin, and reports it when none is free.
- **The source never changes.** Source pins, the source electrical board and the source half are
  read-only inputs.
- Keep key bindings, protected handoff baselines and user locks as they are today.

## Evidence this ticket starts from

109 leaves mirror-authored entries in the mirror electrical board's `assignments` and `locks`
(entries not derived from any source item), drops one that collides with a pin the source holds
for a mirrored item, and locks each mapped source assignment. The saved `removedCopper` and the
Rejoin notice are 109's. This ticket does not change them.

## Acceptance

- [ ] Placing a replacement after an Omit gives it a free pin immediately, never one held by a
      mirrored item, and a pin freed by the Omit is a valid choice.
- [ ] Placing a replacement when no pin is free reports it and changes nothing else.
- [ ] A Rejoin that reclaims the replacement's pin re-pins the replacement, or reports that none
      is free; the Rejoined item has its source pin.
- [ ] Source pins and the source half are unchanged by every case above.
- [ ] Undo of the placement restores the previous assignments exactly.
- [ ] Preview and history behavior follow 111's activation; there is no second assignment path.

Use 111's activated prepared transaction, Save/Open and history routes, and 151's explicit
placement command. Keep automatic assignment in that same transaction. Do not widen the
visibility of the electrical resolver.

## Placement readiness (2026-10-13)

[151](151-explicit-mirror-only-part-placement.md) separates explicit mirror-only placement
from this ticket's existing pin-assignment contract. Normal mirror-side AddPart currently creates
a source Part, so pin verification must use the accepted explicit placement route once 151
settles it. This does not block 111 or authorize a new operation family inside activation.
