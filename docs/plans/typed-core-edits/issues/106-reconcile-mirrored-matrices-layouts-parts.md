# 106: Forward reconciliation of matrices, layouts and parts

Status: ready-for-agent
Type: build
Blocked by: [105](105-mirrored-half-derivation-kernel.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Build the forward reconciliation kernel for matrices, keys, layouts and parts, using
stable source/mirror associations. Test it directly; do not wire it into production edits.

## Evidence and scope

Source evidence at `2478fb151`: `lib.rs::apply(RemoveMatrix)` owns matrix parts, layouts
and empty matrix-owned nets; `matrix.rs::removed_members` removes a switch with its
companions, but deleting a companion removes only that assembly;
`matrix/layout.rs::remove_matrix` preserves independent layout components.

- Re-derive linked objects through the pair map with existing identities and source numbering.
- Handle source additions, removals, matrix resizing, memberships and dependent references;
  report the actual changed identities without churning unchanged objects.
- Keep matrix assemblies and materialized parts consistent. Preserve each item's mode/tweak
  through correspondence rebuilds; retain labels, parameters, side and key bindings.
- Honor durable local/omitted association and owner-projection inputs supplied by 109;
  establish the reconciliation input seam now, without exposing lifecycle actions.
- Keep legacy same-board `LayoutMirrorLink` behavior; a matrix cannot have both link systems.
  New native pairs must not use `boardstudio.mirroredComponentSource`.
- Do not remove unrelated independent parts/layout components or reconcile hosted/outline data here.

## File ownership

Exclusive during this slice: `core/src/mirrored_half.rs` and its private child modules,
`core/src/mechanical/halves.rs` for reconciliation helpers, narrowly required
`core/src/matrix.rs` and `core/src/matrix/layout.rs` helpers, matrix/part reconciliation
cases in `core/tests/mirrored_half.rs`. A separate test file may be registered with a
single entry in `core/tests/integration.rs`; root assigns any additional path first.
`core/src/lib.rs` production dispatch remains untouched.

## Test-first and acceptance

- [ ] Direct-kernel tests fail for the expected absent reconciliation behavior first.
- [ ] Source move/rotation, cell placement/splay/resize, part parameters, additions and
      removals produce the expected linked mirror under axis and rotated maps.
- [ ] Switch/companion pose, tweak, stabiliser and fallback contracts remain intact.
- [ ] A second reconciliation is a no-op; IDs and unrelated local values remain stable.
- [ ] Removing a matrix or switch follows existing ownership; independent layout parts survive.
- [ ] Local/omitted test inputs are excluded without changing neighboring states.

Independent verifier: disable forward mapping, one addition/removal cleanup site and mode/tweak
retention separately; each assertion must fail. Engine Undo/Open/preview are deferred to 111.

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
