# Keycaps Dioxus parity reset audit

**Scope.** Read-only, exploratory comparison of pinned React source `5a472a9426e6e38993361da402cd4ec730feb369` and Dioxus integration worktree `codex/rust-v1-ui-parity-20261001` at `89b1de8a28fdf02db91d972c90a69235bfbbbffb`. Routes tested were React `http://127.0.0.1:5175/` and Dioxus `http://127.0.0.1:34687/`. The named browser session and exact action trail are in [action-trail.md](action-trail.md). No repository source was changed. The two opened REVIUNG documents were not proven byte-identical; all browser findings are reconnaissance, not paired acceptance evidence.

## Observable gap

The React Keycaps workspace is a real editor and preview: board defaults, matrix profile/socket and row/wall settings, selected-key legend/color/profile/socket/row/width/depth overrides, clearance findings, physical 2D labels/caps, 3D assembled caps and STEP export. It also exposes Fit/Top/Bottom/Isometric, display modes, layer visibility and profile dimensions.

The Dioxus **Keycaps** tab is an explicit placeholder: “Keycap editing is not available yet in the Rust interface.” The global Dioxus Export menu contains generic **Export archive** and **Export STEP**, with no keycap-specific route/readiness. In Layout, expanding the right-keys tree and selecting a key updates the selected context and gives X/Y position inputs plus Apply position; it does not expose the React Keycaps editor or its width/height/rotation/key-assembly inspector. Thus Keycaps is not merely a missing menu item: the entire Keycaps presentation route, physical 2D projection, settings inspector, fit presentation, and dedicated preview/export journey are missing from the tested Dioxus UI.

React screenshots cover the populated keycap surface, selection, inheritance/edit controls, 3D-ready assembly, fit and export: [2D workspace](react-keycaps-view.png), [selected key](react-key-selected-sw8.png), [profile/socket details](react-profile-dimensions-open.png), [key overrides](react-keycap-overrides-open.png), [3D assembly loaded](react-keycaps-3d-settled.png), [keycap STEP action](react-before-step-export.png). Candidate screenshots: [placeholder](dioxus-keycaps-view.png), [generic Export menu](dioxus-export-menu.png), [Layout matrix context](dioxus-matrix-selection.png), [Layout key context](dioxus-key-selection.png).

## Capability classification

| Capability | Classification | Source evidence / boundary |
| --- | --- | --- |
| Keycaps tab, board/matrix/per-key settings UI, physical legend view, fit/finding UI | **Missing UI** | Dioxus `web/src/presentation.rs` routes Keycaps to `PlaceholderWorkspace`; observed public route is blank. React `app/src/ui/KeycapPanel.tsx` implements the editor; `app/src/ui/KeymapLayout.tsx` owns Keycaps mode. |
| Stable document settings, existing edit operations and resolution | **Implemented, unwired** | Core model has `KeycapConfiguration` and `SetKeycapBoard`, `SetMatrixKeycaps`, `SetKeycapKey`; `core/src/keycaps/edits.rs` applies edits and `core/src/keycaps.rs` resolves specs, validation and clearance. Core dispatch handles `ResolveKeycaps` in `core/src/lib.rs`. Dioxus has not wired these into a Keycaps view/controller. |
| Projection + command/history + async revision/scope adapter | **Missing private frontend adapter** | React supplies the workflow; Dioxus has no Keycaps query/edit controller or selected-key canvas projection. Existing Core contracts are enough for standard persisted values. Reuse accepted document and current session selection/history; do not make a second store or widen public APIs. |
| 2D key sizing/reflow and linked-layout behavior | **Missing UI/private policy adapter** | React has `app/src/ui/planKeycapResize.ts` and `app/src/ui/keycapReflow.ts`; source contract calls out the TS-only policy and F3 callback boundary. This is a UI-owned policy port, not a new Core geometry API. |
| 3D generated keycap CAD / dedicated STEP | **Missing Dioxus host/CAD adapter; not an absent domain feature** | React `app/src/exports/keycaps.ts` resolves specs then invokes its `CaseClient.keycaps` worker path; `cad/src/index.ts` calls `kernel.build_keycaps`. F6C.5 explicitly records the missing private Dioxus host/worker request adapter. Core resolution is present. Do not call generic Dioxus Export STEP proof of keycap STEP, and do not duplicate CAD/viewer logic. |
| Shared 3D rendering | **Existing shared viewer dependency / consumer integration missing** | F7.3 owns the shared scene/render/model delivery. Published F7.3 issue 05 already defines Keycaps consumer projection, selection mapping, suppression, and conditional Case overlays. F6C.5 owns Keycaps inputs and STEP call only. |
| Underlying profile/clearance semantics | **Existing capability** | Existing Core resolver includes supported profiles/mounts, inheritance, validation and conservative clearance findings. The UI must surface its existing outputs and wording; no new profile/fit semantics are indicated. |

The **Keymap** workflow remains a separate F6 stream. Keycap legend inheritance can consume the existing Keymap binding projection, but logical bindings/layers/macros/encoders are not part of Keycaps. Existing Layout key sizing/orientation remains F3-owned; Keycaps edits must use the existing shared sizing/linked-reflow seam rather than create parallel key geometry or selection state.

