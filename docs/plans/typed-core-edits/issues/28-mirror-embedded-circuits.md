# 28: Mirror embedded circuits in CreateMirroredHalf

Status: claimed
Type: build
Blocked by: [27](27-mirror-modules.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Embedded circuits are refused by ticket 18. Their parts and nets are ordinary host-frame parts and
nets (`circuit.rs` ~:173-195), so the generic loops copy them, but the board-prefix id rule yields
`right/embedded/...`, which breaks the `embedded/<id>/` namespace (`circuit.rs` ~:57, :258), and
the `EmbeddedCircuit` record itself is not copied.

## What to do

- Give circuits their own id rule that keeps the `embedded/<id>/` namespace; copy the
  `EmbeddedCircuit` record with mapped part and net ids; reflect the parts like any others.
- Correspondences for circuits; tests with the embedded-circuit fixture; mirror stays equal
  under edits (with 19).

## Acceptance

- [ ] A board with an embedded circuit mirrors with valid namespaces and circuit records.
- [ ] Mutation checks on the id rule.

## Verification

Core tests, boundary parity, `repo lint test typecheck browser`.
