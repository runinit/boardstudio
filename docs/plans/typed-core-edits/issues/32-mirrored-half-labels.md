# 32: Optional labels for the source and the mirror

Status: claimed
Type: build
Blocked by: [18](18-core-create-mirrored-half.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-companion-modes-pads-labels-2026-10-10)

## Request

User, 2026-10-10: Core does not refuse a source that is neither left nor right; it provides a rename option. Ticket
[26](26-mirrored-half-linked-mechanical-and-unchanged-parameters.md) found that a unibody source gives a mirror labelled
half `unibody`, name "Keyboard mirror", role `central`.

## What to do

- Add optional `labels` to the `CreateMirroredHalf` request: `{source: {name, half, role}, mirror: {name, half, role}}`,
  each field optional; absent keeps today's behaviour (source untouched, mirror labelled by the existing rule).
- Validate (non-empty names, known half/role values, the two halves must not both claim the same half and role) with
  located errors and no mutation. Applying labels is part of the same one Undo step.
- Tests: unibody source with labels gives `Left half`/`Right half`; no labels keeps today's result; invalid labels refused.
- Update ticket [21](21-mirrored-half-web-adapter.md): the adapter passes the labels (physical setup's relabelling) instead of a separate edit.

## Acceptance

- [ ] Labels applied atomically with the creation; unlabelled behaviour unchanged.
- [ ] The ticket 26 equivalence test shrinks its pinned difference list by the relabelling rows when labels are passed.

## Verification

`repo lint test typecheck`; wasm boundary parity if the transcript changes.
