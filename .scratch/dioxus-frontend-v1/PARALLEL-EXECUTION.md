# Frontend v1: portfolio and parallel dispatch view

**Portfolio:** 62 parent work packages; current statuses live in the workflow graph. The 12 published child tickets expand four parents: INT.1 → 01; F2.1 → 02–06; F2.3 → 07–09; F3.1 → 10–12. The other 58 parents remain planned without child tickets. This is a dispatch view, not a scope reduction or a claim that all 62 have been converted to tickets. INT.1 implementation has started; F1 and F3a remain previously verified increments.

## How to read the dependencies

`start_after` gates when implementation or fixture-backed preparation may be dispatched. `acceptance_after` names additional real integration joins before the parent can be accepted. A fixture can support preparation and evidence, but it cannot satisfy a real-provider acceptance join. The graph’s `depends_on` is their conservative union. Empty cells mean no graph dependency of that kind.

## Full portfolio

| Workflow | Parent | Work package | start_after | acceptance_after | Child draft |
|---|---|---|---|---|---|
| Integration | INT.1 | Minimal private workspace seams and shared control ownership | — | — | 01 |
| Integration | INT.2 | Small shared scoped request, asset and delivery seam | INT.1 | — | — |
| Boundary proofs | BND.1 | Prove private Dioxus access to existing Rust keycap CAD export | — | — | — |
| Boundary proofs | BND.2 | Prove PCB export-owned commit and stale-snapshot sequencing | — | — | — |
| Project and workspace UI | F2.1 | Project start, library, search and demos | INT.1 | — | 02–06 |
| Project and workspace UI | F2.2 | Project lifecycle and portable archive | F2.1 | INT.2 | — |
| Project and workspace UI | F2.3 | Workspace panels and compact drawers | INT.1 | — | 07–09 |
| Project and workspace UI | F2.4 | Setup guide, shared menus, shortcuts and focus | F2.3 | — | — |
| Layout | F3.1 | Layout tree, board scope and selection | INT.1 | — | 10–12 |
| Layout | F3.2 | Matrix and component authoring | F3.1 | — | — |
| Layout | F3.3 | Transforms, constraints and snapping | F3.1 | F3.2 | — |
| Layout | F3.4 | Outline editing, versions and refinements | F3.1 | — | — |
| Layout | F3.5 | Layout inspector, relationships and findings | F3.1 | — | — |
| Layout | F3.6 | 2D camera, fit and 3D Layout assembly viewer | F3.1, F7.1 | F7.3 | — |
| Layout | F3.7 | Integrated Layout parity qualification | F3.1, F3.2, F3.3, F3.4, F3.5, F3.6, F3.8 | F2.3, F6C.3 | — |
| Layout | F3.8 | Supported geometry script editor | F3.1 | F3.5 | — |
| Parts library | F4.1 | Browse and search the Parts library | INT.1 | — | — |
| Parts library | F4.2 | Create, import and edit footprint definitions | F4.1 | INT.2 | — |
| Parts library | F4.3 | Edit supported generator settings and preview generated geometry | F4.1 | INT.2 | — |
| Parts library | F4.4 | Inspect isolated 2D and 3D library previews | F4.1, F7.1 | F7.3, INT.2 | — |
| Parts library | F4.5 | Edit and save part mechanical-fit profiles | F4.1 | INT.2 | — |
| Parts library | F4.6 | Author and save reusable key assemblies | F4.1 | F4.4, F3.2 | — |
| Parts library | F4.7 | Inspect VIK module sources and edit library profiles | F4.1 | INT.2, F4.4 | — |
| PCB | F5.1 | PCB workspace and host layers | INT.1 | — | — |
| PCB | F5.2 | Electrical resolver and wiring summary | F5.1 | INT.2 | — |
| PCB | F5.3 | Manual pin review, nets, and protected handoff | F5.2 | F4.2, F3.2, F6K.4 | — |
| PCB | F5.4 | Mounted-module PCB overlays and finding focus | F5.1 | INT.2 | — |
| PCB | F5.5 | Mounted-module placement and inspector integration | F5.4 | F4.7 | — |
| PCB | F5.6 | Physical-board instance setup and Case handoff | INT.1 | — | — |
| PCB | F5.7 | Routed KiCad board and model references | INT.1 | INT.2 | — |
| PCB | F5.8 | PCB cross-workspace parity and inventory closure | F5.1, F5.2, F5.3, F5.4, F5.5, F5.6, F5.7 | F2.3, F4.7, F6K.4, F7.3, F7.7 | — |
| Keymap and Keycaps | F6K.1 | Keymap projection, layers, selection, and active-layer 2D view | INT.1 | F3.1 | — |
| Keymap and Keycaps | F6K.2 | Binding editor, supported behavior fields, and keycode search | F6K.1 | — | — |
| Keymap and Keycaps | F6K.3 | Structured macro editor | F6K.1, F6K.2 | INT.2 | — |
| Keymap and Keycaps | F6K.4 | Encoder bindings, PCB firmware-position editor, and ZMK export handoff | F6K.1, F6K.2 | F5.2, F8.2 | — |
| Keymap and Keycaps | F6C.1 | Keycaps projection, shared selection, and physical 2D view | INT.1 | F3.1 | — |
| Keymap and Keycaps | F6C.2 | Board, matrix, and per-key keycap controls | F6C.1 | — | — |
| Keymap and Keycaps | F6C.3 | Shared key-size drafts, selection scope, and linked reflow | F6C.1 | F3.2, F3.5 | — |
| Keymap and Keycaps | F6C.4 | Keycap fit resolution, findings, and navigation | F6C.2 | INT.2 | — |
| Keymap and Keycaps | F6C.5 | Key workspace shared 3D consumption, keycap preview and STEP action | F6C.4, F7.1 | F7.3, F8.2, BND.1 | — |
| Keymap and Keycaps | F6.6 | Keymap and Keycaps shared integration and paired public acceptance | F6K.1, F6K.2, F6K.3, F6K.4, F6C.1, F6C.2, F6C.3, F6C.4, F6C.5 | F2.3, F5.2, F7.6 | — |
| Case and shared viewer | F7.1 | Common viewer contract and renderer feasibility | — | — | — |
| Case and shared viewer | F7.2 | Authored Case body and stack editor | INT.1 | — | — |
| Case and shared viewer | F7.3 | Single shared assembly viewer and model-preview adapter | F7.1 | INT.2, BND.1 | — |
| Case and shared viewer | F7.4 | Mechanical assembly configuration editor | INT.1 | INT.2 | — |
| Case and shared viewer | F7.5 | Case viewer direct-manipulation preview edits | F7.3, F7.4 | — | — |
| Case and shared viewer | F7.6 | Case preview, readiness, generation and STEP export workflow | F7.2, F7.4 | — | — |
| Case and shared viewer | F7.7 | Physical board/instance Case projection and F5 handoff | F7.2, F7.3, F7.6 | F5.6 | — |
| Case and shared viewer | F7.8 | Cross-workflow viewer adoption and paired acceptance | F7.3, F7.5, F7.6, F7.7 | F3.6, F4.4, F6C.5, F2.3 | — |
| Export | F8.1 | Complete Export workspace, exact output rows, readiness and return path | INT.1 | — | — |
| Export | F8.2 | Reusable private export coordination, snapshot guards and browser delivery | F8.1 | INT.2, BND.2 | — |
| Export | F8.3 | Integrate selected-board KiCad, draft, outline, and footprint exports | F8.1, F8.2 | F3.4, F4.3, F5.2, F5.3, F5.5 | — |
| Export | F8.4 | Join firmware and local keycap STEP exports through F6 owners | F8.1, F8.2 | F5.2, F6K.4, F6C.5 | — |
| Export | F8.5 | Join authored and generated mechanical outputs through F7 owners | F8.1, F8.2 | F7.2, F7.6, F7.7 | — |
| Export | F8.6 | Qualify end-to-end project-to-export journeys and delivery cleanup | F8.1, F8.2, F8.3, F8.4, F8.5 | F2.2, F3.7, F4.5, F4.6, F4.7, F5.8, F6.6, F7.8 | — |
| Qualification and adoption | F9.1 | Inventory and transferable tests | — | — | — |
| Qualification and adoption | F9.2 | Paired workflow verification | — | F2.2, F2.4, F3.7, F4.5, F4.6, F4.7, F5.8, F6.6, F7.8, F8.6 | — |
| Qualification and adoption | F9.3 | Assistive technology qualification | — | F9.2 | — |
| Qualification and adoption | F9.4 | Compatibility and offline journeys | — | F8.6 | — |
| Qualification and adoption | F9.5 | Affected performance and resource gates | — | F9.2 | — |
| Qualification and adoption | F9.6 | Release candidate and adoption patch | F9.1, F9.2, F9.3, F9.4, F9.5 | — | — |
| Qualification and adoption | F9.7 | Approved adoption and retirement | F9.6 | — | — |

