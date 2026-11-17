---
title: 'Codebase research: Editor workspace ownership'
type: codebase
ask: 'Establish current composition, ownership, workflows, testing and history at one committed snapshot.'
repository: '/home/chris/01_Projects/ts-boardstudio2/.worktrees/bmad-ticket-trees'
commit: 'ad9db473012a4a4201217bd5094a4f24e7801932'
branch: 'docs/bmad-ticket-trees'
status: complete
validation: normal
claims: {verified: 24}
created: '2026-11-15'
updated: '2026-11-15'
---

# Codebase research: Editor workspace ownership

This factual report describes committed source; it does not recommend backlog changes or claim tests were run. Researchers receive neutral questions and source pointers, excluding plans and desired changes. Read-only Git snapshot access was explicitly authorized after Gortex could not serve the immutable view.

## Questions

1. How does production composition work? Where are Runtime construction, subscription, reconciliation, providers and App/Editor teardown owned?
2. Where does workspace orchestration actually live? Who owns controllers, projections, admitted actions, surfaces and resources for six workspaces, setup guide and camera?
3. How do cross-workspace workflows preserve ownership, cancellation and stale-result rejection?
4. How is that behavior tested through composition, mounted and packaged seams?
5. Why are the current boundaries and lifetimes this way?

Budget: 15 files per question; 10 history investigations total; normal independent verification. Findings and limitations will be appended from fresh source-bound digests.

## 1. How does production composition work?

WorkbenchApp constructs the production Runtime with `use_hook(Runtime::new)`; App renders that component. WorkbenchApp owns the production subscription and common providers. Its callback rejects inactive owners, failed weak upgrades and reentrant reconciliation, then reconciles scoped selection and invokes the registered cleanup on scope change. Runtime has one replaceable notifier slot, rather than independent subscribers per workspace. `web/src/presentation.rs:277-288 (WorkbenchApp) @ ad9db4730`; `web/src/presentation.rs:324-494 (WorkbenchApp) @ ad9db4730`; `web/crates/runtime/src/runtime.rs:1104-1108 (subscribe) @ ad9db4730`.

Library visibility and teardown differ: production renders Editor whenever an accepted snapshot exists and applies project-browser visibility to its containing div's hidden attribute. Session Close can drain pending work or be blocked by recovery; finish_close sets Closed without removing accepted. Thus Close alone is not the production Editor unmount condition. `web/src/presentation.rs:728-733 (WorkbenchApp) @ ad9db4730`; `application/src/session.rs:2559-2585 (finish_close) @ ad9db4730`.

WorkbenchApp drop deactivates its callback, unsubscribes and submits Close; Runtime drive and its destructor have separate worker/frame cleanup. These source registrations do not establish browser-exit completion or framework teardown ordering. Editor's shown drop hooks disconnect its observer and remove only its matching cleanup registration; the pointer-release/cancel actions belong to the registered closure, not an invocation shown in that drop hook. `web/src/presentation.rs:451-459 (WorkbenchApp) @ ad9db4730`; `web/crates/runtime/src/runtime.rs:2256-2284 (drive) @ ad9db4730`; `web/crates/runtime/src/runtime.rs:8013-8026 (drop) @ ad9db4730`; `web/src/presentation.rs:6170-6218 (Editor) @ ad9db4730`.

The inspected mounted Editor fixture creates its own host, notifier, providers and selection adapter. Its Editor mount condition is lifecycle !=Closed, unlike production's accepted-snapshot condition. A separate shell fixture mounts Header/notifications without full WorkbenchApp. These are existing useful seams, not the same composition path. `web/src/presentation/layout_field_preview_tests.rs:190-291 (mount_document) @ ad9db4730`; `web/src/presentation/notifications.rs:485-539 (production_shell_fixture) @ ad9db4730`.

Q1 inspected 10/15 files. Scoped searches found no browser-exit handlers in main/presentation/Runtime and no supplied-Runtime WorkbenchApp props/full WorkbenchApp VirtualDom construction in the four inspected composition/test files. These negatives are not repository-wide absence. See [Q1 digest](digests/analyzer-q1.md) for exact searches, call paths and inventory. Load-bearing claims are independently verified; the enclosing WorkbenchApp/Editor declarations resolve mechanical symbol-window misses. See [core verifier digest](digests/verifier-core.md).

## 2. Where does workspace orchestration live?

