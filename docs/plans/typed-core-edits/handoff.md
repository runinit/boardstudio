# Implementation handoff: matrix prerequisites, then mechanical edits

## Start here

Implement the approved mechanical refactor in
[typed Core edits ticket 02](issues/02-mechanical-settings-patch.md), including its
Confirmed design and Answer. The user approved the complete design on 2026-10-08.
Proceed with implementation using the agreed seams and verification plan.

Work covers its matrix Core prerequisites first, followed by the mechanical ticket,
in the order below.

Repository: `/home/chris/01_Projects/ts-boardstudio2`.
The approved plans were committed on local `dev` in `fe600d97b` (matrix) and
`41176457d` (mechanical). Start a new worktree from the latest local `dev` revision
containing this handoff and both plan commits;
recheck branch, commits, ticket status and existing owners before changing files.

## Execution order

1. Read the matrix [map](../matrix-layout-consistency/map.md),
   [spec](../matrix-layout-consistency/spec.md) and
   [ticket 01 Answer](../matrix-layout-consistency/issues/01-decide-matrix-transform-contract.md#answer).
   Ticket 01 is resolved: it records the approved contract and is not a build task.
   Use those decisions as the implementation contract.
2. Implement and verify
   [matrix 02: SetMatrixPlacement and stagger axis](../matrix-layout-consistency/issues/02-core-set-matrix-placement.md).
   Claim it under the [tracker conventions](../../agents/issue-tracker.md), then
   follow its TDD cases, checks and required reviews. Resolve it only with recorded
   implementation commits and verification evidence.
3. Implement and verify
   [matrix 03: companion placement modes](../matrix-layout-consistency/issues/03-core-companion-placement-modes.md).
   Integrate both Core tickets into the checkout used for mechanical work. Confirm
   their resolved Outcomes and the operation/scene additions in source; status alone
   is insufficient. Retain one owner for shared Core files and transcript generation.
4. Once both prerequisites are resolved and integrated, claim
   [mechanical 02](issues/02-mechanical-settings-patch.md). Follow its implementation
   sequence: identities/migration, typed edit and atomic clearance projection,
   paired features/findings, Case adapters, then integrated verification.
5. Complete the mechanical ticket's full check and both Standards and Spec reviews
   before resolving it. Record commits, executed checks, findings and limitations
   in its Outcome and update the map.

If a prerequisite is already complete, verify its source/Outcome and continue at the
first unfinished step. If another agent owns it, coordinate ownership before touching
its files. Keep the mechanical ticket unclaimed while its prerequisites remain open.

## Scope and safeguards

- Follow [AGENTS.md](../../../AGENTS.md), the approved ticket contracts and the
  tracker. Preserve unrelated user/agent work; stage explicit paths and separate
  overlapping edits. At the last check, unrelated edits remained in `AGENTS.md`,
  `DESIGN.md`, `PRODUCT.md`, `docs/README.md` and `docs/design/project-context.md`.
- Use GitNexus impact checks before symbol edits and graph change analysis before
  commits. Use TDD at the already agreed Core and Session/Core seams. Run browser
  validation for presentation changes and the full check before a PR.
- Keep the user's approved mechanical behavior: bidirectional linked pairs for
  reused or distinct PCBs, explicit per-feature exceptions, retained fit findings,
  explicit legacy conversion and pair-owned construction. The ticket is the detailed
  source of truth; update it if verified source facts require an implementation adjustment.
- Other matrix web/Relations tickets keep their own dependency order and owners.
  This handoff's prerequisite work is matrix Core 02/03; coordinate shared files
  with agents continuing the rest of that effort.
- Keep [included-keyboards ticket 04](issues/04-refresh-included-keyboards.md) as a
  separate follow-up after mechanical implementation and matrix integrated
  verification. It is not part of this implementation handoff.
- Send concise progress updates, including the active ticket, evidence from the
  current TDD cycle, checks completed and any actual blocker.

## Last verified state — 2026-10-08

Matrix 01 was resolved; matrix 02/03 were `ready-for-agent` with no implementation
integrated in this checkout. Source lacked `SetMatrixPlacement`, `staggerAxis` and
companion placement scene data. Mechanical 02 was approved and `ready-for-agent`,
but blocked by those two Core tickets. Recheck this state rather than assuming it
is unchanged when the handoff runs.
