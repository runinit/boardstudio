# Frontend v1: portfolio and parallel dispatch view

**Current portfolio:** 61 active parent work packages; VIK-only F4.7 is deferred from first release. The original 62-row graph remains recoverable in Git history; the current tasks.json has 61 active parents. There are 48 bounded ticket records, of which 47 are non-superseded; the superseded F6K.4 aggregate is included only in that historical record count. INT.1 is accepted; F1 and F3a remain previously verified increments.

## How to read the dependencies

`start_after` gates when implementation or fixture-backed preparation may be dispatched. `acceptance_after` names additional real integration joins before the parent can be accepted. A fixture can support preparation and evidence, but it cannot satisfy a real-provider acceptance join. The graph’s `depends_on` is their conservative union. Empty cells mean no graph dependency of that kind.

## Confirmed six-stream execution refinement

The user confirmed a six-workspace persistent execution model and narrowly approved proven capability-level starts. The full paired-parity spec and current worktree/ticket mapping are in [six-stream Workbench parity](../dioxus-workbench-parity/spec.md) and [stream reconciliation](../dioxus-workbench-parity/stream-reconciliation.md). Active scope has 61 parents; the original 62-row graph in Git history keeps its rationale, while current tasks.json excludes VIK-only F4.7 and its joins. The composition preparation associated with accepted INT.1 and Case issue09 are implementing against reviewed private contracts; neither closes a parent. Current child counts remain 48 published records / 47 non-superseded records.

## Full portfolio