## Initial dispatch frontier: nine parents

The graph has nine initial work packages with no `start_after` prerequisites: **INT.1, BND.1, BND.2, F7.1, and F9.1–F9.5**. Their outcomes differ: INT.1 and boundary/viewer proofs unblock consumers; F9.1–F9.5 can start transferable-test inventory, paired verification preparation, assistive-technology evidence planning, compatibility/offline preparation, and performance/resource qualification in parallel. F9 preparation is not final qualification or release acceptance: F9.2–F9.5 retain their explicit `acceptance_after` joins, and F9.6 still waits for all five qualification parents.

## Pull scheduling and order of work

Once INT.1 is reviewed and integrated, it directly unlocks 13 more parents: INT.2, F2.1, F2.3, F3.1, F4.1, F5.1, F5.6, F5.7, F6K.1, F6C.1, F7.2, F7.4 and F8.1. Together with the eight other initial packets, that makes 21 start-eligible parents at that point, not 21 concurrent workers. This is a continuous pull queue: start any packet whose own `start_after` is satisfied as an owner and review slot becomes available. Do not hold all work for a whole tranche or milestone. Later acceptance joins remain enforced at each parent.

Prioritize the visible library, panels/drawers and Layout tree first (F2.1, F2.3, F3.1), then pull Parts (F4), PCB (F5), Keymap/Keycaps (F6), Case/viewer (F7) and Export (F8) packets as their own prerequisites clear. F9 test and resource work proceeds alongside feature implementation; its final joins continue to gate qualification and adoption.

