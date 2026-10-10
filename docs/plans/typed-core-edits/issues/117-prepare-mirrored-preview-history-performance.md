# 117: Prepare preview isolation, history and performance evidence

Status: ready-for-agent
Type: build
Blocked by: [116](116-prepare-mirrored-open-repair.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Prepare a bounded native-pair preview/history strategy and paired performance harness using
115/116's inert transaction path. Production activation remains 111; parent 19 owns final cost evidence.

## Scope and evidence

At `2478fb151`, `lib.rs::preview_edit` temporarily mutates accepted data with PreviewBackup;
reconciliation may touch far more than the original operation. `edit` publishes one history
snapshot; `history` restores documents. Existing `matrix_performance.rs` alone may not exercise pairs.

- Prepare a private route that uses the existing cloned preview path for native-paired documents
  if complete optimized backup would expand scope. Preserve the optimized legacy/unpaired route.
  Root reviews any alternative; do not invent a second broad rollback mechanism.
- Exercise private transaction publication/history preparation so one successful edit has one
  snapshot and refused/retired/no-op previews publish none. Do not enable operation routing yet.
- Prepare a representative paired-document cost measurement around existing 115 composition,
  alongside the existing performance guard. Record baseline/candidate costs only in root's exclusive
  quiet slot; final activated cost is measured during parent 19 acceptance.
- Prepare additive boundary scenarios using existing harness, with no golden blessing by children.

## File ownership

Exclusive: private preparation helpers in `core/src/lib.rs` or existing private child modules,
preview/history cases in established mirrored-half tests, narrowly additive paired measurement in
`core/tests/matrix_performance.rs`, `core/tests/support/boundary.rs` additive scenarios only.
Production preview/history routing and all generated goldens remain untouched until 111/root.

## Test-first and acceptance

- [ ] Direct helper red/green cases show complete accepted-document/revision/history isolation
      after successful, refused and retired paired previews, including local/mixed operations.
- [ ] Prepared commit and Undo/Redo restore both halves and exception state with one snapshot;
      nonempty-closure Unlink Undo restores the paired snapshot and Redo restores independent
      materialized configurations/IDs. Prepared reopen preserves the independent result.
- [ ] Existing guard plus paired harness can report added cost; no timing claim without an
      exclusive root-granted slot and actual observed measurements.
- [ ] Boundary scenarios are additive and ready for root blessing; no partial runtime activation.

Independent verifier: mutate preview isolation, one-snapshot publication and exception history
restoration. Final production performance/Undo/Open evidence remains a separate parent review.

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
