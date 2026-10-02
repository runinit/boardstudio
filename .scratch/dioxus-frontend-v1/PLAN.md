# Dioxus frontend execution plan

**Updated 2026-10-02. Scope: 100% frontend parity.** F1 shell/themes and the requested F3a Layout correction remain verified increments. F2–F9 are still open as parent workflows. Thirty-nine bounded child tickets are published across the current frontier; the 62-parent graph and all acceptance joins remain unchanged. This is a living execution plan, not a completion claim.

[Milestone roadmap](../../docs/migration/DIOXUS-FRONTEND-V1.md) · [parent specification](spec.md) · [agent ownership and execution](EXECUTION.md) · [62-task graph](tasks.json) · [source coverage](coverage.json) · [agent models and routing](AGENT-ROUTING.md) · [planning reviews](evidence/planning/review-resolution.md)

Integration branch `codex/rust-v1-ui-parity-20261001`; planning baseline `c827c4e6`; current integration snapshot `3c83cc0f` with active coordinator/feature work; React reference `5a472a9426e6e38993361da402cd4ec730feb369`. Latest bounded artifacts are separately pinned to their producing sources: Case viewport repair `87cea512` (public 34669), Case viewport/44px/offline build `32695555` (public 34671), and Keymap bindings build `6509f557` (public 34673). Case compact and root/subpath offline checks pass. The focused binding public packet is complete with a focus-continuity difference retained; broader F6 acceptance remains open. The existing [editable demo/evidence](evidence/layout-layers/handoff.md) is unchanged.

## Phases and workflow teams

| Phase | Workflow and specification | Remaining visible outcome | Slices |
| --- | --- | --- | --- |
| 1 — usable frame | [F2 Projects and shared UI](issues/02-projects-shared-ui.md) | Project library/search/new/open/import/delete/portable copies, resizable panels, drawers, guide, shared controls and focus | 4 |
| 2 — Layout authoring | [F3 Layout](issues/03-layout.md) | Real object tree, matrix/component tools, transformations/constraints, outlines/refinements/scripts, inspectors/findings and 2D/3D | 8 |
| 2 — library authoring | [F4 Parts](issues/04-parts.md) | Catalogue/import/edit, generator settings, isolated previews, mechanical profiles and assembly/module recipes | 7 |
| 3 — electrical workspace | [F5 PCB](issues/05-pcb.md) | Host/module layers, wiring/pins/protection, mounted modules, physical instances and routed-board references | 8 |
| 3 — logical and physical keys | [F6 Keymap and Keycaps](issues/06-keymap-keycaps.md) | Separate teams for layers/bindings/macros/encoders and profiles/legends/colors/size/reflow/fit/preview | 10 |
| 3 — mechanical workspace | [F7 Case and common 3D](issues/07-case-3d.md) | Complete Case settings/generation and one shared assembly/model viewer for all consumers | 8 |
| 4 — complete journeys | [F8 Export](issues/08-export.md) | Exact offered export rows, readiness, project-copy options, current-scope downloads and return paths | 6 |
| 4 — frontend v1 | [F9 Qualification and adoption](issues/09-frontend-v1.md) | Complete coverage, paired visuals/interaction/AT, compatibility/offline/resources and reviewable adoption/rollback | 7 |

**The canonical backlog still contains all 62 work packages.** The original first-wave proposal produced twelve children; automatic, independently reviewed expansion has since grown the published set to 39 bounded tickets across the current frontier. Child work does not replace parent criteria. The [full backlog and parallel execution view](PARALLEL-EXECUTION.md) shows every parent, exact start blockers, later acceptance joins and current child coverage. Work flows continuously through the dependency frontier; the first tranche is not a barrier holding unrelated workflows.

The 58 workflow slices are supported by two small integration tasks and two early boundary tasks. They do not wait serially for whole preceding milestones. `start_after` in the graph gates dispatch; `acceptance_after` names later real integration joins. A conservative scheduler can use their union, `depends_on`. All workflow scopes require implementation and independent verification/review; planning review does not close them.

## Model allocation

