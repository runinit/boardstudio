# 26: CreateMirroredHalf mechanical features as linked pair features

Status: claimed
Type: build
Blocked by: [18](18-core-create-mirrored-half.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## Request

[ADR-0010's 2026-10-10 amendment](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md):
the mirror's openings and mounts are linked pair features mirrored through the pair, in
`new-board` and `reuse-board`; mirroring never changes a generator parameter, including
`reversible` and `side`. Ticket 18's first implementation cleared the mirror's openings and mounts
and set `reversible` on mirrored parts. If ticket 18's verification already did this, close
this ticket with a pointer to the commits.

## What to do

- `new-board`: link the source half's openings and mounts to the mirror as `MechanicalFeature`s
  in the pair, mapped through the pair's map; do not clear or clone them.
- `reuse-board`: create the flipped instance, the pair record and the linked features
  (physical setup today clones them and records no pair): migrate physical setup to call
  this operation, so there is one path.
- Copy every generator parameter unchanged; remove any write to `reversible` or `side`. Test
  that all parameters of every reflected part equal the source's, for chiral parts (U1, TRRS,
  reset) and both a reversible and a single-sided design.
- Test linked features: an edit from either half moves both; a local exception stays local.

## Acceptance

- [ ] Linked openings/mounts exist on the mirror in both modes; none cleared, none cloned.
- [ ] No parameter differs between source and mirrored parts (mutation-checked).
- [ ] Physical setup and the Core operation agree on the reused-PCB result.

## Verification

Core tests, boundary parity native and wasm, `repo lint test typecheck browser`.

## Update (ticket 18 verification, 2026-10-10)

Ticket 18's verification already made the mirror's features linked pair features in both modes
and copies every parameter unchanged (commit after `af4c329fd`). Remaining here:

- The equivalence test in `web/crates/catalogue`: on one accepted document, compare
  `propose_with_mirror_pair` + `into_submission(Some(&mounting_hole_template()))` against
  `CreateMirroredHalf` `ReuseBoard` (instances, pair, features, generated `case-closure/*` parts).
- Report every difference the comparison exposes (expected: the `halves` record, topology/transport
  and relabelling, a derived versus supplied map). Do NOT move physical setup onto the Core operation
  here: that is part of the web adapter, ticket [21](21-mirrored-half-web-adapter.md). Scope of 26 is the
  equivalence test and its findings only.
