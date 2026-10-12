# 115: Prepare inert mirrored-edit transaction composition

Status: ready-for-agent
Type: build
Blocked by: [114](114-mirrored-edit-compatibility-constraints.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Compose the verified forward kernels, exception helpers and inverse adapters into an inert
transaction preparation path. Final production routing belongs only to 111.

## Scope and evidence

`core/src/lib.rs::{edit,apply}` at `2478fb151` clone/apply/resolve/prune before publishing.
`constraints::resolve_edit` consumes the operation after apply, so the canonicalized request
must reach downstream resolution too. Halves validation must inspect the final reconciled draft.

- Prepare private draft composition: canonicalize request, execute existing operation,
  resolve final authored layout/constraints/components, reconcile all linked domains,
  clean removed associations, validate, and compute actual changed IDs.
- Include exception/Unlink helper composition only through 109's source-verified callable route
  and internally derived context. Do not call a private mechanical dissolution helper directly,
  duplicate its logic, widen visibility or add hardware fields to public Unlink.
- Keep common production edit/apply routing untouched until 111. If existing transaction helpers
  prevent an inert private seam, report the exact route as needs-info before altering architecture.
- Refusals and no-ops produce no partial accepted mutation. Existing legacy/reuse-board paths retain
  behavior; new native pairs do not acquire legacy property-bag links.

## File ownership

Exclusive: private transaction preparation inside `core/src/lib.rs` or existing private child
modules, private mirrored-half helpers, association validation in `core/src/mechanical/halves.rs`,
and transaction-focused mirrored-half tests. Production call sites and publication/history routing
are read-only. No new EditOperation arms, visibility widening, hardware API fields or goldens.

## Test-first and acceptance

- [ ] Direct prepared-draft red/green tests exercise canonical request through downstream
      resolution, final reconciliation/validation and precise source/mirror/membership IDs.
- [ ] Unrelated edits and ReplaceDocument cannot leave dangling association records.
- [ ] Refused preparation leaves accepted bytes unchanged; source/mirror objects are not
      resolved from stale pre-apply inputs; production route remains unchanged.
- [ ] JSON-equivalent `UnlinkHalves {pairId}` preparation follows 109's existing callable
      mechanical edit route with internally normalized context. Nonempty new-board/reuse-board
      closure fixtures preserve effective/stored geometry and exactly-once hole memberships;
      no caller-supplied hardware field or empty-context shortcut is used.

Independent verifier: mutate canonical operation propagation, reconciliation/cleanup order and
final validation separately. History/Open/preview are deliberately owned by 116/117.

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