Ownership is already distributed, but not wholly encapsulated by workspace modules. The table separates existing module responsibilities from caller assembly; it is not a list of missing features.

| Area | Existing module ownership | Assembly still in Editor |
| --- | --- | --- |
| Layout | Workspace state and Objects/toolbar/Inspector construction. `web/src/presentation/layout_workspace.rs:31-120 (InspectorInput) @ ad9db4730`; `web/src/presentation/layout_workspace.rs:159-212 (use_layout_workspace_state) @ ad9db4730`. | Inspector/transform/align/setup/placement/pair/outline mounts and transaction SVG. `web/src/presentation.rs:3455-3525 (Editor) @ ad9db4730`; `web/src/presentation.rs:3641-3660 (Editor) @ ad9db4730`; `web/src/presentation.rs:9945-9964 (Editor) @ ad9db4730`. |
| PCB | Objects/toolbar/SVG and hidden/inert Inspector panes. `web/src/presentation/pcb_workspace.rs:23-165 (PcbInspector) @ ad9db4730`. | Wiring/firmware-position/part-net/part-input controllers and canvas-input construction. `web/src/presentation.rs:3819-3850 (Editor) @ ad9db4730`; `web/src/presentation.rs:8259-8283 (Editor) @ ad9db4730`. |
| Parts | Query, selection, orientation, generations, preview state and surface construction. Preview child owns catalogue/recipe resources. `web/src/presentation/parts_workspace.rs:13-67 (select_mounted_module) @ ad9db4730`; `web/crates/parts/src/parts.rs:1537-1610 (PartsPreviewWorkspace) @ ad9db4730`. | PartPlacementHost construction with definition loading, selection/guide/panels/snap/arbiter. `web/src/presentation.rs:5512-5536 (Editor) @ ad9db4730`. |
| Case | State plus substantial tree/display admission and action implementation; visible CasePanel owns preview/generation effects. `web/src/presentation/case_workspace.rs:693-953 (apply_tree_action) @ ad9db4730`; `web/crates/case/src/cad_presentation.rs:24-48 (CasePanel) @ ad9db4730`. | Admission construction/current-workspace wrappers and mechanical-settings mount. `web/src/presentation.rs:4960-4987 (Editor) @ ad9db4730`; `web/src/presentation.rs:4113-4120 (Editor) @ ad9db4730`. |
| Keymap/Keycaps | Memoized projections; active layer or focused finding/pending fit; surface construction from supplied actions. `web/src/presentation/keymap_workspace.rs:87-110 (use_projection) @ ad9db4730`; `web/src/presentation/keycaps_workspace.rs:20-73 (use_keycaps_workspace_state) @ ad9db4730`. | Fit/settings and layer/encoder/binding/macro controller mounts and source construction. `web/src/presentation.rs:3778-3818 (Editor) @ ad9db4730`; `web/src/presentation.rs:3933-3994 (Editor) @ ad9db4730`. |
| Setup guide | Project-matched requests, deduplication, preferences, stages/dismissal and instance preference state. `web/src/presentation/setup_workspace.rs:15-114 (use_setup_workspace_state) @ ad9db4730`. | Stage controls/routing, selection clearing and drawer composition. `web/src/presentation.rs:9121-9142 (Editor) @ ad9db4730`; `web/src/presentation.rs:9855-9881 (Editor) @ ad9db4730`. |
| Shared camera | Pan state, Space listener, mount measurement and camera-event submission. `web/crates/ui-shared/src/canvas_navigation.rs:37-57 (use_canvas_navigation_state) @ ad9db4730`; `web/crates/ui-shared/src/canvas_navigation.rs:125-148 (mount_handler) @ ad9db4730`. | SVG storage, ResizeObserver and page-specific handler arbitration. `web/src/presentation.rs:5178-5222 (Editor) @ ad9db4730`; `web/src/presentation.rs:7138-7146 (Editor) @ ad9db4730`. |

Controller lifetime does not imply current UI authority: retained Keymap layer/macro hooks set owner scope to None outside Keymap, follow owner changes and continue terminal observation. PCB's parent-mounted plan owner instead remains available independently of visible PCB children, rejecting dead/superseded/current-identity mismatches. `web/crates/keymap/src/keymap/layer_controller.rs:137-201 (use_layer_operations) @ ad9db4730`; `web/crates/keymap/src/keymap/macro_controller.rs:270-330 (use_macro_operations) @ ad9db4730`; `web/crates/pcb/src/pcb_wiring/controller.rs:934-998 (use_pcb_wiring_resolution_owner) @ ad9db4730`; `web/crates/pcb/src/pcb_wiring/controller.rs:1090-1109 (start_resolution) @ ad9db4730`.

