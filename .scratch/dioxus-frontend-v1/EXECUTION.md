# Frontend workflow execution and agent ownership

**Scope:** complete the existing React interface in Dioxus, including UI-owned TypeScript controllers, styles, themes, assets and interaction behavior. **Planning baseline:** `c827c4e6`; executable source `f44a3d1b`; React reference `5a472a9426e6e38993361da402cd4ec730feb369`. F1 and F3a are verified increments. Full F2–F9 remain open.

The user requested multiple agents for each workflow on 2026-10-02. The plans were drafted by workflow specialists and checked by a different agent. Implementation uses the same separation: a workflow author, independent behavior/visual verification, and independent Standards/Spec review. Roles rotate through the available slots; they do not require all specialists to run at once. Planning review is not implementation acceptance.

## Dispatch and completion

[`tasks.json`](tasks.json) is the slice dependency graph. `start_after` gates implementation dispatch; `acceptance_after` adds the real integration joins required before completion. `depends_on` is their union for conservative schedulers. An empty `start_after` permits fixture-backed/private-module work, not a claim that final acceptance can run yet. `external_gates` identify actual host/approval conditions that elapsed time cannot satisfy. Parent milestone rows are acceptance summaries, not blanket locks on all child work.

Each task has a concrete user-visible outcome, priority, relative effort, owner, integration boundary and acceptance oracle. S/M/L estimates express relative implementation/verification size, not calendar promises. Source investigation can proceed read-only before dispatch; edits and verification use the declared task boundary. A task progresses planned → implementing → implemented → verified → accepted only with the corresponding source and evidence. No workflow is complete because its tab exists.

Before implementation, record the exact baseline and task IDs, inspect overlapping changes, and establish the smallest component/read-model/callback seam in the existing private presentation modules. Keep one application session, one authoritative document/history path and current worker/storage identities. The coordinator integrates shared edits serially. Do not invent a general UI framework before the first useful screen.

## Team boundaries

| Lane | Author owns | Independent verifier/reviewer owns | Shared handoff |
| --- | --- | --- | --- |
| F2 Projects/shared UI | Project library/lifecycle forms, panels/drawers, guide, shared controls/focus | Library/archive/recovery, resize, preferences, keyboard and paired themes | Coordinator installs shell routes, private lifecycle actions and shared components |
| F3 Layout | Tree, canonical board selection, matrices/placement/transforms, outlines/scripts, inspector/findings, 2D view | Geometry/history/draft/gesture traces, paired canvas/tree/inspector and 3D consumer | F6 provides shared key-size control; F7 provides common viewer |
| F4 Parts | Catalogue/definitions/generator forms, library profiles, assembly recipes and isolated preview inputs | IDs/assets/source fidelity, form failures, preview races, save and placement integration | Shared artifact/assets adapter; F3 placement; F5 mounted-module consumer; F7 viewer |
| F5 PCB | Host/module layers, wiring/pins/protection, mounted modules, physical instances, routed references | Electrical outcomes, scope/reflection, source-owned copper, findings, import and history | F4 definition/profile components, F6 firmware control, F7 physical/viewer consumer |
| F6 Keymap | Logical layers/bindings/macros/encoders and legacy quick control | Stable IDs, legacy behavior, generated firmware, validation/reload/Undo | F5 electrical/hardware inputs; shared artifact/download adapter |
| F6 Keycaps | Physical profiles/legend/color/size, existing TS resize/reflow port, fit and local STEP action | Inheritance, linked halves, geometry/fit, drafts and exports | F3 Layout control callback; F7 common viewer |
| F7 Case/common 3D | Case controls and the single shared assembly/model scene/view adapter | Scope, materials/model transforms, camera/picking, cancellation/readiness, GPU/listener/worker cleanup | F3/F4/F6 provide their view inputs; F5 supplies physical instances and routed references |
| F8 Export | Export workspace/options/readiness/return and shared private export orchestration | Snapshot/current-scope outputs, actual formats, errors/retry/cancellation where offered, URLs/downloads | F2 portable archive, F6 local export consumers, F5/F7 inputs |
| F9 Qualification | Coverage ledger, maintained public scenarios, final evidence and adoption/rollback patch | Cross-workflow visual/interaction/AT, compatibility and applicable resource/performance evidence | Coordinator integrates final route/build changes; user approves concrete production cutover |

The coordinator alone edits shared `web/src/presentation.rs`, `web/src/runtime.rs`, module registration, global theme/layout CSS, shared build/manifest wiring, run ledgers and final integration. Feature authors own named private modules and feature-local styles. A feature needing a shared edit sends a bounded patch or describes the change to the coordinator; it does not race another owner on those files. Do not widen public member/API visibility to bypass a private adapter problem. Routine private extraction and composition using existing public providers is within scope.