| Workflow | Parent | Work package | start_after | acceptance_after | Published child ticket(s) |
|---|---|---|---|---|---|
| Integration | INT.1 | Minimal private workspace seams and shared control ownership | — | — | T1-01 |
| Integration | INT.2 | Small shared scoped request, asset and delivery seam | INT.1 | — | — |
| Boundary proofs | BND.1 | Prove private Dioxus access to existing Rust keycap CAD export | — | — | Keycap CAD 01 |
| Boundary proofs | BND.2 | Prove PCB export-owned commit and stale-snapshot sequencing | — | — | — |
| Project and workspace UI | F2.1 | Project start, library, search and demos | INT.1 | — | T1-02–06 |
| Project and workspace UI | F2.2 | Project lifecycle and portable archive | F2.1 | INT.2 | — |
| Project and workspace UI | F2.3 | Workspace panels and compact drawers | INT.1 | — | T1-07–09 |
| Project and workspace UI | F2.4 | Setup guide, shared menus, shortcuts and focus | F2.3 | — | — |
| Layout | F3.1 | Layout tree, board scope and selection | INT.1 | — | T1-10–12 |
| Layout | F3.2 | Matrix and component authoring | F3.1 | — | `.scratch/dioxus-layout-authoring/issues/01–04` |
| Layout | F3.3 | Transforms, constraints and snapping | F3.1 | F3.2 | — |
| Layout | F3.4 | Outline editing, versions and refinements | F3.1 | — | — |
| Layout | F3.5 | Layout inspector, relationships and findings | F3.1 | — | — |
| Layout | F3.6 | 2D camera, fit and 3D Layout assembly viewer | F3.1, F7.1 | F7.3 | — |
| Layout | F3.7 | Integrated Layout parity qualification | F3.1, F3.2, F3.3, F3.4, F3.5, F3.6, F3.8 | F2.3, F6C.3 | — |
| Layout | F3.8 | Supported geometry script editor | F3.1 | F3.5 | — |
| Parts library | F4.1 | Browse and search the Parts library | INT.1 | — | Parts 01–02 |
| Parts library | F4.2 | Create, import and edit footprint definitions | F4.1 | INT.2 | — |
| Parts library | F4.3 | Edit supported generator settings and preview generated geometry | F4.1 | INT.2 | — |
| Parts library | F4.4 | Inspect isolated 2D and 3D library previews | F4.1, F7.1 | F7.3, INT.2 | Parts 03 |
| Parts library | F4.5 | Edit and save part mechanical-fit profiles | F4.1 | INT.2 | — |
| Parts library | F4.6 | Author and save reusable key assemblies | F4.1 | F4.4, F3.2 | — |
| PCB | F5.1 | PCB workspace and host layers | INT.1 | — | PCB 01–02 |
| PCB | F5.2 | Electrical resolver and wiring summary | F5.1 | INT.2 | — |
| PCB | F5.3 | Manual pin review, nets, and protected handoff | F5.2 | F4.2, F3.2, F6K.4 | — |
| PCB | F5.4 | Mounted-module PCB overlays and finding focus | F5.1 | INT.2 | — |
| PCB | F5.5 | Mounted-module placement and inspector integration | F5.4 | — | — |
| PCB | F5.6 | Physical-board instance setup and Case handoff | INT.1 | — | — |
| PCB | F5.7 | Routed KiCad board and model references | INT.1 | INT.2 | — |
| PCB | F5.8 | PCB cross-workspace parity and inventory closure | F5.1, F5.2, F5.3, F5.4, F5.5, F5.6, F5.7 | F2.3, F6K.4, F7.3, F7.7 | — |
| Keymap and Keycaps | F6K.1 | Keymap projection, layers, selection, and active-layer 2D view | INT.1 | F3.1 | Keymap 01–02 |
| Keymap and Keycaps | F6K.2 | Binding editor, supported behavior fields, and keycode search | F6K.1 | — | Keymap 03 |
| Keymap and Keycaps | F6K.3 | Structured macro editor | F6K.1, F6K.2 | INT.2 | Keymap 04 |
| Keymap and Keycaps | F6K.4 | Encoder bindings, PCB firmware-position editor, and ZMK export handoff | F6K.1, F6K.2 | F5.2, F8.2 | Keymap 06–08 (05 retained superseded) |
| Keymap and Keycaps | F6C.1 | Keycaps projection, shared selection, and physical 2D view | INT.1 | F3.1 | Keycaps 01 |
| Keymap and Keycaps | F6C.2 | Board, matrix, and per-key keycap controls | F6C.1 | — | — |
| Keymap and Keycaps | F6C.3 | Shared key-size drafts, selection scope, and linked reflow | F6C.1 | F3.2, F3.5 | — |
| Keymap and Keycaps | F6C.4 | Keycap fit resolution, findings, and navigation | F6C.2 | INT.2 | — |
| Keymap and Keycaps | F6C.5 | Key workspace shared 3D consumption, keycap preview and STEP action | F6C.4, F7.1 | F7.3, F8.2, BND.1 | — |
| Keymap and Keycaps | F6.6 | Keymap and Keycaps shared integration and paired public acceptance | F6K.1, F6K.2, F6K.3, F6K.4, F6C.1, F6C.2, F6C.3, F6C.4, F6C.5 | F2.3, F5.2, F7.6 | — |
| Case and shared viewer | F7.1 | Common viewer contract and renderer feasibility | — | — | Viewer 01 |
| Case and shared viewer | F7.2 | Authored Case body and stack editor | INT.1 | — | Case 01 |
| Case and shared viewer | F7.3 | Single shared assembly viewer and model-preview adapter | F7.1 | INT.2, BND.1 | Viewer 02–06 |
| Case and shared viewer | F7.4 | Mechanical assembly configuration editor | INT.1 | INT.2 | Case 03–06, 08 |
| Case and shared viewer | F7.5 | Case viewer direct-manipulation preview edits | F7.3, F7.4 | — | Case 07 |
| Case and shared viewer | F7.6 | Case preview, readiness, generation and STEP export workflow | F7.2, F7.4 | — | — |
| Case and shared viewer | F7.7 | Physical board/instance Case projection and F5 handoff | F7.2, F7.3, F7.6 | F5.6 | — |
| Case and shared viewer | F7.8 | Cross-workflow viewer adoption and paired acceptance | F7.3, F7.5, F7.6, F7.7 | F3.6, F4.4, F6C.5, F2.3 | — |
| Export | F8.1 | Complete Export workspace, exact output rows, readiness and return path | INT.1 | — | Export 01 |
| Export | F8.2 | Reusable private export coordination, snapshot guards and browser delivery | F8.1 | INT.2, BND.2 | — |
| Export | F8.3 | Integrate selected-board KiCad, draft, outline, and footprint exports | F8.1, F8.2 | F3.4, F4.3, F5.2, F5.3, F5.5 | — |
| Export | F8.4 | Join firmware and local keycap STEP exports through F6 owners | F8.1, F8.2 | F5.2, F6K.4, F6C.5 | — |
| Export | F8.5 | Join authored and generated mechanical outputs through F7 owners | F8.1, F8.2 | F7.2, F7.6, F7.7 | — |
| Export | F8.6 | Qualify end-to-end project-to-export journeys and delivery cleanup | F8.1, F8.2, F8.3, F8.4, F8.5 | F2.2, F3.7, F4.5, F4.6, F5.8, F6.6, F7.8 | — |
| Qualification and adoption | F9.1 | Inventory and transferable tests | — | — | — |
| Qualification and adoption | F9.2 | Paired workflow verification | — | F2.2, F2.4, F3.7, F4.5, F4.6, F5.8, F6.6, F7.8, F8.6 | — |
| Qualification and adoption | F9.3 | Assistive technology qualification | — | F9.2 | — |
| Qualification and adoption | F9.4 | Compatibility and offline journeys | — | F8.6 | — |
| Qualification and adoption | F9.5 | Affected performance and resource gates | — | F9.2 | — |
| Qualification and adoption | F9.6 | Release candidate and adoption patch | F9.1, F9.2, F9.3, F9.4, F9.5 | — | — |
| Qualification and adoption | F9.7 | Approved adoption and retirement | F9.6 | — | — |

