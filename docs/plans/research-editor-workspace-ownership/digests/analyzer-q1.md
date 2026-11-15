# Q1 — Production composition

Snapshot ad9db473012a4a4201217bd5094a4f24e7801932; Git source reads only. No tests, builds or history analysis. Fresh analyzer inspected 10/15 source files and stopped after bounded trace.

## Load-bearing claims

LB1: WorkbenchApp, not Editor, constructs production Runtime and owns subscription/application teardown. Startup main→app→presentation::App→WorkbenchApp; WorkbenchApp use_hook(Runtime::new), startup errors become alert. `web/src/main.rs:26-42 (main) @ ad9db4730`; `web/src/presentation.rs:277-288 (WorkbenchApp) @ ad9db4730`.

LB2: Library visibility is not Editor removal; Close does not itself remove production Editor. WorkbenchApp renders Editor whenever accepted.is_some(), passing hidden:project_browser(); Library condition is browser open or no accepted. `web/src/presentation.rs:728-733 (WorkbenchApp) @ ad9db4730`. Session close cancels gesture/jobs/exports, blocks for recovery or enters Closing for pending work; finish_close sets Closed and clears preview but retains accepted. `application/src/session.rs:2559-2585 (finish_close) @ ad9db4730`.

LB3: WorkbenchApp drop sets active=false, unsubscribes, submits Close. This registered action is not a browser-termination completion guarantee. Runtime drive has closed-session worker/frame cleanup, and Runtime destructor independently cancels frames/closes preview/CAD/export/Core workers. `web/src/presentation.rs:451-459 (WorkbenchApp) @ ad9db4730`; `web/crates/runtime/src/runtime.rs:2256-2284 (drive) @ ad9db4730`; `web/crates/runtime/src/runtime.rs:8013-8026 (drop) @ ad9db4730`.

LB4: The inspected mounted Editor fixture supplies its own notifier/providers/selection adapter rather than injecting Runtime into WorkbenchApp. It renders Editor only when lifecycle !=Closed, unlike production accepted-snapshot condition. mount_document creates test Runtime and VirtualDom::new(host), supplies root contexts. `web/src/presentation/layout_field_preview_tests.rs:190-258 (host) @ ad9db4730`; `web/src/presentation/layout_field_preview_tests.rs:269-291 (mount_document) @ ad9db4730`.

## Trace and checks

Runtime::new→new_with_restoration(true)→deployment prefix/BrowserStore::scoped→new_with_store→Session::new/CoreWorker factory/browser persistence. Diagnostic persistence wraps the store. Returns Rc<Runtime> or propagates error. `web/crates/runtime/src/runtime.rs:455-491 (new_with_store) @ ad9db4730`.

Restoration startup upgrades weak Runtime before installing lease/clearing initialization/restoring active project; error branch is distinct; test/test-support exclude service-worker registration. `web/crates/runtime/src/runtime.rs:558-610 (restore_active_project) @ ad9db4730`. open_saved rejects obsolete open_sequence results on success, missing and error paths, then confirms replacement before Open/RecoverWithDocument. `web/crates/runtime/src/runtime.rs:7506-7560 (open_saved) @ ad9db4730`.

Runtime notifier is one replaceable slot: subscribe assigns Some, unsubscribe takes it, changed clones and invokes. `web/crates/runtime/src/runtime.rs:1104-1108 (subscribe) @ ad9db4730`; `web/crates/runtime/src/runtime.rs:1169-1173 (changed) @ ad9db4730`.

Production callback rejects inactive owner, failed weak upgrade and reentrant reconciliation. Scope transition clears selection/anchor, advances generation, records transition and invokes registered cleanup. Context currentness and eligible/live membership reconcile selection; invalid anchors clear. `web/src/presentation.rs:324-448 (WorkbenchApp) @ ad9db4730`. Separate Session reconciliation after accepted persistence prunes missing parts and invalid board/instance. `application/src/session.rs:2027-2063 (persist_completed) @ ad9db4730`; `application/src/session.rs:2199-2233 (reconcile_selection_and_board) @ ad9db4730`.

WorkbenchApp provides Runtime/version/project-creation/selection/workspace/browser/compact-panel/export-return/Case-generation and visibility/appearance contexts; notification provider is descendant. `web/src/presentation.rs:462-494 (WorkbenchApp) @ ad9db4730`; `web/src/presentation.rs:557-578 (WorkbenchApp) @ ad9db4730`; `web/src/presentation.rs:680-681 (WorkbenchApp) @ ad9db4730`. Editor consumes contexts and installs layer-dock/assembly providers and workspace hooks. `web/src/presentation.rs:3273-3321 (Editor) @ ad9db4730`. Typed workspace dispatch excludes Layout pointer SVG. `web/src/presentation/workspace_composition.rs:71-138 (canvas) @ ad9db4730`.

Editor explicit drop hooks disconnect geometry observer and remove only the matching cleanup registration. Registered cleanup releases pointer capture and conditionally cancels owned non-pan drag; shown drop hook removes rather than invokes that closure. `web/src/presentation.rs:5218-5222 (Editor) @ ad9db4730`; `web/src/presentation.rs:6170-6218 (Editor) @ ad9db4730`. App has separate media-query-listener drop. `web/src/presentation.rs:608-618 (WorkbenchApp) @ ad9db4730`.

Runtime submit routes Session event, notifies, drives effects. `web/crates/runtime/src/runtime.rs:2121-2150 (submit) @ ad9db4730`; `application/src/session.rs:1260 (submit) @ ad9db4730`. Aborted pending saves may block deferred Close by recovery; committed saves may finish it; closed Session refuses subsequent operation events. `application/src/session.rs:1972-2005 (persist_completed) @ ad9db4730`; `application/src/session.rs:2062-2063 (persist_completed) @ ad9db4730`; `application/src/session.rs:712-719 (submit) @ ad9db4730`.

Support Runtime disables restoration, install replaces Session and uses in-process Core, open_document adds memory persistence/Open. `web/crates/runtime/src/runtime.rs:8708-8709 (new_runtime) @ ad9db4730`; `web/crates/runtime/src/runtime.rs:8722-8759 (open_document) @ ad9db4730`. Separate production_shell_fixture composes Header/notifications with supplied state and own subscriber, explicitly not full WorkbenchApp. `web/src/presentation/notifications.rs:485-539 (production_shell_fixture) @ ad9db4730`.

## Scoped negatives and uncertainty

No beforeunload|pagehide|unload in main.rs,presentation.rs,runtime.rs; no WorkbenchApp Runtime props or VirtualDom::new(App/app/WorkbenchApp) patterns in main,presentation,field-preview,notifications. These negatives are not repository-wide absence. Framework teardown ordering, browser-exit completion, last-Rc timing and full-App-removal test guarantees were not established.

## Inventory

web/src/main.rs; web/src/lib.rs; web/src/presentation.rs; web/src/presentation/workspace_composition.rs; web/crates/runtime/src/runtime.rs; application/src/session.rs; web/src/presentation/layout_field_preview_tests.rs; web/src/presentation/notifications.rs; web/crates/library/src/library.rs; web/src/presentation/command_palette.rs. Role reference was absent from Git and read from working tree solely as workflow, not evidence. No excluded plans used.