A verifier can author public browser scenarios while its paired implementer writes the private feature. Before reporting acceptance, a different agent reviews the integrated source and evidence for Standards and Spec; the author cannot approve its own work. Keep the four-slot cap: root integration plus up to three active agents. At the review stage free an author slot and rotate in the reviewer. Do not create user-owned chats for these subtasks.

## Visible delivery order

1. **Finish the usable frame and Layout navigation.** First parallel tranche: F2.1 project cards/search, F2.3 resizable panels/drawers, F3.1 actual object tree and selection. Integrate through the minimal shared seam and publish one runnable candidate with paired desktop/compact captures. Preserve the already restored keycaps, five layers and Footprints state.
2. **Replace placeholders with working authoring surfaces.** Rotate Parts catalogue/import, PCB host layers/wiring and Keymap/Keycaps saved-fixture controls alongside Layout matrix/outline/inspector work. Run F7 common-viewer mapping and BND.1/BND.2 provider/snapshot investigations early so true renderer gaps cannot remain hidden until release. Each increment must contain a complete useful action, its invalid/error state and normal history/save behavior.
3. **Complete the dense editors and shared 3D view.** Integrate Layout transforms/scripts/refinements, Parts generators/recipes/profiles, mounted modules/pins/physical instances, Keymap macros/encoders, Keycaps size/fit, full Case settings and common model/material/camera/picking behavior. Join only the affected dependency slices; unrelated 2D editors continue while a 3D or hardware join is pending.
4. **Complete output and end-to-end use.** F8's navigation/readiness shell and adapter work can begin on fixtures earlier. Qualify each actual format when its real provider inputs and workspace controls are available, then run complete new/import→edit→inspect→save/reopen→export journeys.
5. **Qualify frontend v1 and prepare adoption.** F9 runs continuously; its final joins establish complete inventory coverage, UI/theming/keyboard/actual AT, compatibility/offline and applicable performance/resource results. Prepare an exact default-entrypoint/retirement patch with rollback. Request production-cutover approval only when that patch and evidence are reviewable.

This ordering prioritizes visible frontend progress. It does not schedule engine, CAD kernel, generator or native-host rewrites. Retained providers remain accurately described as retained services; frontend v1 does not mean the wider runtime is already entirely Rust.

## Adapter and policy register

| ID | Verified current state | Smallest frontend work / owner | Dependent work |
| --- | --- | --- | --- |
| A1 Shared presentation seam | Existing single Runtime/Session, read model, navigate/select/edit events; monolithic composition | Coordinator publishes private workspace inputs/callbacks, draft/scope key, theme/panel slots; use existing types | All workspace composition; independent private forms can start on immutable fixtures |
| A2 Project lifecycle | BrowserStore list/load/save/delete/active ID, existing archive pack/unpack and Open/Recover contracts | F2 + coordinator private create/delete/portable-copy/status adapters, reference project-name filename and optional used bundled-model embedding; local assets always included; F8 owns its copy/checkbox controls | F2.2 and F8 whole-project copy consumer |
| A3 Artifacts/assets/downloads | CoreWorker generic artifact dispatch and existing core/CAD requests, BrowserStore assets; Runtime lacks a complete typed workspace facade | INT.2 adds the small shared dispatch/identity/file/asset/delivery seam; consuming visible slices add their own bounded typed calls incrementally with stale/error/cancel/cleanup checks. This is not an all-provider prerequisite | Parts imports/extraction, PCB board reference/modules, keycap/firmware queries, Export |
| A4 Common assembly/model viewer | RendererHost mount/update_scene and lifecycle exist; camera/picking/composition are not a complete workspace facade | F7.1 maps already exported wasm state/handle/pick/scene/camera operations to crate-private host wrappers; F7.3 implements one shared viewer. Keep actual public-contract gaps separately blocked and continue unaffected controls | Layout canonical board, isolated Parts preview, Keymap/Keycaps view and Case physical instances |
| A5 Retained generators | F3a packages unchanged Ergogen catalogue/generator service and Rust SVG projection | Reuse that service for generator parameters/compile and export preparation; document purpose, inputs/outputs, ownership/errors/cancellation/cleanup/cost/retirement. No generator rewrite | Parts generated preview and PCB/export compilation |
| A6 Remaining TS interaction policy | Resize/reflow, assembly preview orchestration and other UI helpers still have TS implementations | Owning frontend lane ports UI-owned state/policy to private Rust controllers, reuses domain authorities and retains behavior tests; F9 audits transitive UI imports | F6 sizing/reflow, F3 gestures, F7 preview, all inventory closure |
| A7 Accepted gesture mismatch | React component drag begins on non-zero world movement; current Dioxus waits 4 CSS px | F3.3 records paired small-drag/stationary-click/panel-reflow regression then ports reference behavior while preserving capture/cancel/history/performance gates | Complete Layout parity; F3a retains only its already tested claims |

