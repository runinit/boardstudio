# 111: Atomically activate prepared mirrored-half reconciliation

Status: ready-for-agent
Type: build
Blocked by: [117](117-prepare-mirrored-preview-history-performance.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Connect verified inert preparation from 105–110 and 112–117 to production in one coherent
activation. This ticket does not build another reconciliation/translation/lifecycle engine
and does not perform parent 19's complete acceptance review.

## Prerequisites and bounded activation

- All preparatory children are independently resolved. 109's nonempty-closure Unlink route
  and 114's compatibility/constraint design must be concretely settled, not waived.
- Wire prepared canonicalization/draft composition before accepted edit publication, Open
  repair before final Open validation, and prepared paired preview isolation/history behavior.
- Add the agreed `SetHalfException`/`UnlinkHalves` EditOperation vocabulary and dispatch to
  already verified lifecycle helpers. Derive required Unlink context internally via 109's
  reviewed callable route; no public hardware fields or visibility widening.
- Keep existing unpaired/legacy/reuse-board behavior. Ensure downstream resolution receives
  canonical operation and final validation sees reconciled state. No intermediate partial
  production activation or per-domain feature flags.
- Stop/report if connection needs new derivation rules, missing operation-family support,
  new context APIs or significant transaction redesign. Those are preparation blockers,
  not extra implementation hidden inside this activation ticket.

## File ownership

Exclusive: bounded call-site/dispatch/vocabulary edits in `core/src/lib.rs` and
`core/src/model.rs`, necessary private helper wiring, a small set of Session/Core smoke cases
in established mirrored-half test files. No new subsystem implementation, storage/schema
migration, preview backup engine, performance harness, golden edits or visibility changes.
Root assigns any additional path before editing.

## Test-first and acceptance

- [ ] Production-seam smoke cases first fail because runtime edit/Open hooks are absent.
- [ ] One representative rotated-map linked edit from mirror, local/Omit/Rejoin/Unlink
      (including nonempty linked closure context), repaired Open and paired preview reaches
      the prepared paths atomically; one Undo/Redo restores both halves.
- [ ] Refusal publishes no partial data; unchanged legacy/unpaired behavior remains covered.
- [ ] No new reconciliation or operation-family logic was required to activate; every helper
      and compatibility decision has independent preparatory verification evidence.

Independent verifier: mutation-check each newly connected runtime route, including canonical
request propagation and refusal publication. Run affected checks in root-granted slots.
Root then schedules a **separate fresh verifier for parent 19's entire acceptance**: all item
kinds/both directions, save/reopen, exceptions/unlink, full preview/history matrix, actual
activated performance and native/WASM boundary evidence. Parent remains open until that
verification holds; activation smoke coverage alone cannot resolve it. Root owns full batch
check, transcript blessing, integration and graph refresh.

## Execution and verification gates

This is a child of [19](19-reconcile-mirrored-halves.md); resolving it does not resolve 19.
The entire 19 execution batch runs **alone**: no independent implementation or heavy checks.
Root must first finish 37, integrate/sync the verified prerequisite lanes, complete the batch
check, refresh the bound lane index, and explicitly grant this slice and its exclusive files.
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