## Source evidence

The root source repository’s CodeGraph exploration of `KeycapPanel`, `KeymapLayout`, keycap resize/reflow and keycap export located the React behavior. The integration checkout has no CodeGraph index, so targeted `rg` and source reads were used. Relevant source paths:

- React `app/src/ui/KeycapPanel.tsx`, `app/src/ui/KeymapLayout.tsx`, `app/src/ui/createKeymapWorkspace.tsx`, `app/src/ui/keycapReflow.ts`, `app/src/ui/planKeycapResize.ts`, `app/src/keycapSettings.ts`, `app/src/exports/keycaps.ts`.
- Dioxus `web/src/presentation.rs` (Keycaps placeholder and existing Layout keycap overlay), `web/src/presentation/keymap/view.rs` (Keymap projection, not Keycaps editor), `web/src/cad_jobs.rs` (generic CAD requests, no Keycaps operation), `core/src/model.rs`, `core/src/keycaps.rs`, `core/src/keycaps/edits.rs`, `core/src/lib.rs`, and React-side `cad/src/index.ts`.
- Existing decomposition: `.scratch/dioxus-frontend-v1/issues/06-keymap-keycaps.md`, `.scratch/dioxus-keycaps-projection/issues/01-keycaps-projection-2d.md`, `.scratch/dioxus-shared-viewer/issues/05-shared-viewer-keycaps.md`.

The currently published F6C.1 ticket is already a bounded, useful first slice: actual physical 2D canvas, stable shared selection from canvas/search, effective dimensions/color/legend, explicit blank vs inherited legend, empty/mixed/standalone states, no persisted defaults, and no CAD prerequisite. Recommend preserve that ticket and clarify that its visible selected-key summary is an Inspector/read projection only; no fabricated or disconnected edit controls. Real setting writes belong in F6C.2. Root/coordinator owns module registration, route mount/shared selection wiring/global CSS.

## Existing 62-parent / child mapping and gates

The canonical `tasks.json` graph remains 62 parent work packages; the current published portfolio reports 48 ticket records (47 non-superseded). Do not add a parent or alter canonical edges. F6 explicitly has separate Keymap and Keycaps streams. The existing Keycaps child map and gates are:

| Child | Vertical outcome | Start prerequisite | Later acceptance join |
| --- | --- | --- | --- |
| F6C.1 | Board-scoped physical 2D view + shared key selection | INT.1 | F3.1 |
| F6C.2 | Board/matrix/per-key profile, socket, row, color and legend editors | F6C.1 | none |
| F6C.3 | Shared key-size drafts, selection scope, linked reflow | F6C.1 | F3.2, F3.5 |
| F6C.4 | Existing ResolveKeycaps findings and navigation | F6C.2 | INT.2 |
| F6C.5 | Shared F7 viewer consumer, keycap preview and dedicated STEP | F6C.4 + F7.1 | F7.3, F8.2, BND.1 |
| F7.3 / issue 05 | Common viewer’s Keycaps consumer projection/picks/models/overlay rules | F7.1; common model bridge landing is implementation coordination | INT.2, BND.1; F7.8 later adoption joins F6C.5 |

These are start-vs-acceptance distinctions, not full milestone blockers. F6C.1 can begin from accepted snapshots after INT.1; F3.1 is its acceptance join. The 2D/editor work must not wait on the CAD/viewer path. `F6C.4` keeps INT.2 as acceptance, and F6C.5 keeps F7.3/F8.2/BND.1 as acceptance joins. F7.3 remains blocked only by F7.1 and keeps INT.2/BND.1; issue 05 does not close it. F7.8 remains the later cross-workflow acceptance. F3.7 and F9.2 keep their canonical joins; F6C.3 specifically joins Layout after F3.2/F3.5.

## Paired journey and acceptance evidence still needed

Use one pinned, byte-identified saved project/archive in both applications, not the exploratory separate REVIUNG entries used here. Exercise in both public routes: open board; select same key by canvas and search; inspect effective/inherited and explicit-blank legend; change board, matrix and per-key settings; exercise Layout width/height/orientation and linked reflow; inspect clearance findings and navigate affected selection; switch 2D/3D and fit; retry missing/error model as applicable; export keycap STEP and validate bytes; Undo/Redo, save/reopen and compare accepted document/revision, resolver output, selected IDs and download result. Include compact/desktop and themes as scoped; keyboard/focus and errors/loading/stale-result teardown; retain native/WASM/build checks under the integration owner. This audit does not claim those paired gates pass.

### Refactoring record

RF-005 handoff: the Keycaps workflow spans the existing Core edit/resolve contract, TS-owned 2D/reflow controller, private CAD worker and common F7 viewer. Keep those seams explicit; do not force all responsibilities into one panel/controller or widen APIs to compensate for missing private wiring. No new architectural refactoring takeaway observed beyond that evidence-backed seam clarification. Keep the published RF register authoritative.