Use **Luna Medium by default**, Low for fixed-oracle mechanical packets, and High for specified state/gesture/async/adapter work. Reserve **Astra High for independent reviews and behavioral bug fixing**, with Extra High for difficult unresolved cases. The 62 rows are work packages (31 L, 29 M, 2 S); dispatch smaller packets with exact ownership, source/fixture oracle and concrete callbacks. Shared boundaries get a reviewed call-path proof before implementation. Eleven available runtime slots do not mean eleven concurrent authors: keep review, public verification and integration capacity ahead of author throughput.

[Agent policy](AGENT-ROUTING.md) contains the first-wave packet map, cadence, escalation and launch rules; [machine routing](agent-policy.json) assigns every parent task. Fast/priority is the preference and host configuration is already priority, but the exposed spawn call has no tier override. This update does not change host configuration or a running model.

## First visible tranche

INT.1 is accepted and supplies the private shell/read-model/callback seam. The visible tranche remains **F2.1 project library**, **F2.3 panels/drawers**, and **F3.1 Layout tree/selection**, with bounded children already underway. Preserve default keycaps, all five Layout layers and the shared Footprints state already delivered by F3a. Recent panel-mode and tree hierarchy corrections have bounded public evidence, but their parent/workflow acceptance still has open joins.

Current pull work includes Parts/selected-context public proof, Keymap binding follow-up on focus continuity/full F6 gates, mounted Case mechanical settings controller/source verification and remaining shared-viewer public proof. BND.1 and BND.2 remain separately tracked boundary gates; do not imply the Keymap layer route or shared viewer closes either. F9 coverage/test preparation runs alongside feature work.

Each workflow uses a feature author plus a different verification/review agent. Root integrates shared runtime/shell/global CSS/build files; feature authors work in private modules with explicit ownership. With eleven available slots, root schedules disjoint authors while reserving independent verification, both review axes and serialized integration/build capacity. Author concurrency is capped by those queues, not by slot count. [Dispatch and ownership details](EXECUTION.md) describe exact boundaries and handoffs.


## Current execution frontier — 2026-10-02

- **F2.3 / T1-07:** desktop panel modes have bounded implementation and public checks; actual assistive-technology evidence remains open. This is not F2.3 acceptance.
- **F3.1 / T1-10:** hierarchy placement and unowned-component retention passed paired public checks at `515f390d`; the semantic ownership repair at `f65b0c83` has source review/build and zero candidate axe violations in the focused public packet. Typed selected-context summary and numeric public QA are mounted at `e510dd4c`. Full tree/canvas/Inspector, scope/reopen, stale callback, empty/disabled context, keyboard/compact/theme and actual AT evidence remain open. T1-11 owns outline/bridge navigation; T1-12 owns rectangular range and modifier semantics. The public gaps inspected so far fit these existing tickets; no duplicate F3.1 children are drafted. See [current frontier evidence](evidence/planning/current-frontier-20261002.md).
- **F4.1a / Parts:** left Objects catalogue, right Inspector and typed selected-context summary are mounted at `e510dd4c`; strict WASM/native checks, 14 integrated page tests, eight separate Parts-native tests, root/subpath/offline builds and targeted numeric public QA are retained. Independent public catalogue/source/failure/activation proof remains in progress; ARIA/contrast and actual AT limits remain explicit.
- **F7 / common viewer:** the `6468e80d` root/subpath/offline artifact is verified; bounded Case viewport checks on `87cea512` cover 1280×577, 390×844 and 760×844. Refreshed package `frontend-case-viewport-final-20261002` from `32695555` passes its compact 390×844/44px-control check and natural root/subpath offline reload on port 34671. F7.3 now has the existing Case issue 02 continuation plus four separately scoped Layout, Keymap, Keycaps and Parts sample tickets (03–06). Four new tickets take the total to 39; issue 02 is not counted again. All start from F7.1; INT.2/BND.1 remain F7.3 acceptance joins and F7.8 remains the later cross-workflow join. Model delivery, full consumer proof and parent acceptance remain open.
- **F6K.2 / Keymap bindings:** source `6509f557` is mounted through root; strict WASM/native Clippy, formatting and 30 native tests pass, and Astra source review is clear. The seven-command, 961-hash build is served at 34673 with its focused public packet complete: synthetic driver-select red was retired because React reproduces the same programmatic event ordering; native Tab during blur-save can move focus to BODY while controls are disabled, while post-idle pointer/native selection preserves Tap and Hold. This is a focus-continuity difference, not saved-data loss. Full F6 legacy/malformed/stale-race, broad semantics and actual AT gates remain open.
- **F7.4 mechanical settings:** the controller and mount are now present in the integrated commit `0cad7577`; strict WASM/native Clippy and 34 native checks pass. Build `frontend-mechanical-settings-20261002` completed all seven commands with 965 source hashes at `0cad75775e84aba13dc2f88d513b9f662e037e5e`; public verification is now underway at ports 34675 (root and `/boardstudio/`). Configure/edit/disable, saved payload, Undo/Redo, reopen and parent acceptance remain open. Macro editor source `ac43980c` is merged in `96874be6`, but its UI is not mounted or compiled.
- **F6K.2 and F7.4:** the binding editor is mounted and its focused public packet is complete; full F6 acceptance remains open. Mechanical controller/mount are integrated in `0cad7577`; the current owned closure source `61e8c43` is in `86a83ddf`, with 34 native tests including six closure and four feedback cases plus strict WASM/native Clippy. The fresh 965-hash package is under public verification. The older planner-only file prefix `f96b9b28` tested at `b25d6887` is historical. F7.7 child 02 remains a bounded default-instance repair and does not close its parent.
- **Portfolio:** 39 bounded tickets are published; all 62 canonical parent tasks and their start/acceptance edges are unchanged. Eleven runtime slots are available; active authoring is throttled to keep independent review, verifier capacity and serial integration ahead. The public API proposal remains unapplied pending the explicit decision; no parent is marked closed by this planning update.

