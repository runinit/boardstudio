# 116: Prepare inert Open repair and warning derivation

Status: ready-for-agent
Type: build
Blocked by: [115](115-prepare-mirrored-edit-transaction.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Prepare source-authoritative Open reconciliation and its warning, without enabling the
production Open hook. Reuse verified kernels; do not invent a second reflection implementation.

## Scope and evidence

`core/src/lib.rs::handle(Open)` at `2478fb151` resolves scripts/layout/components then validates
halves but does not derive linked mirrors. Parent 19 requires altered linked mirror data to be
re-derived with a warning; local/omitted data and ADR-excluded fields remain preserved.

- Prepare private Open draft ordering: existing normalization/resolution, linked derivation,
  cleanup, final validation. Compare relevant mirror inputs before overwriting to detect repair.
- Produce a located warning through an existing finding/report shape. Reopening the repaired
  accepted document is stable and does not repeatedly report a new repair.
- Preserve labels, per-item modes/tweaks/exceptions, key bindings, parameters/side, references
  and copper exclusions. Malformed or ambiguous associations follow 109's compatibility rules.
- No storage format change or archive/browser decode refusal path is added. Production Open
  activation and end-to-end boundary evidence belong to 111 and parent acceptance.

## File ownership

Exclusive: private Open preparation in `core/src/lib.rs` or its existing private child modules,
private mirrored-half helpers and established mirrored-half tests. Existing finding shapes may
be reused; any additional schema/archive/storage path needs root approval/ownership first.
No production Open branch change, visibility widening or generated fixture edit.

## Test-first and acceptance

- [ ] Direct prepared-Open red/green cases cover altered/missing linked mirror geometry,
      ordinary saved data, local/omitted exceptions and malformed association refusal.
- [ ] Repair reports a located warning and stable second preparation; unrelated local/ADR-excluded
      data remains unchanged and the accepted input is not mutated on failure.
- [ ] Existing legacy/unpaired/reuse-board behavior is preserved; runtime Open is still unchanged.

Independent verifier: disable repair, pre-overwrite warning detection and local/omitted protection
separately; assert meaningful failures. Actual save/reopen and native/WASM transcript are final gates.

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
