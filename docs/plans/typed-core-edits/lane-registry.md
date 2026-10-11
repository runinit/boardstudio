# Lane registry and coordination

Root owns this registry. Read the central copy at
`/home/chris/01_Projects/ts-boardstudio2/.worktrees/tce-matrix/docs/plans/typed-core-edits/lane-registry.md`
before planning a dispatch, changing files, requesting checks or handing work to a verifier. A lane's branch copy
may lag. Confirm current worktree status and index freshness in the granted command window; a snapshot is not a lease.

[Parallel-streams decision](parallel-streams-review.md) is approved: B Core work remains serial; isolated A/F streams
may receive separate edit leases after exact audits and the original 105 verification session ends. A Core-only acceptance
gap does not block audited independent work. Approval does not grant unlisted files or commands.

## Coordinators

The user requires a persistent chat for every lane. Each coordinator owns its lane's planning, bounded tickets,
Luna implementation and fresh Sol 6.1 xhigh verification sessions. Use the Luna-first effort table in
[agent-workflow.md](../../agents/agent-workflow.md#models-and-who-does-what-the-same-in-codex-and-claude)
for new sessions; active sessions keep their settings and scope.
The coordinator does not implement or verify work it delegates. Root owns integration, maps, shared file leases,
command scheduling, batch checks and every GitNexus refresh. Reuse the listed chats rather than starting duplicate owners.

Root: **Orchestration Chat**, `01a1272d-9690-7342-a509-877cc412ec02`.

| Lane | Coordinator chat / ID | Worktree and branch | Current work / gates |
| --- | --- | --- | --- |
| A browser tests | Coordinate browser test lane A / `01a1276a-18ec-76d0-83ed-82d289372b07` | `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-web-sleeps`, `codex/lane-web-sleeps`; own target/CAD cache | Bounded 128 passed fresh independent verification and nine restored mutation variants; committed `3faf4088`, integrated `1ad6b95c8`. Full integration batch passed 401.0 s. Parent 37 remains partial; 38 blocked. No new implementation lease. |
| B mirror Core | Coordinate mirrored-half reconciliation / `01a12740-01ce-7f21-ba6b-39982e260d5c` | `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, `lane/mirror-core` | 105 integrated. 106 remains claimed/incomplete. Guarded prototype hash `480257d6` passed independent rework review and focused execution; nonempty retained-dependency and whole-document repeat proof findings are closed. Final repo/lint/test/typecheck passed 102.0 s. Root graph/partial-progress commit pending; generalization, assemblies, ownership transitions and deletion remain open. |
| C check speed | Coordinate check speed lane C / `01a1276a-2f0a-7d01-b42e-9a16249e8fbc` | Not assigned | HOLD. 11 needs the machine alone; 12 follows A38; 13 is a human decision. |
| D test cleanup | Coordinate test cleanup lane D / `01a1276a-40fa-77d0-935d-c68fefdb47ca` | Not assigned | HOLD. 35 after mirror/recipe integration; 24 closes after cleanup. Root alone regenerates goldens. |
| E web adapters | Coordinate mirrored half web lane E / `01a1276a-5631-7b73-a8d3-9d864b586f5f` | Not assigned | HOLD. 21, 41, 22 follow their B dependencies. Companion placement excludes independent parts. |
| F boards | Coordinate BoardStudio board adaptations / `01a1273f-b992-7133-b8ed-1eae6bbc90da` | `/home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2`, `codex/sofle-adaptations` | 119, 129 and 130 passed bounded independent verification and are integrated. 130 merge `bf464be5e` includes verification record `2d4b03c9`; shared batch at `1ad6b95c8` passed 401.0 s. Physical hardware gates and 118 remain open. No further implementation lease. |

F's earlier `.worktrees/lane-boards-review` branch is committed and integrated; it is frozen, not another active writer.
Other existing worktrees and old ticket chats are not active lane leases. Preserve them and confirm ownership before reuse.

## Dispatch and conflict escalation

1. Coordinator reads this central registry and the ticket dependencies, then proposes the absolute worktree/base commit,
   exact paths, shared symbols/APIs, generated outputs and required commands. Root grants a bounded file and command lease.
   A new lane gets its persistent chat and isolated worktree before implementation; a blocked lane may plan while holding.
2. In the command window, confirm path/branch/status, inspect overlapping edits and the latest integration changes.
   Run GitNexus impact against the uniquely named index before symbol edits. Compare callers/processes with other lanes'
   leases, including shared Core dispatch/model/reflection, catalogue assembly/recipe and browser adapter seams.
3. Escalate an overlapping file, shared API/behavior change, stale base that invalidates a prerequisite, unexpected dirty
   tree or unassigned generated output to root **before editing the contested scope**. Include paths/symbols, base/HEAD,
   index freshness/risk, affected lanes, proposed owner/order and whether checks are running. Root resolves the lease and
   ordering and informs affected coordinators; work outside the contested scope may continue only within its existing lease.
4. Before commit, run complete bound `detect_changes` and compare its actual changed files/processes to the lease.
   HIGH/CRITICAL impact must be reported before edits; UNKNOWN/empty/stale results need source confirmation.
   Partial/truncated analysis is unresolved. An index can report capped call/process coverage even when the delta is complete;
   an absent caller/flow is not proof of safety. Only root refreshes and communicates the new indexed base.
5. Implementer reports `implemented (awaiting verification)`, exact commit/paths, checks, limitations and changed shared seams.
   Coordinator starts a fresh verifier only after the implementer has stopped and the next command window is granted.
   Verifier independently reproduces acceptance/mutations; resolve only the complete ticket scope. Return a handoff with
   commit IDs, clean/dirty status, evidence, open decisions and released leases. Root merges and updates this registry.

Coordinator awareness comes from the shared registry plus root's handoffs, not from independent guesses about other chats.
Root broadcasts a changed lease, dependency, integration base, index refresh or conflict decision to affected coordinators.
Coordinators keep their own lane detail in ticket Outcomes; only root edits this shared registry and maps.

## File and command leases

One foreground shell command runs globally at a time, including reads, commits, builds and browser runs.
Root grants a bounded sequence and waits for explicit COMMAND WINDOW RELEASED before the next owner.
No release is inferred from inactivity. Approved non-shell edits/research may overlap ordinary checks only in
separate checkouts with disjoint inputs; timing measurements require the machine alone.

Current command owner: A, comparing the exact keycaps list and running three sequential clean modules;
B's focused preservation test follows. Root's integration batch has exited successfully.
The full default check at `4fd8f7f3` passed in 535.5 s, including 70 presentation tests and 15 CAD browser checks.
A has identified both publisher composition and mutually overwriting failure/gate fixtures. Publisher composition
is corrected and frozen. A bounded compound held-failure fixture scaffold and runtime adapter regression are now
leased for test-first work in `web/crates/runtime/src/in_process_support.rs` and the test-support facade/adapter tests
inside `web/crates/runtime/src/runtime.rs`, plus existing keycaps tests and128tracking. Existing failure/gate semantics
and visibility remain unchanged; no GateDriver/shared enum/native-stub edits. The intended compiled behavioral RED is confirmed.
Root now permits the reviewed private failure state, capture before await and focused lifecycle regressions. Existing gate/failure facade impacts are CRITICAL.
B's valid two-board materialized fixture passed both validators. Its compiling document-kernel no-op then failed
the intended stale linked-neighbor assertion. An independent fixed geometry oracle is required before implementation
GREEN; local-preservation sensitivity and full106 acceptance remain open. Shared helpers remain read-only.
F129 corrected bounded acceptance passed independent verification at `0c9a1178`, recorded at `b3c8fd48`
and integrated at `8e8f6bfab`. The lower-hole drill mutant failed the intended dimension assertion and exact
production bytes were restored. All F write and temporary mutation leases are released; 118 remains open.
Root owns129 allocation;128 belongs to A and reserved118–127 retain their assignments. Every command requires a window.
Root materialized [131](issues/131-diode-sod123-opt-in-qualification.md) for the opt-in Vishay 1N4148W land-pattern configuration and qualification plan in lane F.
No implementation lease yet. Preserve legacy defaults/output, explicitly extend the strict schema contract,
and retain HIGH registry / CRITICAL Core-schema impact warnings. IDs 120–127 remain reserved separately.
First assignment is baseline-only preparation; producer/qualification work follows in a separate session.
The next A inspector conversion is deferred: the nine-test proposal has only 1.08 s of scheduled waits,
no measured gain, and unresolved negative-case causal proof. Shared helpers have HIGH impacts; keep them unchanged.
Ticket 37 remains partial and 38 remains blocked; deferred work is not completed acceptance.
Root reserves 130 for a read-only audit of the existing Panasonic EVQ-PUC02K SMT side-reset candidate.
Proposed ticket: `issues/130-reset-switch-smd-side-qualification.md`; no ticket/test write or producer mutation
lease is active. Catalogue identity and official drawing evidence must be audited before acceptance/implementation.

### B: Core ticket 106

Initial write lease:

- `core/src/mirrored_half.rs`: private reconcile module/glue only.
- `core/src/mirrored_half/reconcile.rs`: unactivated forward kernel and direct unit tests.
- `docs/plans/typed-core-edits/issues/106-reconcile-mirrored-matrices-layouts-parts.md`.

Existing derive.rs, mechanical/halves.rs, matrix.rs, matrix/layout.rs and creation tests remain read-only.
Before a necessary helper edit, B reports exact symbol/change, necessity, callers/processes/risk for an additional
root lease. No existing visibility widening, model/schema/lib dispatch, hosted/electrical/outline kernel,
lifecycle/activation, 40, 107 or generated-output work belongs to this slice.

The first no-op test compiled, preserved identity/board and failed stale rows 1 versus 2. Its matrix-projection
implementation passed the same focused test in 5.786 s after correcting test imports. Remaining full 106 acceptance needs coherent test-first batches; the first tracer
does not close the ticket. Source additions/removals, ownership/membership, layouts/materialized parts,
local/omitted inputs, per-item settings, changed IDs/idempotence and rotated maps remain required.

The later valid two-board/two-matrix fixture baseline passed after one missing-import setup repair.
`changed_source_matrix_projects_linked_neighbor_after_entry_snapshot` then compiled and failed first on
stale mirror pose `(19, 0) @ 0`, demonstrating missing projection rather than fixture failure. This no-op RED
does not prove local preservation. Source is frozen pending an independently calculated numeric oracle and
the next bounded implementation grant. Raw fixture and baseline/RED logs are in the ticket106 planning cache.

### A: Browser ticket 128

Write lease: `web/crates/keycaps/src/keycaps_settings.rs` only inside `queued_settings_tests`, its local
`mounted_legend`, `Probe`, `host` and their initializer sites; plus ticket128 tracking. Preserve the five original
test names/assertions and all 30 keycaps test names. No production Core, model, catalogue or golden edits. The following bounded runtime test-support extension supersedes the earlier shared-helper exclusion.
Additional phased lease: `web/crates/runtime/src/in_process_support.rs` and only the test-support facade plus
focused in_process_adapter_tests inside `web/crates/runtime/src/runtime.rs`. First add a gate-only compound
fixture scaffold and one behavioral RED test; that RED is confirmed and reviewed private failure-state implementation is now granted.
Preserve existing failure/gate behavior and visibility. No GateDriver enum/native stub or production Core edits.
Existing facade impacts are CRITICAL; stale, lower-bound and UNKNOWN adapter answers remain explicit.

Compound fixture runtime gates now pass: direct adapter request ownership (1/1), adapter module (9/9),
and separate restart/failure regression (1/1). An executor failure restarts and closes the old adapter,
so a canceled later gate cannot establish no leakage; the corrected ownership test uses separate direct
request futures/gates on one adapter and discloses reused transport identity. Runtime mapping coverage remains.
The retirement test is now wired to the compound held-failure fixture. Its focused run passed 1/1
(29 filtered, 0.08 s test time); source is frozen for exact-list/module acceptance and later mutations/verification.

`mounted_legend` impact was LOW with four test callers; test-entrypoint UNKNOWN results were source-confirmed,
not treated as proof of complete independence. Use existing `boardstudio_web_ui_shared::test_wait` dependency.

The dimensions test now observes distinct B/empty operation slots, request sequences, both Completed outcomes,
accepted state/action readiness and original UndoSomeB. Its focused run passed; the other three converted tests have
prior individual evidence. Sol 6.1 max research identified the existing private KeycapsEditsContext.edits signal.
Root authorized test-only capture of exact owner/request retirement and pruning while the reply is held, followed
by exact terminal outcome and a checkpoint capturing render facts. The existing failed-reply fixture may strengthen
the retirement case while retaining original assertions. No production or visibility change is authorized.
Retirement mutation sensitivity and causal ordering still require independent verification; DOM absence, render
counts, runtime completion alone and unproven effect ordering cannot certify retirement.

Durable baseline/failure/pass logs are under `/home/chris/.cache/boardstudio-baselines/ticket128/`.
The passing dimensions log is `keycap-dimensions-operation-completion-v2.log`. Full 128/37 acceptance stays open;
38 still waits. Remaining module/list, required runs/mutations and affected checks must be independently verified.

### F: Socket ticket 119 and planning docs

Implementation/revision write lease: `footprints/tests/mx_socket_qualification.rs` and ticket119 tracking only.
Revision `c5d2626c` passed fresh independent Sol 6.1 xhigh verification. Baseline and restored focused runs passed
4/4; changing one front socket drill from 3.0 to 3.2 mm caused two intended nominal-drill assertion failures.
Exact production bytes were restored and touched; repo/lint passed in 8.480 s and all 77 footprint tests in 7.067 s.
Its earlier manual rename was remediated through GitNexus; that history remains recorded. The verifier record and reviewed planning dependencies were committed separately before integration.
F is frozen; no source or temporary mutation lease is active.

Selected candidate uses existing front/back outer pad width parameters set to 2.55, nonreversible hot-swap,
1.6 mm PCB, F/B at 0/90/300 degrees. Nominal geometry uses numerical epsilon, not inferred reference tolerances.
Default 2.6 stays unchanged and explicitly unqualified against that nominal target; no recipe application or physical
fit claim follows. Independent review must check rotation/down-Y oracle, exact layers/nets, emitted model transforms
and the explicit missing absolute datum/contact/solid-fit evidence.

`footprints/src/generators/ceoloide/switch_mx.rs`, registry and model sources are read-only. A later temporary
pad/drill mutation needs a separate explicit lease, exact-byte restoration plus touch and failing/restored tests.
No F mutation lease is active. The manufacturer PDF/pixels are in the lane's ignored `.scratch/source-evidence/`;
SHA256 `e41ada68576e65b61fd1a2c34c22ccc549f37a736d1cec9eb2e2aff491d19350`.
Pinned Sofle v2 SW26 pose is in `content/layouts/sofle-layouts.json:178`: x96.7, y133.41, rotation300.

F's separate authorized planning-doc lease covers the hardware-candidates report and ticket118. Their existing
unstaged/untracked work must be preserved, reviewed and persisted separately before integration needs those links.
Electrical/firmware/stack and native recipe prerequisites remain open; selected adaptations preserve measured layouts
and use existing case generation for plate, gasket and tolerances.

Maps, this registry, CONTEXT, ADRs and generated goldens remain root-owned. `scripts/check.py` remains C-owned
only with an active explicit lease. C/D/E are preparing; none has an implementation or command lease.

During this batch root's Stop-hook registration remains under the suspended filename recorded in the parallel decision.
Automatic indexing is prevented. Root refreshes in foreground windows with the indexed checkout and inputs frozen,
waits for the actual analyzer exit, and restores registration only after leases end while preserving intervening changes.

## Integration and graph snapshot

Integration source merge is `8e8f6bfab`, adding verified bounded129 (`0c9a1178`, verification `b3c8fd48`) to
verified105 (`09e07a7a`) and bounded119 (`c5d2626c`, verification `4b97b59a`). The new integration batch
check passed at tracker commit `44cb6ab3` in 356.9 s: repo 0.1, tooling 3.0, lint 3.1, build 47.1,
test 54.9 and browser 248.7 s. All 70 presentation tests passed; durable full output is
`/tmp/boardstudio-tce-batch-44cb6ab3.log`. Root captured the merged source index
successfully in 32 s before this metadata commit (`/tmp/boardstudio-129-integration-index.log`);
wrapper exit 1 records the two uncommitted tracker files, not analyzer failure. The105 precommit graph captured six files/36symbols and ten creation flows at HIGH risk. The119 merge graph
captured five files/54symbols at LOW risk. Both deltas were complete. Final root refresh at `da3ef6e7` reported
`boardstudio-tce-matrix: current`; this later documentation record will advance metadata without changing source. Full default check passed source batch `4fd8f7f3` in 535.5 s: repo 0.1, tooling 12.5, lint 9.9,
build 149.8, test 76.2 and browser 287.0 s. All 70 presentation tests and 15 CAD browser checks passed.
Durable log: `/tmp/boardstudio-tce-batch-4fd8f7f3.log`. Independent 105 affected checks passed.

B synchronized at `767b31fa`; its unique `boardstudio-lane-mirror-core` index was CURRENT there before the
106 documentation claim and unactivated source drafts. A uses `boardstudio-lane-web-sleeps`, captured at `f2d180c6`
before claim/test edits. F uses `boardstudio-ts-boardstudio2`, bound to its exact managed path; source was captured before revision c5d2626c committed the same tested bytes; metadata predates that commit. Root integration alias is `boardstudio-tce-matrix`. Stale/UNKNOWN/capped answers require
source confirmation; no bare `boardstudio` or generic hook notice certifies these lane indexes.

Every dispatch rechecks actual HEAD/status/freshness. Ticket 19 stays open through final activation;114 still needs
constraint decisions. Ticket37 and128 remain partial,38 blocked;119 passed bounded independent acceptance and is integrated;118 remains open. Preserve these
distinctions in handoffs and maps.
