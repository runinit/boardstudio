# 114: Classify compatibility and constraint translation

Status: needs-info
Type: build
Blocked by: [113](113-inverse-map-reviewed-outline-edits.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Finish the inverse-operation coverage ledger and resolve compatibility/constraint behavior
before preparing its private adapter. This draft is not implementation-ready for constraints.

## Explicit technical design gate

At `2478fb151`, constraints have no correspondence kind and mirror-axis constraints encode
vertical/horizontal axes. Arbitrary rotated pair maps and linked/local endpoint mixtures have
not been shown representable. Root must review a source-grounded design before implementation;
no schema/visibility widening, new public payload or silent unsupported pass-through is authorized.
Determine which operations delegate existing behavior and which require canonical endpoint/axis
handling. Report exact unrepresentable cases and affected parent acceptance rather than guessing.

## Remaining inventory families

`SetConstraint`, `RemoveConstraint`; `SetLayout`, `CreateMirroredPair`;
`ApplyMechanicalEdit`, `SetMechanical`, `SetCase`; `EditKeymap`, `SetKeyBinding`;
`CreateMirroredHalf`, `ReplaceDocument`. Check every 40-variant entry in 110 has an owner.

- Delegate typed mechanical pair semantics once. Raw whole configurations require explicit
  compatibility classification; they are not generic point-map requests.
- Preserve bindings on the targeted half (ADR exclusion), creation labels/map validation,
  and source-authoritative ReplaceDocument reconciliation rather than inferring intent from diffs.
- Keep native/legacy same-board layouts; refuse overlapping link systems without silently unlinking.
- Constraints must use the canonical operation throughout apply and downstream resolution.
  Cover linked/linked, local/local and mixed endpoints, rotated maps and durable IDs explicitly.
- After root's design review, prepare only approved private helpers; activate nothing before 111.

## File ownership

Read-only design inventory: `core/src/model.rs`, `core/src/lib.rs`, constraint and layout modules.
Once root grants the design, exclusive writer scope is private mirrored-half translation modules,
association lookup in `core/src/mechanical/halves.rs`, and established mirrored-half test files.
Any constraint implementation/schema file needs separate explicit ownership before editing.

## Test-first and acceptance

- [ ] Root-reviewed constraint/compatibility design is recorded, with representability limits.
- [ ] Every remaining variant is translated/delegated/deliberately unchanged/unsupported with
      reason; applicable unsupported parent behavior remains a blocker.
- [ ] Approved private adapter cases have expected red/green tests for endpoints/axes,
      native/legacy conflict, bindings, raw compatibility and import/recovery policy.
- [ ] No production behavior is enabled and no independent legacy data is linked implicitly.

Independent verifier: mutate one constraint conversion/delegation, legacy ownership guard and
ADR-excluded binding pass-through. Do not claim readiness or resolution from inventory alone.

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
