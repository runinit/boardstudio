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

Review follow-ups, specified in 006, run in order one at a time (shared worktree and `target`):

- [006: Review follow-ups for CAD deepening](issues/006-review-follow-ups-spec.md)
- [007: kicad-cli tests pass under an AppImage launcher](issues/007-kicad-cli-under-appimage.md)
- [008: Kernel rejects coordinates Core would never send](issues/008-kernel-coordinate-limit.md)
- [009: One wording for placement-transform errors](issues/009-transform-error-display.md)
- [010: One polygon area function](issues/010-polygon-area.md)
- [011: Kernel tests share one prepared-case fixture](issues/011-kernel-tests-one-fixture.md)
- [012: Proof modules live under qualification](issues/012-proof-modules-under-qualification.md)

## Decisions so far

All five tickets are resolved; each ticket's `## Comments` records what it did and did not do.

- 001 shared only the leaves (import options, digest check, placement check). The host and
  worker query and operand types stay separate by design.
- 002 unified containment, not area; the f32 renderer copy and test oracles stay.
- 003 deleted the production profile builder; tests build through `build_case`.
- 004 was partial; [012](issues/012-proof-modules-under-qualification.md) finished it: the proof
  modules live under qualification and meshing takes an observer.
- 006 specified the review follow-ups 007–012, all resolved.
- 005 kept the Cadrum benchmark and cleanup paths on purpose, commented.
