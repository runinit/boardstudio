# 147: Mirror keycap settings and plain nets after creation

Status: ready-for-agent
Type: build
Blocked by: [112](112-inverse-map-hosted-electrical-edits.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [115](115-prepare-mirrored-edit-transaction.md), [111](111-integrate-mirrored-half-reconciliation.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Every item on a mirrored board must be mirrored. Today reconciliation does not project keycap
settings after creation, nor non-circuit (plain) nets, so an edit to either changes one half only
(found in 112's review). The two halves must stay in step.

## Rules

- Keycap settings (board, scalar and legend payloads, matrix keycap choices) and plain nets
  follow their source through linked correspondences, like every other item.
- **Omission is allowed.** An item the user omits from the mirrored pair is absent on the mirror, and
  the nets that exist only because of it are removed with it (ADR-0010's private-net rule: a mirror
  net whose source net has no non-controller pin outside an Omitted item is absent). Local items keep
  their own value on the mirror.
- Key bindings are still never mirrored; legends are copied as written.

## What this ticket owes

- Forward projection for keycap settings and plain nets in the reconciliation kernel, with
  idempotence, exceptions (Local/Omitted) and the private-net rule.
- Inverse mapping already exists in 112; add the round-trip cases it could not test without
  forward projection (mirror-side keycap/net edit reaches the source then the mirror exactly once).
- Net registration on the document's first board only (`SetNet` execution) is a pre-existing
  wrinkle; decide here whether a mirrored net must register on the mirror board.

## Acceptance

- [ ] Red-first: keycap board/setting and plain-net pin edits from either half appear on the other.
- [ ] Omitted keycap/part: no mirror item, and orphaned plain nets are removed, not left empty.
- [ ] Local item keeps its own mirror value across unrelated source edits.
- [ ] Exactly-once, idempotent reconcile; unrelated geometry unchanged; mutation check.

## File ownership

Private mirrored-half reconciliation modules and established tests; no production call site before 111.

