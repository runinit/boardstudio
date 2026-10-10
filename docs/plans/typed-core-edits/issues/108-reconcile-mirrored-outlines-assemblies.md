# 108: Forward reconciliation of outlines, versions and assemblies

Status: ready-for-agent
Type: build
Blocked by: [107](107-reconcile-mirrored-hosted-electrical-items.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Complete forward derivation for outlines, outline versions and assembly references,
including anchored geometry. Continue to leave the kernel unactivated.

## Evidence and scope

Creation's `mirrored_half.rs::new_board` at `2478fb151` already maps outline identities,
part-anchored geometry and protected gaps; 18/26 record those contracts. Existing mechanical
pair ownership regenerates closure output; reconciliation must not copy `case-closure/*`
or replace linked mechanical features with independent clones.

- Keep stable outline/version identities, provenance and anchors while source geometry,
  versions, assemblies and referenced parts change.
- Use existing outline ownership/resolution behavior for fixed and linked versions;
  source removal cleans only genuinely dependent references/choices.
- Preserve shared document-level assembly definitions where creation shares them;
  reconcile owned references rather than cloning global definitions unnecessarily.
- Derive mechanical effects through the existing pair/feature ownership pipeline.
- Honor exceptions and owned projections; do not activate edit/Open reconciliation.

## File ownership

Exclusive during this slice: `core/src/mirrored_half.rs` and private child modules,
`core/src/mechanical/halves.rs`, outline reconciliation tests in the established mirrored-half
test files. If an existing outline/mechanical helper requires editing, first report its exact
path/symbol and obtain root's ownership assignment; no broad outline subsystem refactor.

## Test-first and acceptance

- [ ] Direct-kernel tests first fail for source outline/version/anchor changes not propagating.
- [ ] Axis and rotated-map cases preserve stable identities and correctly map independent-part
      anchors, protected gaps and linked/fixed version ownership.
- [ ] Removal cleans dependent references while unrelated versions/choices remain intact.
- [ ] Assemblies remain consistent, no closure parts are copied, and a second pass is a no-op.

Independent verifier: mutation-check anchored geometry mapping, stable version/reference
mapping and closure-copy exclusion. Engine ordering and Open repair remain 111's responsibility.

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
