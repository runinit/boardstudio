# Lane registry and coordination

Root owns this registry. Read the central copy at
`/home/chris/01_Projects/ts-boardstudio2/.worktrees/tce-matrix/docs/plans/typed-core-edits/lane-registry.md`
before planning a dispatch, changing files, requesting checks or handing work to a verifier. A lane's branch copy
may lag. Confirm current worktree status and index freshness before writes; read-only checks need no global lease. Heavy execution requires its explicit slot; a snapshot is not a lease.

[Parallel-streams decision](parallel-streams-review.md) is approved: B Core work remains serial; isolated A/F streams
may receive separate edit leases after exact audits and the original 105 verification session ends. A Core-only acceptance
gap does not block audited independent work. Approval does not grant unlisted files or commands.

## Coordinators

The user requires a persistent chat for every lane. Each coordinator owns its lane's planning, bounded tickets,
Luna implementation and one fresh Sol 6.1 medium review for code or behavior changes; low-risk documentation follows the root-review exception. Use the Luna-first effort table in
[agent-workflow.md](../../agents/agent-workflow.md#models-and-who-does-what-the-same-in-codex-and-claude)
for new sessions; active sessions keep their settings and scope.
The coordinator does not implement or verify work it delegates. Root owns integration, maps, shared file leases,
command scheduling, batch checks and every GitNexus refresh. Reuse the listed chats rather than starting duplicate owners.

Root: **Orchestration Chat**, `01a1272d-9690-7342-a509-877cc412ec02`.

| Lane | Coordinator chat / ID | Worktree and branch | Current work / gates |
| --- | --- | --- | --- |
| A browser tests | L4 — Browser test cleanup (parent #37, deferred) / `01a1276a-18ec-76d0-83ed-82d289372b07` | `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-web-sleeps`, `codex/lane-web-sleeps`; own target/CAD cache | Bounded 128 passed fresh independent verification and nine restored mutation variants; committed `3faf4088`, integrated `1ad6b95c8`. Full integration batch passed 401.0 s. Parent 37 remains partial; 38 blocked. No new implementation lease. |
| B mirror Core | L1 — Typed Core mirror edits (parent #19) / 01a12740-01ce-7f21-ba6b-39982e260d5c | /home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core, lane/mirror-core | [106](issues/106-reconcile-mirrored-matrices-layouts-parts.md) RESOLVED: source16b3234d integrated6c2fd1a525, final817040 source accepted by the same independent Sol medium; cohort passed.107 owns the current correction HEAVY cycle after first fresh review FAIL, same writer/reviewer retained.139 private math source preparation is isolated in lane-mirror-constraints; runtime checks queued. Full19/111 and browser acceptance remain OPEN. |
| C check speed | L6 — Check speed (parent #11, deferred) / `01a1276a-2f0a-7d01-b42e-9a16249e8fbc` | Integration measurement checkout; no new worktree/source changes | User canceled the warmup with Ctrl-C, exit130; no second run or accepted baseline. C STOPPED and explicitly released GLOBAL foreground to ROOT. Full11 remains open; test-speed work is deferred with no C command lease. |
| D test cleanup | L5 — Test maintenance (parent #24, deferred) / `01a1276a-40fa-77d0-935d-c68fefdb47ca` | Not assigned | HOLD. 35 after mirror/recipe integration; 24 closes after cleanup. Root alone regenerates goldens. |
| E web adapters | L3 — Mirror Inspector integration (parent #21) / `01a1276a-5631-7b73-a8d3-9d864b586f5f` | `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-inspector-copy`, `codex/inspector-copy` | Matrix [12](../matrix-layout-consistency/issues/12-inspector-copy.md) passed fresh independent review and the E12+F135 cohort at `78eb5069`; source `58486522`, accepted source `6aabef…`, final test `4602899d…`, verification `/home/chris/.cache/boardstudio-baselines/inspector-copy12/verification-result.json` and `source-review.md`. Prior12 source/test lease released. Root reports clean integration sync. [137](issues/137-export-blocker-explanation.md) RESOLVED after fresh independent Sol 6.1 medium PASS; source fa6b1bc8 merged no-ff at b618ece511339b89ece793711c68d17fa3a113cc. Accepted export_workspace.rs SHA256 0e193336011ff1650b7b4c70eca29d7687142aedbe44539494bbf75a774571b0; compiled mounted WASM RED → GREEN 1/1, repo/lint/typecheck evidence reused. Review: /home/chris/.cache/boardstudio-baselines/export137/independent-review.json. E STOPPED/released with no active command lease; preserve its independently specified synthetic Export regression during demo retirement. E141 focused native1/1 and mounted Chrome1/1 observations completed; HEAVY explicitly released. Separately seeded right protection limits conclusions; same-writer source/fixture correspondence diagnosis continues without product edits. E21/41 minimal Luna medium plan is complete; bounded non-shell planning only, no implementation until the actual Core prerequisites are accepted and no extra plan-review layer. A37 waits/helpers/fixtures remain deferred. |
| F boards | L2 — Demo removal (parent #138) / 01a1273f-b992-7133-b8ed-1eae6bbc90da | /home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2, codex/sofle-adaptations | [138](issues/138-retire-shipped-demos.md) source acceptancePASS by same independent SolMEDIUM after same-writer five-gap corrections; sourcecbe6c792 integratede6ce90b389. Exact67 reviewed paths/hash/deletion states and binarydelta match; root final capture complete128/128symbols67filesHIGH15flows,28liveindexedhashesexact/37deletedabsent/2JSONbound. Final integration batch and generated-output absence passed; corrected test-only source6bad928a integratede3eeb1be5, resumed test/typecheck/browser PASS424.2s.138 RESOLVED. F writer/reviewerSTOPPED, no active source/check lease; demo/Sofle reintroduction deferred. Review and root binding pointers are in138Outcome. The following board delivery records are historical provenance, not current correctness oracles. Ticket136 resolved/integrated at 6163b9221; fresh Sol6.1 xhigh passed six exact-restored mutations, native2, Chrome Library16 and complete source-bound graph MEDIUM14/14 symbols, four files. Full cohort PASS382.2s; log /tmp/boardstudio-cohort-publication136-retry.log; proof binding /home/chris/.cache/boardstudio-baselines/sofle-next-unit/publication136-verifier-final-source-output-binding.json. DATA archive a2509dd893761b9a85ab0ba7085f701653cd8cbe77120de8844c5b76e404f63e unchanged. Choc request-level model/portable-archive/fresh-scene export evidence is COMPLETE/PASS; four current document/scene exports reproduced 464 actual pad nets, token/revision/contour bindings and SHA.extension model packages. Final authority and limited scope are in the delivery queue. The prior stale-contour input finding is CLOSED at request level; no Core bug fix or source edit occurred. The completed DATA cycle and read-only caller trace STOPPED/released. The source-bound trace confirms an existing coordinated whole-selected-matrix Layout Inspector preset route; the earlier per-cell RGB metadata probe used the wrong route and proves no capability bug. Contract: /home/chris/.cache/boardstudio-baselines/sofle-next-unit/assembly-swap56/assembly-preset-caller-contract.md (SHA256 ed8d8edbbc4986ca4baa7b36c67a04d495fb789add310200b408c532bb79f208). The bounded harness accessibility check is COMPLETE/STOPPED and explicitly released; actual Sofle whole-matrix MX → RGB → Choc → restore acceptance is BLOCKED/UNPERFORMED because the existing synthetic harness has no supplied-fixture entry. Coordinator record (not independently reviewed): /home/chris/.cache/boardstudio-baselines/sofle-next-unit/assembly-swap56/existing-inspector-harness-access-limit.json. This establishes a harness boundary, not product failure or capability absence. No automatic verifier, source change, extension grant or Sofle harness-capability ticket follows;137 is the separate app Export issue. Ticket 56 remains PARTIAL/ready-for-human; live UI/human assembly, mirror and physical bench/fit gates remain open. B’s historical wrapper HUMAN HOLD is superseded by current explicit approval. |

F's earlier `.worktrees/lane-boards-review` branch is committed and integrated; it is frozen, not another active writer.

Independent UI lane L7: **L7 — UI design (parent140)**, coordinator `01a12c41-1a51-7101-a555-188f1c85edce`, worktree `/home/chris/.codex/worktrees/2182/ts-boardstudio2`, branch `codex/ui-impeccable`, base53970a404. [Parent140](issues/140-ui-design-impeccable.md) and [plan](../ui-design/plan.md) grant discovery, direct user questions in that chat, bounded UI implementation and independent acceptance; later merge only. UI source work is disjoint from B Core. F retirement test hunks, E future mirror API wiring, Core/CAD algorithms, goldens and shared tracking are excluded. Exact UI paths are recorded per batch. Private dev-loop exception below is user-approved; private provider bootstrap and frontend preview are ready on8182/8183; capped watcherPID/PGID3863727, runtime receipt .scratch/ui-dev/runtime.json. First coherent Tailwind/contextual Layout unit is frozen in layout_toolbar.rs/layout_transform_toolbar.rs plus scoped styling/docs; private visual correction passed at1440/820/390, and formal affected acceptance queues after B release. Shared acceptance suites and unique index bootstrap remain root-scheduled. Preserve main dirty docs.
Other existing worktrees and old ticket chats are not active lane leases. Preserve them and confirm ownership before reuse.

### Current B standing authorization

The user approved the exact removal-only `pub(crate) remove_parts_by_id` wrapper in [106](issues/106-reconcile-mirrored-matrices-layouts-parts.md#current-approved-execution-authority-2026-10-11); `remap_parts` and `PartChanges` remain private. The prior HUMAN hold is lifted, superseding stale hold/START language anywhere in historical records or branch copies. B owns required mirror-chain planning → Luna implementation → ONE fresh Sol 6.1 medium acceptance per coherent unit → dependency-ready continuation. Tightly coupled private slices may share a unit/review with each ticket’s acceptance recorded; routine helpers/tests/fixed-scope children need no repeated root START. Already approved ticket/ADR feature operations/schema are authorized as specified, not a new blanket approval gate. Current explicit approvals/grants survive compaction; root handoff is notification, not implicit STOP. Preserve meaningful regressions/counterexamples/full19 acceptance and111 activation. Root still schedules bounded GLOBAL HEAVY cycles and owns refresh/integration/goldens; read-only commands and disjoint source work need no global lease.

## Dispatch and conflict escalation

1. Coordinator reads this central registry and the ticket dependencies, then proposes the absolute worktree/base commit,
   exact paths, shared symbols/APIs, generated outputs and required commands. Root grants new/cross-lane ownership and a complete bounded GLOBAL HEAVY debugging/affected-check cycle; B selects approved dependency-ready scopes under its standing chain without another per-child START.
   A new lane gets its persistent chat and isolated worktree before implementation; a blocked lane may plan while holding.
2. Before writes/heavy execution, confirm path/branch/status, inspect overlapping edits and the latest integration changes; read-only inspection runs freely.
   Run GitNexus impact against the uniquely named index before symbol edits. Compare callers/processes with other lanes'
   leases, including shared Core dispatch/model/reflection, catalogue assembly/recipe and browser adapter seams.
3. Escalate an actual product/representability decision, cross-lane ownership/conflict, genuinely unapproved API visibility/schema/configuration or destructive user-data change, or concrete failed-acceptance scope to root **before editing the contested scope**. Routine private helpers/tests/fixture ports within the approved contract are not new approval gates; request root-owned source/index refresh when needed. Include paths/symbols, base/HEAD,
   index freshness/risk, affected lanes, proposed owner/order and whether checks are running. Root resolves the lease and
   ordering and informs affected coordinators; work outside the contested scope may continue only within its existing lease.
4. Before commit, run complete bound `detect_changes` and compare its actual changed files/processes to the lease.
   HIGH/CRITICAL impact must be reported before edits; UNKNOWN/empty/stale results need source confirmation.
   Partial/truncated analysis is unresolved. An index can report capped call/process coverage even when the delta is complete;
   an absent caller/flow is not proof of safety. Only root refreshes and communicates the new indexed base.
5. Implementer reports `implemented (awaiting verification)`, exact commit/paths, checks, limitations and changed shared seams.
   Coordinator starts fresh review after completed implementer STOP; read-only review runs freely, with a HEAVY slot only for necessary heavy verification.
   ONE fresh Sol 6.1 medium reviews the completed coherent unit, reusing sufficient checks and targeting actual gaps/important guarantees; record each ticket’s complete acceptance without an automatic review per ticket number. Return a handoff with
   commit IDs, clean/dirty status, evidence, open decisions and released leases. Root merges and updates this registry.

Coordinator awareness comes from the shared registry plus root's handoffs, not from independent guesses about other chats.
Root broadcasts a changed lease, dependency, integration base, index refresh or conflict decision to affected coordinators.
Coordinators keep their own lane detail in ticket Outcomes; only root edits this shared registry and maps.

## File and command leases

L7 has a user-approved private persistent UI dev-loop exception, recorded in [agent-workflow.md](../../agents/agent-workflow.md#standing-lane-authority-and-continuity): isolated initial provider preparation, frontend watcher/rebuild and visual browser inspection may run independently. L7 records its processes/port/private outputs. Full affected/acceptance suites and root index/integration work remain scheduled; timing pauses active UI work explicitly. This exception supersedes older blanket no-overlap instructions for the private loop only.

The user-approved GLOBAL HEAVY policy supersedes every older global-every-shell/read rule in this registry, including historical checkpoints. Read-only shell commands/searches/diffs/graph queries and approved disjoint source work in separate worktrees run freely; one writer per worktree and shared writable-output ownership remain required.

One GLOBAL HEAVY slot covers builds/compilation, test execution, browser, timing and index refresh. Root grants a complete bounded debugging + affected-check cycle and waits for explicit HEAVY SLOT RELEASED before another heavy owner; no release is inferred from inactivity. Commands remain sequential within each lane, resource-heavy runs never overlap and timing requires the machine alone. Carry approved RED → fix → GREEN in the same window without per-command/single-probe handoffs. Complete/restore a running probe before any writer switch; fresh review follows completed writer STOP.

Current HEAVY slot owner: ROOT, final107/139 integration index/tracking capture then frozenUI unique2182graph/sourcecommit. Full107/139 default batchPASS859.9s plus explicittypecheck25.5s,72presentationtests/0failures, exact accepted four source hashes unchanged. L7 scope code/behavior/visualPASS and8mountedtests/WASMpagecheckPASS; unchanged19fileClippybaseline failed/reported, laterUImerge/current-base gates OPEN. B108 claimed/source draftSTOP, five meaningful candidate tests/inert seam ready; completeRED→fix→GREEN/affectedchecks gets next slot after root captures. E141 STOP, two candidate-suppression counterexamples preserved; user connected-outline repro/root cause unproved.139 later solver/lifecycle/activation OPEN. Caps4/4/1/Binaryen4;106/107/138/142 resolved/integrated; root owns graph/integration/tracking; pollingPAUSED.

Active resource mode: **laptop multi-stream**, user-approved2026-10-11. Follow [the workflow resource mode](../../agents/agent-workflow.md#laptop-multi-stream-resource-mode): `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 BOARDSTUDIO_CHECK_JOBS=1 BINARYEN_CORES=4`; one browser instance per lane. Container forwarding gap is tracked in142; runtime affinity capped the initial integration optimizer without interruption. Applies to future/restarted commands and integration checks, superseding12jobs. Lanes run focused/affected coverage; full workspace/native/browser suites run once at integration. Existing HEAVY scheduling and private UI exception remain; completed evidence is reused. Root notified active lanes; deferred lanes inherit this central policy on resumption.

L1/B139 parallel private preparation: root-created `.worktrees/lane-mirror-constraints`, branch `codex/mirror-constraints-139`, baselinee3eeb1be5, coordinated by the existing L1 chat.106closure activates the existing accepted design/initial lease: constraints.rs module declaration, new constraints/relation.rs and endpoints.rs, derive.rs private math reuse only if necessary after impact. Separate writer/worktree from107reconcile.rs. No model/lib/migration/solver/placement/production/browser activation in this unit; later lifecycle/solver stages retain their prerequisites. Source preparation is free; heavy checks queued and one fresh Sol medium accepts the coherent completed unit. Root retains shared tracking/index/integration.

UI source-only critique preparation has five objective packets complete; no browser observation has occurred, and there is no visual-polish completion claim or added delivery gate.

Completed F131 edit lease (released): `footprints/src/generators/ceoloide/diode_tht_sod123.rs` PARAMS/body only; new `footprints/tests/diode_sod123_qualification.rs`; `footprints/tests/generators.rs` strict schema extension/private explicit-false comparator, preserving baseline helpers; narrow `docs/investigations/footprint-generators-rust.md` deviation. No shared API/Registry/Core changes or generated outputs. Existing absent whole-record comparison stays unchanged; explicit false accounts for only the added input flag and compares output/export/net/error payloads exactly. Root owns ticket/map edits. Completed B132 write lease (released): only `core/src/mirrored_half/reconcile.rs` private fixed-membership geometry/view builder and focused tests. Shared matrix/reflection helpers, visibility, schema, activation, topology/add/remove/deletion and generated outputs are excluded.

### Historical command and lease checkpoints

The following records describe prior windows; they do not grant current ownership. The lane table and current lease above take precedence.

Previous command owner: A, comparing the exact keycaps list and running three sequential clean modules;
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
Root owns129 allocation;128 belongs to A and reserved118–127 retain their assignments. That historical every-command restriction is superseded by the current GLOBAL HEAVY policy.
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

B bounded focused 44/44 PASS at reconcile.rs SHA256 3b1a17548b8ae392435bf124a70324f9d0ca2471d4ae8a11eb0aad31e97ba575; preserved writer STOPPED with no active command lease. Removal-only cleanup-wrapper visibility awaits HUMAN approval. Full106/19 remain open.

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

Child133's completed lasting-source lease was the new `footprints/tests/encoder_ec11e15244g1_qualification.rs`, front-only0/90/120 numeric-pattern/terminal-role tests. Root separately authorized two verifier-only encoder-body geometry/common-net probes; both compiled and failed at the intended assertions, then producer bytes were restored exactly. Fresh verification and the full cohort passed. Source/probe leases are released; no producer/default/registration/Core/recipe/asset change remains. Parent118's profile/plating/backside/STEP/electrical/firmware/installed-fit gates stay open.

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

### Ticket138 discovered shared test borrowers

Root extends the active F138 writer lease to `web/crates/ui-model/src/tree_tests.rs`, test code only in `web/crates/runtime/src/firmware_position_projection.rs`, `web/crates/pcb/src/pcb_physical_setup/tests.rs`, `web/crates/layout/src/objects/keycap_size_tests.rs`, and test code only in `web/crates/layout/src/objects/matrix_inspector_controller.rs`. These live borrowers of the retired REVIUNG fixture require independent synthetic inputs/oracles. No production behavior, visibility or API edits; source-bound impact and exact test disposition required in handoff. Same writer unit and F complete HEAVY cycle; original planner manifest stays frozen, amendment recorded separately.
