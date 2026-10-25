# 31: Aggregate reflection and fixtures

Status: claimed
Type: build
Blocked by: [28](28-aggregate-anchor-core.md) (aggregate shape and evaluation)
Parent: [deferred plan](../deferred-items-plan.md) · Research: [deferred-items-research.md](../deferred-items-research.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Plan slices A6 (linked reflection) and A8 (fixtures) only. Does
not change evaluation, Follow, validation, lifecycle, or UI.

## Outcome

Aggregate targets map through the pair correspondence by identity, with
relative-negate rules applying to the resolved aggregate pose rather than per
part (mirror of the whole candidate, ticket-20 transported-filter precedent).
Upstream-pinned average and intersection goldens are generated through
`scripts/generate-ergogen-parity-fixtures.cjs` (spec §10).

## First RED / GREEN slice

Fail a public-route test: linked Add with an average aggregate, asserting the
counterpart resolves the mirrored whole-candidate pose. Seam:
`core/src/matrix/layout.rs::sync` with kernel `FollowBasis`.

## Subsequent acceptance

- Non-origin-axis Follow transport over aggregates; actual-Local part keys in
  linked pairs. Tests mirror ticket 20's (linked Add, transport, Local).
- Average + intersection goldens pinned upstream through the fixture
  generation script; tolerance precedent spec §10 (`1e-6` upstream /
  `1e-9` local). No bulk-regenerated unrelated snapshots.

## Checks

Core integration tests via public routes; affected quick/browser checks;
fixture script run documented. Normal tests do not invoke Ergogen/Node.
