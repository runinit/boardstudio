# 107: Forward reconciliation of hosted and electrical items

Status: claimed
Type: build
Blocked by: [106](106-reconcile-mirrored-matrices-layouts-parts.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Extend the unactivated forward kernel to modules, embedded circuits, nets and electrical
configuration, preserving each domain's existing ownership and deletion rules.

## Evidence and scope

Source evidence at `2478fb151`: `modules/placement.rs::remove` removes the module record
only; its host connector is a separate part. `modules/circuit.rs::remove_circuit` removes
owned member parts/pins and unused private nets/definitions, updates dependent memberships
and copper, preserves joined host nets, and refuses locked members. Creation already maps
modules and circuits through their own namespaces (27/28).

- Derive module pose/attachments and circuit-owned definitions, members, private nets and
  references through established correspondences and namespaces.
- Reconcile structural net/electrical references, without copying key bindings, board
  references or copper traces/vias from the source. Cleanup of existing dependent copper
  follows existing circuit deletion rules; it is not permission to mirror copper.
- Add/remove source objects using existing domain closures. Preserve joined host nets and
  module host connectors unless separately removed by their actual owner.
- Honor owner exceptions through the reconciliation seam; do not auto-localize neighbors.
- Keep collisions/refusals atomic in direct-kernel tests; no edit/Open activation here.

## File ownership

Exclusive during this slice: `core/src/mirrored_half.rs` and private child modules,
`core/src/mechanical/halves.rs`, narrowly necessary existing helpers in
`core/src/modules/placement.rs` and `core/src/modules/circuit.rs`, hosted/electrical
reconciliation tests in `core/tests/mirrored_half.rs` or the test file registered by 106.
Do not widen visibility to reach helpers; ask root to resolve that seam first.

## Test-first and acceptance

- [ ] Direct-kernel red/green cases cover module pose and circuit/net edits, additions,
      removals and namespace collisions under a rotated map.
- [ ] Module removal leaves its independently owned connector; circuit removal preserves
      joined host nets and unrelated parts, and respects locked-member refusal.
- [ ] Private definitions/references are neither duplicated nor left dangling; idempotence holds.
- [ ] Key bindings, board references and copper are not mirrored; item exceptions are preserved.

Independent verifier: mutate module mapping, circuit member cleanup and joined-host-net
preservation separately; confirm meaningful failures. Do not turn source cleanup into an
unrelated data migration or infer new circuit ownership.

## Execution and verification gates

This is a child of [19](19-reconcile-mirrored-halves.md); resolving it does not resolve 19.
Core children execute serially with exclusive Core file/API leases. The user approved
independent A/F streams on 2026-10-10; follow the central lane registry and parallel-streams
decision for audited disjoint edits. Every shell/check command remains globally sequential;
activation and timing require exclusive windows. The prerequisite batch and delivered 37
verification slice are complete; root must refresh the bound index and grant this slice.
Use `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, branch `lane/mirror-core`;
report path/branch/status and inspect overlaps before editing. Every shell call has that
absolute `workdir`; commands run one at a time in the foreground.

Read `docs/agents/agent-workflow.md`, `verification-process.md`, `gitnexus-index.md` and
parent 19. Luna 6 high implements test-first and records `implemented (awaiting verification)`;
a fresh Sol 6.1 xhigh independently verifies/mutation-checks, then alone may set `resolved`.
Run bound GitNexus impact before every symbol edit; list callers, processes and risk.
`apply`/`layout::resolve` are known CRITICAL paths: warn before editing. UNKNOWN, stale or
missing answers need source confirmation and honest limitation reporting, not an all-clear.
Use `boardstudio-lane-mirror-core`; root alone refreshes. Run complete `detect_changes` before
any commit; truncated/partial evidence is not clean.

Request every test/build/browser/performance command slot from root before running it.
Affected checks: `repo lint test typecheck`, plus native/WASM boundary parity when its
behavior/transcript changes; targeted red/green and verifier mutations also need slots.
No full check per slice, no `release`, no background checks; Cargo uses 12 jobs.
Maps, ADRs, CONTEXT.md, scripts/check.py and generated goldens remain root-owned.
No merges/push/PR, stash/reset/discard or broad staging; explicit paths only.
Commit only after root permits it, with the actual-author trailer. Outcome records exact
commands/results/timings, mutation failures/restoration and every unrun or blocked gate.