Q2 inspected 15 source bodies plus one locator-only hit in a sixteenth file; that search-only overage is disclosed rather than counted as a read. Binding/placement internals, Keycaps navigation internals, viewer teardown and panel-host mount policy were outside this question's body budget. See [Q2 digest](digests/analyzer-q2.md).

## 3. How do cross-workspace workflows preserve ownership?

PCB plan identity is board-scoped: current_input removes physical-instance selection, validates lifecycle/scope/revision consistency and constructs the token/revision/executor identity; start_resolution compares the live identity before publication. Editor distributes one controller resolution to its consumers. Sequence/alive/current-input checks suppress obsolete publication; they do not prove the underlying future was aborted. `web/crates/pcb/src/pcb_wiring/controller.rs:1023-1054 (current_input) @ ad9db4730`; `web/crates/pcb/src/pcb_wiring/controller.rs:1065-1124 (start_resolution) @ ad9db4730`; `web/src/presentation.rs:3819-3839 (Editor) @ ad9db4730`.

Core completion retains a candidate and emits Persist; only the matching committed save publishes accepted document/token/scene and Landed settlement. Observation retirement is distinct: EditTicket and PendingEdits may retire/replace UI observations without cancelling the submitted Session work. `application/src/session.rs:1838-1935 (core_completed) @ ad9db4730`; `application/src/session.rs:1966-2059 (persist_completed) @ ad9db4730`; `web/crates/runtime/src/edit_ticket.rs:87-194 (settlement_of) @ ad9db4730`; `web/crates/runtime/src/pending_edits.rs:32-103 (PendingEdits) @ ad9db4730`.

Physical setup shares one busy workflow but distinguishes guide and Case contexts. Resource preparation rechecks the owner before submission. A Case reassignment admits only its recorded one-generation transition; finish_operation navigates/reconciles only after Landed while the original or expected reassignment owner remains current. `web/src/presentation.rs:3851-3928 (Editor) @ ad9db4730`; `web/crates/pcb/src/pcb_physical_setup/controller.rs:229-287 (use_controller) @ ad9db4730`; `web/crates/pcb/src/pcb_physical_setup/controller.rs:610-646 (expected_case_reassignment_owner) @ ad9db4730`; `web/crates/pcb/src/pcb_physical_setup/controller.rs:870-967 (finish_operation) @ ad9db4730`.

Placement preparation uses exact token/revision and source/workspace admission. Its post-submit route intentionally does not require the old token/revision: Landed selection comes from accepted membership, with kind-specific Layout/PCB return routes. Cancel refuses while committing and cannot redirect a changed owner. Module attachment instead captures Parts selection/generation/scope; only Landed builds its navigation result, which the root rechecks against accepted module/host/source and current Parts destination. `web/crates/layout/src/part_placement.rs:835-938 (use_controller_placement) @ ad9db4730`; `web/crates/layout/src/part_placement.rs:1219-1357 (use_controller_placement) @ ad9db4730`; `web/crates/layout/src/part_placement.rs:1401-1422 (placement_route_is_current) @ ad9db4730`; `web/crates/parts/src/parts/modules_catalogue/module_attachment.rs:175-223 (ModuleAttachment) @ ad9db4730`; `web/src/presentation.rs:4853-4913 (Editor) @ ad9db4730`.

Guide stages map Project/Layout/Wiring to Layout, Case to Case and Review to Export. Finding navigation has stronger deferred authority than scope alone: destination must remain selected, pending fit revalidates before effects, and queued focus checks lifetime before current owner. Layout cross-board findings have a separate board-hop/resume path retaining source identity and live-finding checks. `web/crates/layout/src/setup_guide.rs:610-635 (activate_stage) @ ad9db4730`; `web/crates/keycaps/src/keycaps_navigation.rs:80-110 (NavigationOwner) @ ad9db4730`; `web/crates/keycaps/src/keycaps_navigation.rs:122-194 (use_pending_layout_fit) @ ad9db4730`; `web/crates/keycaps/src/keycaps_navigation.rs:219-233 (queue_owner_focus) @ ad9db4730`; `web/crates/layout/src/layout_findings_state.rs:73-176 (use_layout_findings_state) @ ad9db4730`.