## Initial dispatch frontier: nine parents

The graph has nine initial work packages with no `start_after` prerequisites: **INT.1, BND.1, BND.2, F7.1, and F9.1–F9.5**. Their outcomes differ: INT.1 and boundary/viewer proofs unblock consumers; F9.1–F9.5 can start transferable-test inventory, paired verification preparation, assistive-technology evidence planning, compatibility/offline preparation, and performance/resource qualification in parallel. F9 preparation is not final qualification or release acceptance: F9.2–F9.5 retain their explicit `acceptance_after` joins, and F9.6 still waits for all five qualification parents.


## Current integrated evidence and open joins

Latest bounded status (2026-10-02): the Case-only `b6d2af49` candidate at 34679 and its 924px document measurement are retained as historical evidence. Current source `e2a84d8b00418eab4d1ec47e0bf5b6957da9ffe3` is packaged as `frontend-case-focus-resize-final-20261002` (eight commands/971 source hashes) and served at 34683 root/subpath. The bounded final public packet is green for the tested compact panel/keyboard/focus matrix and root/subpath/offline delivery. The resize settlement assessment distinguishes an immediate intermediate DOM read from the settled state: same Generate child retains focus, both drawers close, and six timed runs have no covered sample in 30 compact animation frames each. This is not a general responsiveness/accessibility claim; broader workspace behavior, actual AT and parent acceptance remain open. F7.3’s issue02 remains the Case integration continuation; issues03–08 cover Layout, Keymap, Keycaps, Parts sample and imported/generated model delivery; F7.3 still starts after F7.1 and keeps INT.2/BND.1 acceptance and later F7.8 unchanged.

