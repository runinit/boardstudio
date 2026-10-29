# 19: Node-free Ergogen oracle

Status: ready-for-agent
Type: task
Blocked by: [Face-completeness replacement](18-face-completeness-replacement.md)
Parent: [CI wall time map](../map.md)
Related: [Web-split design](04-web-split-design.md)

## Question

Can the Ergogen regression retain its authenticated upstream parser and dependencies, real CLI,
filesystem and Git protections, and byte-identical output without executing Node?

## Work and completion

- After the face-completeness decision, test the most direct viable approach using the real pinned
  sources and dependencies. Document their provenance and licensing boundary; fixture-only or mock
  substitutes do not establish equivalence.
- Compare actual outputs and behavior with the existing required test, including negative cases.
  Keep that test in the gate until the replacement has independently demonstrated parity.
- Record a supported approach or a concrete blocker and its evidence before proceeding to the
  remaining migrations. No push, merge or hosted CI.

## Comments

- 2026-10-29: Ticket 18 is resolved for the local face-replacement decision only. This ticket is
  the next eligible Node-free sequence frontier, unclaimed; the complete browser step and full gate
  are not green and remain in [22](22-node-free-gate-verification.md). Proceeding here follows the
  user-approved face-local parity then Ergogen sequence, not a claim of full Node independence.