Canvas admission and cancellation use scope/pointer/gesture-generation and transaction identity; they cannot cancel an unrelated Session gesture. Workspace changes cancel the owned drag. Export and job currentness use their own operation/snapshot/scope identities, not the canvas identity. `web/src/presentation.rs:6636-6696 (Editor) @ ad9db4730`; `web/src/presentation.rs:6785-6857 (Editor) @ ad9db4730`; `web/crates/ui-model/src/selection.rs:459-492 (owns_session_gesture) @ ad9db4730`; `application/src/session.rs:2357-2401 (invalidate_gesture) @ ad9db4730`; `application/src/session.rs:699-710 (export_is_current) @ ad9db4730`.

Q3 used 15/15 files. Deeper plan-consumer validation, catalogue proposal internals, complete canvas-owner arbitration and setup-state persistence remain untraced. No product failure or test pass follows from this source trace. See [Q3 digest](digests/analyzer-q3.md).

## 4. How is this behavior tested?

The inspected tests provide substantial reusable proof at distinct seams. Source inspection here establishes their assertions, not successful execution.

| Seam | What its assertions establish | What they do not establish |
| --- | --- | --- |
| Mounted Editor host | Production Editor with a separately assembled notifier/providers and supplied Runtime; document execution/saves use replacement in-process/memory adapters. The initial Runtime constructor can still construct browser infrastructure. `web/src/presentation/layout_field_preview_tests.rs:190-291 (mount_document) @ ad9db4730`; `web/crates/runtime/src/runtime.rs:8752-8759 (open_document) @ ad9db4730`. | Production WorkbenchApp composition/restoration/drop. |
| Matrix/Case fixtures | Retained matrix draft/update blocker, one current-session registration and one pending Create; conditional Case Inspector hook ordering and previous-geometry/current-finding distinctions. `web/src/presentation/layout_draft_safety_tests.rs:1656-1816 (matrix_hidden_draft_survives_inspector_park_workspace_switch_and_update) @ ad9db4730`; `web/src/presentation/case_workspace.rs:1526-1642 (case_inspector_navigation_keeps_hooks_in_the_mounted_child) @ ad9db4730`; `web/src/presentation/case_workspace.rs:1673-1815 (production_workspace_keeps_same_scope_layer_context_then_retires_it_on_owner_change) @ ad9db4730`. | Complete production workspace routing or all-workspace retention policy. |
| PCB plan owner | Controlled resolver failure/retry and obsolete other-board/same-board publication rejection. `web/crates/pcb/src/pcb_wiring/controller.rs:1315-1450 (mounted_resolution_ignores_old_same_board_reply_after_controller_revision_changes) @ ad9db4730`. | Every consumer's page wiring or production resolver execution. |
| Setup/placement hooks | Controlled setup admission/hidden settlement/stale-control/save-failure behavior; placement has actual hook-component unmount during held save with no late selection. `web/crates/pcb/src/pcb_physical_setup/tests.rs:248-500 (exact_accepted_proposal_lands_silently_without_leaking_to_another_context) @ ad9db4730`; `web/crates/layout/src/part_placement.rs:3336-3363 (production_hook_does_not_read_signals_after_unmount_while_waiting_for_saved_state) @ ad9db4730`. | Production guide/Parts/PCB/Editor composition or WorkbenchApp teardown. |
| Findings/camera | Layout focus helpers over handwritten Inspector-like DOM; Keycaps projection rendering from a handcrafted view; real measured SVG with direct Space/pan/zoom methods. `web/crates/layout/src/layout_findings_state.rs:487-626 (mounted_back_enter_focuses_board_and_outline_inspectors_and_rejects_stale_scope) @ ad9db4730`; `web/src/presentation/keycaps_workspace_tests.rs:118-160 (inspector_header_shows_accepted_assigned_and_total_key_count) @ ad9db4730`; `web/crates/ui-shared/src/canvas_navigation.rs:312-446 (handle_keeps_the_canvas_surface_measurement_owned_by_navigation_state) @ ad9db4730`. | Actual Inspector navigation, trusted browser gesture arbitration or listener removal. |
| Session gestures | Direct Core/Session coalescing, pointer-up sample, accepted revision/Undo and ReleasePointer/cancel effects with supplied save outcomes. `application/tests/durable_session.rs:1267-1383 (drag_coalesces_previews_and_commits_the_pointerup_sample_once) @ ad9db4730`; `application/tests/durable_session.rs:2014-2089 (cancelled_pointer_makes_late_preview_inert_and_releases_capture) @ ad9db4730`. | Actual DOM capture release or durable browser storage. |
| Diagnostic packaged/store checks | Normal import/form action, focus/header deferral, held save/release, exact export/adoption/reopen; separate BrowserStore tests check pre-permit absence and post-permit durability/failure-retry. `scripts/verify-update-adoption.py:1655-1736 (leg_layout_constraint) @ ad9db4730`; `web/crates/runtime/src/diagnostic_persistence.rs:646-730 (held_save_reports_entered_without_ack_or_durable_write_until_released) @ ad9db4730`. | A current built package or successful execution this run; all ownership races or GPU qualification. |

