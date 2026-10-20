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

All five tickets are resolved; each ticket's `## Comments` records what it did and did not do.

- 001 shared only the leaves (import options, digest check, placement check). The host and
  worker query and operand types stay separate by design.
- 002 unified containment, not area; the f32 renderer copy and test oracles stay.
- 003 deleted the production profile builder; tests build through `build_case`.
- 004 is partial: meshing no longer names `crate::qualification` types, but the proof modules
  and trace hook still live under `profile/`.
- 005 kept the Cadrum benchmark and cleanup paths on purpose, commented.

## Open follow-ups

- Polygon area copies (see 002).
- Through `build_case`, a hole ring at about 1e13 mm panics inside `cavalier_contours` instead
  of returning an error. Core rejects input beyond 1e6 mm, so it is unreachable today, but the
  kernel does not defend against it.
