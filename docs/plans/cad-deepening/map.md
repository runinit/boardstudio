# Map: CAD deepening

## Destination

Each concept on the case-generation and CAD path has one owner: shared wire contracts
between the host and the CAD worker, one polygon predicate module, one solid builder
tested through its production interface, and proof machinery behind its own interface.

## Notes

- Source: architecture review of `dev` at `9099b3ef7` (2026-10-20), after the
  [CAD backend replacement](../cad-backend-replacement/map.md).
- Already done: the **Case boss** rule (wider than its hole) is enforced by both Core
  preparation and the layered kernel (`beb634350`); see [CONTEXT.md](../../../CONTEXT.md).
- Respect [ADR-0012](../../adr/0012-bounded-outline-validation.md) for polygon predicates.
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).

## Order

Tickets 001, 002, 003 and 005 can start in parallel; 004 waits on 003.

- [001: Collision query contract shared by host and CAD worker](issues/001-collision-query-contract.md)
- [002: One polygon containment and area module](issues/002-polygon-predicates.md)
- [003: Kernel tests build through the production case builder](issues/003-tests-through-build-case.md)
- [004: Qualification owns mesh with evidence](issues/004-qualification-mesh-with-evidence.md)
- [005: Remove leftover Cadrum references](issues/005-remove-cadrum-leftovers.md)

## Decisions so far

None yet.