The takeover script explicitly models a deferred/cancelled attempt while saving is held, not a post-permit worker-probe race. `scripts/verify-update-adoption.py:2085-2135 (leg_takeover_draft) @ ad9db4730`.

No inspected scenario jointly asserts production WorkbenchApp drop, Editor observer/cleanup-registration removal and Runtime destruction. This is a bounded coverage conclusion, not repository-wide test absence, a teardown defect or browser-exit guarantee. The production paths are distinct. `web/src/presentation.rs:451-459 (WorkbenchApp) @ ad9db4730`; `web/src/presentation.rs:5218-5222 (Editor) @ ad9db4730`; `web/src/presentation.rs:6207-6217 (Editor) @ ad9db4730`; `web/crates/runtime/src/runtime.rs:8013-8026 (drop) @ ad9db4730`.

Q4 inspected 15/15 files, selected spans. Focused Keymap/Keycaps finding suites, module attachment, decoder/provider isolation, packaging/check registration and normal-package diagnostic-gate absence remain outside this bounded read. See [Q4 digest](digests/analyzer-q4.md) and [independent test verification](digests/verifier-tests.md).

## 5. Why are the boundaries and lifetimes this way?

Three history claims were traced; commit subjects express author intent, and the described code changes were checked against diffs. No originating PR or production-incident evidence was retrieved.

1. Initial extraction moved composition rather than all controller ownership. Keymap SVG moved to its workspace module while parent-built inputs remained; PCB's placeholder later became typed accepted-state SVG. The Layout inline-pointer-SVG exception was explicit, but its deeper rationale was not established. `1c5de957f "Extract private workbench compositions"`; `9271ade53 "Mount accepted PCB scene with guarded selection"`; `web/src/presentation/workspace_composition.rs:94-105 (canvas) @ ad9db4730`.
2. Retained layer/macro controllers originally protected pending single-flight/terminal-observer state. Later visibility-aware owner retirement preserved that retention while ending departed UI observation before settlement. This is not evidence that hidden editors should regain fresh admission. `1413685e6 "Add private Keymap layer edit controller"`; `96874be6b "Add private keymap macro editor"`; `ed92e9d9d "Complete panel field bindings and retire departed presentation observations"`; `web/crates/keymap/src/keymap/layer_controller.rs:137-201 (use_layer_operations) @ ad9db4730`; `web/crates/keymap/src/keymap/macro_controller.rs:242-244 (use_macro_operations) @ ad9db4730`.
3. PCB resolution was explicitly unconditional and Editor-lifetime, normalizing physical-instance selection so Case/Layout instance changes would not churn a board query. The later injected resolver enabled controlled replies while preserving production admission/settlement checks. `b29c3abfb "Add private PCB wiring preview adapter"`; `924ba2820 "Qualify PCB resolver error retry and stale board ownership"`; `web/crates/pcb/src/pcb_wiring/controller.rs:934-998 (use_pcb_wiring_resolution_owner) @ ad9db4730`; `web/crates/pcb/src/pcb_wiring/controller.rs:1023-1054 (current_input) @ ad9db4730`.

### Two comparable pattern dates

Dates are author dates. Touch attribution is bounded to the ten inspected commits and supplied line ranges, not whole files or globally latest history. The visibility change was introduced within the surrounding settlement ranges; not every line of those ranges was new.