## Dependency structure

```mermaid
flowchart LR
  I[Private shell and provider seams] --> UI[Parallel workspace UI slices]
  V[F7 common viewer] --> C[Layout Parts Keymap Keycaps Case consumers]
  K[Existing keycap CAD adapter] --> C
  UI --> C
  E[Export commit-lineage proof] --> X[Export formats and complete journeys]
  UI --> X
  C --> X
  Q[Continuous coverage and parity verification] --> R[Frontend v1 qualification]
  X --> R
  R --> A[Concrete adoption and rollback review]
```

This is a summary; the [acyclic slice graph](tasks.json) is the exact execution authority. In particular, Layout does not wait for all project-management features; Parts previews do not wait for custom definition editors; 2D Keymap/Keycaps do not wait for complete PCB or Case; and Case forms do not wait for all Layout/Parts work.

## Completion and known boundaries

- **UI parity:** all 63 production TSX responsibilities, 15 TSX tests, one benchmark, 18 stylesheets and supporting UI controllers/assets have assigned owners. Planning makes no new whole-file migration claims. F9 follows remaining transitive UI imports before release.
- **3D:** use one shared viewer. Layout/Keymap/Keycaps show the canonical board; Case uses its selected physical instance; Parts uses an isolated sample project. The private host/viewer path now builds in the 6468 artifact; full renderer lifecycle, picking and consumer acceptance remain open.
- **Keycap CAD:** Rust WASM already exports `build_keycaps`; the current Dioxus worker lacks its request path. BND.1 proves a private adapter, chunked preview cancellation and STEP delivery without new CAD algorithms or public API widening.
- **Export:** preserve the exact reference rows and readiness. PCB's own wiring/protection commits need a proven private lineage guard; the current immutable Session export token cannot simply span those commits. Portable copies must preserve local assets, optional used bundled models and the reference filename.
- **Release:** actual screen-reader evidence remains host-blocked; applicable carried performance/resource failures remain explicit. Production cutover/React retirement requires approval of the final concrete patch. Existing backend/provider rewrites and wider full-Rust runtime completion remain outside this frontend plan.

No required placeholder or React UI island satisfies frontend v1. Each accepted slice needs current source/build provenance, public behavior/output tests, paired visual/keyboard checks and independent Standards/Spec review. Preserve existing M1 evidence and its limits separately.

## Refactoring takeaways during implementation

The user requested a living record of architectural, design, theoretical and general software-quality issues encountered during the rewrite. Update [post-port takeaways](../../docs/migration/POST-PORT-REFACTOR.md) and the [RF register](refactor-findings.json) at every workflow handoff/review, or state that no new takeaway was observed. Record evidence and uncertainty, impact, current mitigation, later proposal and validation. F9 carries the accumulated register into the major post-port refactoring phase. Required correctness stays in the current slice; broader structural redesign is deferred without waiving acceptance gates.