| A8 Keycap CAD handoff | Core resolves keycap specs/findings; current Rust web CAD operations do not construct keycap meshes/inlays or STEP, while the existing packaged Rust WASM build_keycaps export does | BND.1 proves a crate-private worker path to the already packaged build_keycaps export and records identity, chunk/yield cancellation and lifecycle; F6/F7 integrate it. Any actual new public contract requires its own concrete decision | Keycap preview/STEP parts of F6C.5/F7.3/F8.4; 2D settings stay actionable |
| A9 Export-owned commits | React ExportContext adopts its own wiring/protection commits; Session export captures an immutable token without an adopt transition | BND.2 proves private orchestration or records a precise contract gap; F8.2 retains ordering and stale guards | PCB handoff generation/delivery; independent export UI can proceed |

No new public API is approved or assumed by this plan. BND.1 proves a private path to an existing Rust WASM keycap export; BND.2 resolves export-owned commit lineage. These are bounded frontend integration tasks with explicit outputs, not backend rewrite milestones. A4 is an early contract check, not a reason to block project panels or existing 2D workflows. If a public service capability is actually absent, record the smallest concrete proposal and affected task; continue work with existing boundaries. Do not replace the missing feature with a permanent placeholder.

## Evidence and handoff per slice

A task handoff names its baseline, exact source/diff and owned files; the reference fixture/action trace; initial missing/failing behavior when correcting a bug; affected native/WASM/build checks; paired browser captures for meaningful states in light/dark and desktop/compact; keyboard/focus and axe observations; output/history/storage assertions; independent Standards/Spec reviews; and unresolved limits. Include request/scope identity, stale/error/cancel paths when an async adapter changes.

Use `agent-browser` for browser work. Keep task browser sessions and test stores isolated. Reuse sufficient unchanged evidence; rerun affected checks after integration changes. CAD-heavy functional tests stay serial under the existing project policy even when authors work in parallel. A build passing is not visual or workflow acceptance.

The existing page-only build helper may reuse maintained providers only while its source/hash guard passes. Any changed provider inputs require the affected provider rebuild and browser checks. Keep the exact root and `/boardstudio/` asset/deployment behavior. Do not publish a fresh demo label against an older source artifact.

Actual screen-reader testing remains an external host gate; keyboard/axe/AX trees do not close it. Carry the existing frozen UI/live failures, ineligible CAD comparison and unperformed resource/material evidence accurately. Determine their relevance to changed frontend paths under F9; neither silently waive them nor replace frontend work with unrelated backend optimization.

## Source accountability

[`coverage.json`](coverage.json) assigns a lead and consumers to all 63 production TSX files, 15 TSX tests, the benchmark, all 18 CSS files and inventoried hooks/assets. The existing [inventory](evidence/tsx-inventory.json) preserves source hashes and F1/F3a evidence. Workbench/main are shared responsibility maps; Export behavior embedded there is explicitly owned by F8. No file is marked fully ported by this planning run.

F9.1 follows transitive UI-owned TS imports and records their replacement or retained-service disposition. The initial helper list is not a claim that every TS dependency has already been audited. A source row closes only with the owning public behavior and integration evidence, not by deleting or renaming its React file.

## Refactoring takeaways during implementation

The user requested a living record of architectural, design, theoretical and general software-quality issues encountered during the rewrite. Update [post-port takeaways](../../docs/migration/POST-PORT-REFACTOR.md) and the [RF register](refactor-findings.json) at every workflow handoff/review, or state that no new takeaway was observed. Record evidence and uncertainty, impact, current mitigation, later proposal and validation. F9 carries the accumulated register into the major post-port refactoring phase. Required correctness stays in the current slice; broader structural redesign is deferred without waiving acceptance gates.

### Crate boundary checkpoint

The page binary's Runtime/presentation and the publicly imported `boardstudio_web` library are separate crate boundaries. A `pub(crate)` method added to the library cannot be called by that binary. F7.1/INT.1 must choose private wrapper placement in the consumer's crate or an existing sufficient public facade; prove the call path before declaring the adapter ready. Do not silently make methods public to bypass this constraint. See RF-002 in the post-port register.