| Pattern | Introduction | Latest inspected touch of supplied range |
| --- | --- | --- |
| Parent inputs / module SVG, Keymap | 1c5de957f, 2026-10-02 | Canvas b100ac59f, 2026-11-05, CSS changes; parent inputs 784dcb079, 2026-10-07. |
| Parent inputs / module SVG, PCB | 9271ade53, 2026-10-02 | Canvas 26f543d1d, 2026-11-05, CSS changes; parent inputs 784dcb079, 2026-10-07. |
| Retained controllers / retiring UI owners, layer and macro | ed92e9d9d, 2026-10-08; retention foundations 1413685e6/96874be6b, 2026-10-02. | ed92e9d9d, 2026-10-08. |

The additional date-source subjects are `784dcb079 "Compose editor workspace state handles"`, `26f543d1d "Migrate workbench and workspace presentation"` and `b100ac59f "Migrate keymap workspace layouts"`.

Q5 followed exactly ten historical commits: 1c5de957f, 9271ade53, 784dcb079, 26f543d1d, b100ac59f, ed92e9d9d, b29c3abfb, 924ba2820, 1413685e6, 96874be6b. Intermediate helper evolution, wider alternatives and originating PR/issues remain untraced. See [history digest](digests/historian-q5.md) and [independent history verification](digests/verifier-history.md). All three history claims are independently verified within that scope.

## Where the answers meet

WorkbenchApp's subscriber links composition and ownership: scope transition updates generation, reconciles selection and invokes the registered cleanup; Editor owns that matching registration. `web/src/presentation.rs:324-448 (WorkbenchApp) @ ad9db4730`; `web/src/presentation.rs:6170-6218 (Editor) @ ad9db4730`.

The PCB resolution owner links workspace ownership, cross-workspace delivery and tests: it stays parent-mounted but admits publication only for alive/current request identity; controlled resolver tests exercise that seam without full application composition. `web/crates/pcb/src/pcb_wiring/controller.rs:934-998 (use_pcb_wiring_resolution_owner) @ ad9db4730`; `web/crates/pcb/src/pcb_wiring/controller.rs:1090-1109 (start_resolution) @ ad9db4730`; `web/crates/pcb/src/pcb_wiring/controller.rs:1230-1267 (host) @ ad9db4730`.

## Open questions and limits

- What teardown ordering and late-resource behavior occur under actual full production composition? Source registrations and focused tests do not answer this jointly.
- What do the focused Keymap/Keycaps navigation and module-attachment suites assert beyond the bounded Q4 inventory?
- What deeper history explains leaving the Layout transaction inline? The inspected extraction records the choice, not that rationale.
- Which current built package/source/environment qualifies trusted pointer capture, observer/listener cleanup and real provider/GPU behavior? None was executed here.

These are factual follow-up questions, not authorized implementation tasks. The selected load-bearing code claims and three history claims have fresh independent verification, recorded in the digests and append-only claim ledger. No builds, tests, browser interactions, migrations or commits were performed.

## Prior documents

No prior research run was resumed. Relevant decision-record titles were inventoried: queued edits at execution (ADR-0005), save before publication (ADR-0013), document authority/captured lifetimes (ADR-0014), separate export ownership (ADR-0016), real workflow/mounted-browser testing (ADR-0021), toolbar-led workbench (ADR-0041). They were not used as evidence of implemented behavior or historical causality. The separate [planning context digest](digests/planning-context.md) is coordinator-only requirement context, not researcher evidence.

## Verification record

The script-derived ledger tally is 24 verified selected claims: 20 load-bearing code/history claims and four additional citation-enclosure claims. This is not a test count or verification of every possible behavior. Fresh verifier digests retain the checks and precision corrections; [enclosure verification](digests/verifier-symbols.md) resolves the 80-line lexical symbol-window misses without changing any cited symbol.

Mechanical anchors found 98/98 code/history citations at the approved snapshot; the numeric-marker citation check passed (this report uses direct code/history anchors). Root documentation links passed. Logs are under `.evidence/editor-workspace-recon/`. Structure/prose lens findings are preserved in the digests; factual sections and citations remain intact. No implementation checks were run. The separate [ticket proposal](ticket-proposal.md) contains coordinator recommendations, not additional code facts or approved work.

## Rechecking this report

All code/history evidence is pinned to ad9db4730. Re-run `recon_kit.py anchors` without `--at` after source changes to identify changed cited files. Passing mechanical anchors locates evidence; it does not replace the semantic verification in the digests.