## Capacity, ownership and review

There are four total concurrent slots including the coordinator: **root + at most three agents**. Normal steady state is two Luna feature authors and one Luna verifier; rotate an available slot to Astra High for independent review as evidence lands. Use Astra Extra High only for difficult unresolved cases. Slots are reused continuously; the portfolio does not imply that all 62 tasks execute simultaneously.

The coordinator owns shared presentation/runtime/shell, global CSS, build files, shared control contracts and the portfolio/dispatch ledger. Feature authors edit their explicitly owned private modules and hand off bounded shared changes for coordinator integration. Keep overlapping shared-file edits serialized.

This view is derived from the [62-parent graph](tasks.json), whose start and
acceptance edges remain authoritative, and the [first-wave child proposal](../dioxus-frontend-tranche-1/proposal.json).
The coordinator verified all four parent-to-child mappings against that saved
proposal. The [model policy](AGENT-ROUTING.md) still governs assignments.

Eligible work packages require bounded, source-checked child tickets before
implementation dispatch. The remaining 58 have not yet received that decomposition;
they must not be dispatched as 58 whole-workspace prompts. The user authorized implementation and automatic next-ticket publication on
2026-10-02; the first twelve child tickets are published. This view changes neither
that approval state nor parent task statuses, and authorizes no API or cutover
changes. Planning/review can prepare the next independent tickets while current
implementation and verification slots are occupied.