The Keymap layer/common-viewer build at `6468e80d` has page plus offline root/subpath packaging and controlled root/subpath offline reload evidence. F6K.1’s layer source is not a binding/macro/controller acceptance. The binding editor focused public packet is complete (broader F6 gates remain open). Case mechanical settings controller/mount are integrated in `0cad7577`; strict WASM/native Clippy and 34 native checks pass, build `frontend-mechanical-settings-20261002` completed all seven commands with 965 source hashes; bounded public semantic checks are green at port 34675 (root and `/boardstudio/`); draft-state/layout checks remain open. Macro source `ac43980c` is mounted and independently reviewed in `9ef5bc5c`; native34, strict native/WASM Clippy and formatting pass. Fresh build `frontend-keymap-macros-20261002` completed all seven commands with 967 source hashes from `9ef5bc5cad09ab3064711b45fc348b4cc09a74d4`; it is served at [root](http://127.0.0.1:34677/) and [subpath](http://127.0.0.1:34677/boardstudio/). Public semantic authoring/history/save/reopen checks are complete in distinct profiles; candidate and React both retain the global Core error after reverting to the accepted value, recorded under RF-006. Root/subpath offline/layout checks also pass. Broader F6, firmware and parent acceptance remain open. The earlier planner-only baseline is file SHA-256 prefix `f96b9b28`, checked at `b25d6887` plus root Result/compiler repairs and formatting. Current owned closure source `61e8c43` is integrated in `86a83ddf`; the current 34 native tests include six closure and four feedback tests, and strict WASM/native Clippy passes. Bounded mechanical public checks additionally verify imported Sofle Right-to-CNC while preserving Left’s exact scope, clearing all eight closure parts/definitions on Disable, and matching Undo/Redo, full archive equality and reload. Split-instance semantics are green; draft-state and layout checks remain underway, and authored mesh/STEP plus whole-parent acceptance remain open. Root integration evidence, including the native and Clippy logs, is retained in `../dioxus-case-workspace/evidence/mechanical-feedback-regression/`. F7.7 child 02 repairs the bounded default-instance selection behavior; it does not close F7.7.

Keymap binding source `6509f557` is now mounted through the root, independently source-reviewed and built in `frontend-keymap-bindings-20261002`; strict WASM/native Clippy, formatting, 30 native tests, all seven packaging commands and 961 source hashes pass. The root/subpath demo is served at 34673, with the focused public packet complete. A programmatic driver-select red is retired because React exhibits the same synthetic event ordering; actual native Tab during blur-save moves focus to BODY while single-flight disables inputs, unlike React focus moving to Hold. After-idle pointer/native selection preserves and saves Tap and Hold. This is a focus-continuity difference, not data loss. Full F6 legacy/malformed/stale-race, broad semantics and actual AT gates remain open; no ticket acceptance is inferred. Mechanical settings controller/mount are integrated in `0cad7577`; strict WASM/native Clippy and 34 native checks pass. The fresh seven-command/965-hash build is served at 34675; bounded Configure→four NPTH, CNC materials, Undo/Redo around Disable, split Sofle Left-preservation/Right-CNC, all-eight closure removal/restoration and archive equality/reload checks pass. Draft-state and layout checks continue; authored mesh/STEP and full parent acceptance remain open.

The shared-viewer model-delivery base contract and issues03–08 are independently cleared and published: issue02 continues as the Case integration umbrella; issues03–06 cover Layout, Keymap, Keycaps and Parts sample, and issues07–08 cover imported-board and generated-board model delivery (six additional tickets, 35→41). Issue07’s model module source is reviewed but unregistered/uncompiled; issue08’s worker authoring starts in the shared-viewer worker lane, while Runtime/host/mount/build remain root serial ownership. All existing work retains F7.1 start, INT.2/BND.1 F7.3 acceptance joins and later F7.8; no parent graph edge changes. Full model delivery and consumer acceptance remain open.

For F3.1, T1-10 already covers hierarchy and board groups, independent disclosure, scope-safe tree/canvas/Inspector selection, keyboard/pointer operation, focus and history-neutrality evidence. T1-11 separately owns outline-version and bridge navigation; T1-12 owns rectangular keyboard range and modifier semantics. Paired hierarchy/unowned-row public checks passed at 515f390d; selected-context summary/numeric QA is mounted at e510dd4c. The f65 semantic row repair has source review/build and focused candidate axe/keyboard evidence, but broad current-scope public proof and actual AT remain open. The retained T1 tickets are sufficient for the remaining identified F3.1 gaps; no duplicate children were added. See [source-grounded F3.1 frontier note](evidence/planning/current-frontier-20261002.md).

## Pull scheduling and order of work

INT.1 is already accepted and directly unlocks 13 more parents: INT.2, F2.1, F2.3, F3.1, F4.1, F5.1, F5.6, F5.7, F6K.1, F6C.1, F7.2, F7.4 and F8.1. Together with the eight other initial packets, that makes 21 start-eligible parents, not 21 concurrent workers. This is a continuous pull queue: start any packet whose own `start_after` is satisfied as an owner and review slot becomes available. Do not hold all work for a whole tranche or milestone. Later acceptance joins remain enforced at each parent.

Current pull work includes remaining F2.1 lifecycle/search, F2.3 compact drawers, F3.1 tree/outline/range acceptance, Parts footprint/assembly proof, PCB/Keymap/Keycaps consumers, Case settings/controllers and export rows. Start each child from its active parent start gate; do not wait for full completion of another workflow unless the active roadmap names that dependency. F9 test and resource work proceeds alongside feature implementation; its final joins continue to gate qualification and adoption.

## Capacity, ownership and review

Keep six persistent private workbench author queues. The user/configured ceiling is30, while tool metadata and observed spawn rejection currently impose11 live slots including root. Dispatch within actual capacity and reserve independent verification, Sol 6.1 High Standards/Spec review, QA and serial root integration/builds. Reduce simultaneous authoring when review or public verification queues. Historical Astra reviews remain evidence; new review dispatch follows [current routing](AGENT-ROUTING.md). The active roadmap has 61 parents.

The coordinator owns shared presentation/runtime/shell, global CSS, build files, shared control contracts and the portfolio/dispatch ledger. Feature authors edit their explicitly owned private modules and hand off bounded shared changes for coordinator integration. Use the [integration handoff and root lease](integration-handoff-template.md) to serialize overlapping shared-file edits and freeze packaging inputs.

This active view is derived from the [61-parent graph](tasks.json); the original VIK-only F4.7 parent and edges remain recoverable in Git history. The first-wave child proposal remains at [proposal.json](../dioxus-frontend-tranche-1/proposal.json). The current published-child mapping is listed in the table above. The [model policy](AGENT-ROUTING.md) governs assignments.

The user authorized implementation and automatic bounded-ticket publication on 2026-10-02; 48 bounded ticket records are published, of which 47 are non-superseded. This cumulative count includes the retained superseded F6K.4 aggregate; it does not imply dispatch readiness or acceptance. Remaining active parent scopes without a bounded child still require source-checked decomposition before broad implementation dispatch. This view changes neither that authorization nor parent statuses and authorizes no API or cutover changes. The original graph remains in Git history; this dispatch view and current tasks.json have 61 active parents. Planning/review may continue while implementation and verification use the available capacity.


Retained portfolio history (2026-10-02): the F6K.4 encoder/firmware aggregate issue05 is superseded for dispatch by published issues06–08; its criteria/history remain. At that checkpoint counts were 48 published records total / 47 non-superseded records, with 62 source-graph parents. F6K.4 still starts after F6K.1/F6K.2 and keeps F5.2/F8.2 as acceptance joins. Issue06’s private binding-target contract has cleared review. Source correction `868edfcb` is mounted and the eight-command/973-hash build `frontend-encoder-select-fixed-20261002` is served at 34687; targeted 34687 browser regression now passes (all three empty values are `none`). The genuine physical-fixture workflow now passes: fresh None defaults, clockwise/counterclockwise/push edits, Undo/Redo and export/reimport. The final separate regression/layout packet covers both offline routes and ordinary non-first binding selections after reload. Scope switching, full keyboard/focus/AT, module rejection/recovery, F5.2/F8.2 and issue06/parent acceptance remained open at that checkpoint.
