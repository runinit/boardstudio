# 107: Forward reconciliation of hosted and electrical items

Status: implemented (awaiting verification)
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
decision for audited disjoint edits. The current central workflow/registry governs read-only concurrency and complete bounded GLOBAL HEAVY windows; activation and timing retain their scheduled isolation. The standing approved mirror chain requires no repeated per-helper START. Use `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, branch `lane/mirror-core`;
report path/branch/status and inspect overlaps before editing. Every shell call has that
absolute `workdir`; commands run one at a time in the foreground.

Read `docs/agents/agent-workflow.md`, `verification-process.md`, `gitnexus-index.md` and
parent 19. Luna 6 high implements test-first and records `implemented (awaiting verification)`;
ONE fresh Sol6.1 medium independently accepts the coherent unit, reusing adequate source-bound checks and retaining the three required mutation guarantees, then may set `resolved`. The same reviewer follows corrections.
Run bound GitNexus impact before every symbol edit; list callers, processes and risk.
`apply`/`layout::resolve` are known CRITICAL paths: warn before editing. UNKNOWN, stale or
missing answers need source confirmation and honest limitation reporting, not an all-clear.
Use `boardstudio-lane-mirror-core`; root alone refreshes. Run complete `detect_changes` before
any commit; truncated/partial evidence is not clean.

Root grants a complete bounded GLOBAL HEAVY cycle for test/build/browser/performance execution; read-only work and approved disjoint source work need no lease.
Affected checks: `repo lint test typecheck`, plus native/WASM boundary parity when its
behavior/transcript changes; targeted red/green and verifier mutations also need slots.
No full check per slice, no `release`, no background checks; active laptop mode uses Cargo4/native test4/check runner1/Binaryen4, including explicit container forwarding where relevant.
Maps, ADRs, CONTEXT.md, scripts/check.py and generated goldens remain root-owned.
No merges/push/PR, stash/reset/discard or broad staging; explicit paths only.
Commit only after root permits it, with the actual-author trailer. Outcome records exact
commands/results/timings, mutation failures/restoration and every unrun or blocked gate.

## Implementation handoff (2026-10-11)

Luna6 high writer STOPPED with exact `core/src/mirrored_half/reconcile.rs` SHA256 `63c6ff42af3b4b60e0c8b0aaf6a995d4486eaf744808b9367d7fa85384e04368`; this is the only modified source path. Final focused57/57 and Clippy pass on these bytes. The reviewer corrected the formatting binding: its earlier PASS covers6d8cdsource and is not claimed for63c6; the corrected final source needs an actual formatting check. Root synced accepted integration into the lane at `29cb5bb12d2bdb782ef1a395d533c03428686b3e` without changing the dirty107source. Post-sync Core360passed/3ignored/0failed and exact-source WASM check pass. The known old138missing-gap fixture failure is closed by the previously accepted lib.rs correction, not by a107algorithm change. Earlier compile/fixture failures are not counted as behavioral REDs; valid module-pose RED is retained.

Evidence: `/home/chris/.cache/boardstudio-baselines/ticket107-hosted/implementation/acceptance-crosswalk.txt`, source-bound raw RED/debug/Core/WASM/lint logs and `root-presync-binding.json`/`root-postsync-binding.json`. ONE fresh Sol6.1 medium read-only acceptance has started. Original module-mapping, circuit-cleanup and joined-host-net mutation guarantees remain explicit and pending independent disposition/execution. No source acceptance, final graph/commit/integration or full19 completion is claimed. Current central workflow/registry supersedes obsolete per-command/xhigh/12-job text above; laptop4/4/1/Binaryen4 and coherent bounded windows apply.

### Independent review: FAIL, same-writer correction underway

The ONE fresh Sol6.1 medium reviewer completed a bounded source review and stopped. Authority: `/home/chris/.cache/boardstudio-baselines/ticket107-hosted/independent-medium/independent-assessment.md`, SHA256 `383d8e21aeb6d097d4c53f385a569224d8cfd221584dbdbbd4623a9c067b3472`, with `source-evidence-binding.json` and `reviewed.diff` alongside it. Six concrete blockers: durable pair/exception writes instead of private preparation evidence; Add replay cannot converge; source-side module structural references survive projection; existing private circuit-definition changes are ignored; joined-net replacement drops unrelated target host pins; circuit/member/net Local/Omitted states are not honored. Green existing fixtures do not establish these missing cases.

The SAME preserved writer is preparing the review's six minimal independent regressions before behavior corrections; the SAME reviewer will reassess after writer STOP. No new API/schema/visibility,109lifecycle expansion, dropped case or acceptance waiver is authorized. E141 explicitly released its observation window;107 now owns the full bounded correction window. Three original mutation guarantees remain pending for the corrected unit. Status stays awaiting verification; no part of this failed source is integrated.

### Correction evidence qualification

Same-writer corrected regression inputs produced meaningful compiled failures for the review findings. An intermediate focused66/66 GREEN at source `c394713361a18f4e3803a46a408f48fd6d64dba2367ffa3eb7aa9f3d3d0e4e50` is insufficient: removal fixtures manually retired durable source pair rows to bypass validation. The lane explicitly rejected that fixture provenance as acceptance evidence. The same writer is correcting the ordinary source-operation/private-context path without persistent pair edits or a109lifecycle/API expansion. Preserve original raw evidence and same-reviewer follow-up; no source acceptance or integration follows this intermediate GREEN.

### Corrected completed writer handoff

Same writer STOPPED at reconcile.rs SHA256 `f3e56ab602891a44e14f0cbb4092b948f5764a1a03da61a166c58752ac9f3251`, lane HEAD29cb5bb12. Only that source path changed. Exact final-source `correction/test-core-lib02` passed369/3ignored; `check-core02`, `WASM02`, `fmt03`, `clippy-core02` passed. Focused `debug10`66/66 passed on34fb930f before a final let-chain cleanup and is not relabeled as the final hash. Evidence is under the existing `ticket107-hosted/implementation` cache. Authentic ordinary source-operation removal fixtures now preserve full durable hardware bytes; earlier manually retired input GREEN remains invalid. Writer explicitly released HEAVY; SAME independent Sol medium reassessment is active read-only over all six findings and the corrected private-validation boundary. Original three mutation guarantees remain pending explicit disposition/execution. No107source acceptance, graph/commit/integration or full19completion is claimed.

### Same-reviewer corrected-unit FAIL

The SAME Sol medium completed read-only reassessment at finalf3e56ab6 and STOPPED. Updated `independent-medium/independent-assessment.md` SHA256 `a089febeb68ed130e2e1c1a83433300c9313fbe6013f166af5a666d72f6c919b`; initial six-finding report and binding/diff are preserved as `initial-*` siblings. Three residual P1 defects remain within the original findings: R1 applied AddCircuit provenance omits complete member/reference/definition/net maps and can retarget unrelated existing state; R2 removal baseline authenticates record existence without exact pair/board/owned-closure authorization; R3 Omitted child tests keep records physically present, while truly absent Omitted members/nets still fail or retain dangling circuit references. The exact final checks remain valid evidence for their covered cases, not acceptance of these missing contracts.

Return to the SAME writer for the review's bounded independent regressions and private corrections while139 owns heavy execution. No new API/model/visibility or109lifecycle scope is granted. Original three mutation guarantees remain pending until a valid corrected unit clears read-only reassessment; run no mutations against this known-failing unit.
