# Dioxus frontend v1 — complete React interface parity

**Current priority: 100% frontend.** Updated 2026-10-01.
**F1 implemented and verified as the first increment; F2 is next.**
[Demo and evidence](../../.scratch/dioxus-frontend-v1/evidence/handoff.md).

The user clarified that this run is the Dioxus migration: rewrite and port every
existing TSX screen/component, its presentation behavior, theming and responsive
states. Backend engines, generators, CAD internals and output formats are not the
workstream. We reuse their existing service boundaries while replacing React.
A complete Dioxus frontend is the v1 target for this roadmap. The wider full-Rust
runtime destination remains recorded in ADR 0003, outside this frontend backlog.

## What v1 means

The existing React application can be replaced by Dioxus without losing a
supported frontend workflow. Layout, PCB, Keymap, Keycaps, Case, Parts and Export
are complete, along with project management, onboarding, themes, menus, panels,
canvases, inspectors, shortcuts, focus, errors and responsive behavior. No required
workspace remains a placeholder and no React island implements a required screen.
Every production TSX file is accounted for with a Dioxus replacement or a reviewed
consolidation. Tests and benchmark entrypoints have an explicit retained/replaced
disposition; deleting a filename is not proof of behavioral parity.

The visual/behavior reference is React `5a472a9426e6e38993361da402cd4ec730feb369`.
The Rust starting candidate is `f0ac0a19` (M1 production source `a49bb798`).
The [source inventory](../../.scratch/dioxus-frontend-v1/evidence/tsx-inventory.json)
and [reference notes](../../.scratch/dioxus-frontend-v1/evidence/reference-inventory.md)
track all 79 TSX files: 63 production components, 15 tests and one benchmark.
The inventory also accounts for 18 source stylesheets, UI hooks, fonts/assets and
adjacent presentation controllers. Re-inventory later reference changes before
accepting them.
The [parent spec](../../.scratch/dioxus-frontend-v1/spec.md) and
[local task graph](../../.scratch/dioxus-frontend-v1/PLAN.md) govern execution.

Reuse [CONSTRAINTS](../../CONSTRAINTS.md), the existing
[capability owners](../../CAPABILITY-MAP.md) and
[ADR 0003](../adr/0003-rust-application-ownership.md). Dioxus owns presentation
state; the existing Rust engine/session and services remain authoritative.
Port React hooks/controllers that own UI behavior into Dioxus/Rust as part of
their screens. Reuse existing domain calls rather than reimplementing algorithms.
Document any temporarily retained service adapter explicitly; calling it does
not justify describing that service as already migrated. No new API, schema,
visibility, budget or persistence cutover is implied by a frontend ticket.

## Phases and milestones

| Phase | Milestone | Visible deliverable | Exit criterion | Dependency |
| --- | --- | --- | --- | --- |
| 1 — recognizable application | F1: workbench shell and themes | Exact reference navigation, project menu, Objects/canvas/Inspect/footer, light/dark/system, compact navigation; working existing Layout/Case/Export, explicit placeholders for pending workspaces. | Matched desktop and compact captures, themes/preferences, reachable tabs/menu, preserved edits/history and functional existing paths. | M1 implementation |
| 1 — usable application frame | F2: projects, panels and shared controls | Complete project start/library/demos/create/rename/duplicate/delete/copy/import; onboarding, resizable panels/drawers, menus, shortcuts and focus restoration. | Paired project/panel/menu flows, persisted preferences, all loading/empty/error/recovery states, supported compact and zoom behavior. | F1 |
| 2 — complete authoring | F3: Layout, tree and inspector | Full matrices/components/relations/constraints, placement/transforms/snapping, outlines/cutouts/refinements/scripts, findings, layers, camera and selection UI. | Every reference Layout action through public services, same geometry/history/save results, matching inspector/tree/canvas and keyboard interaction. | F2 |
| 2 — complete library | F4: Parts and assembly UI | Search/browse/import/create/edit, generator parameter forms, footprint/model previews, module catalogue and assembly editor. | Every supported Parts action and failure state, preserved assets/IDs, same preview and service input/output behavior. | F2; integrates with F3 |
| 3 — electrical workspace | F5: PCB and hardware UI | Layers, modules/physical instances, controller/connectors/wiring/pins/jumpers, profiles, reference panels and located findings. | Paired electrical/module scenarios, complete controls and scope selection, same readiness and service calls. | F3, F4 |
| 3 — key workspaces | F6: Keymap and Keycaps UI | Layers, key/behavior search/editing, macros/encoders; profile/legend/size/fit editing and 2D/3D keycap presentation. | All reference-supported controls and saved settings, stable selection/history, keyboard workflows and matching preview. | F3, F4; F5 for hardware handoff |
| 3 — mechanical workspace | F7: Case and assembly UI | Full Case inspector/settings/construction/physical scope, generation controls, findings and reference 3D viewer behavior. | Paired complete settings/cancel/retry/preview paths, camera/picking/layers/material presentation, no stale/mis-scoped canvas or resource regression. | F3, F4; F5 for hardware-dependent views |
| 4 — complete frontend workflow | F8: Export UI and cross-workspace flows | Reference Export workspace, format/scope/readiness/progress/error/cancel/retry/download controls and return paths. | Every supported reference export reachable through existing providers with accepted snapshot identity; end-to-end project→edit→inspect→export. | F5, F6, F7 |
| 4 — frontend v1 release | F9: parity qualification and React entrypoint retirement | Complete Dioxus default frontend, documented migration/rollback, transferred frontend regression coverage. | Entire TSX/style/hook inventory disposed; no required placeholders/React islands; visual, keyboard, actual AT, theme, responsive, functional and affected performance gates resolved. | F1–F8 and applicable carried gates |

F1 is implemented first to show visible progress immediately. F2 completes the
application frame before feature density grows. F3/F4 are the next major ports;
F5/F6/F7 can then progress independently where their dependencies are available.
F8 joins the complete user journey. F9 qualifies the frontend and prepares a
reviewable entrypoint switch. Existing backend benchmark or generator rewrite
projects do not replace or consume this frontend sequence.

## Detailed slice order

**F1 — shell and theming.** Match the exact reference labels and order:
**Layout, PCB, Keymap, Keycaps, Case, Parts**, with **Export** at the right.
The current Design mode is called Layout in the UI. Move project discovery/import
into the project menu, put board/instance and component navigation in Objects,
position editing in Inspect, and let the canvas occupy the remaining viewport.
Reuse the bundled Source Sans 3/Source Code Pro, tokens, icons, spacing and focus
language. Include functional Light/Dark/System preferences and browser-scoped persistence.
Keep available Layout, Case and exports wired to existing services. Pending tabs
open clear placeholder panels with a return path and no pretend editing/readiness.
Tab changes preserve session/history and safely settle or retain drafts. This is
the only milestone where placeholder workspaces count as the requested deliverable.

**F2 — shared product UI.** Port ProjectStart/ProjectLibrary, setup/choice guides,
WorkspacePanel, CommandMenu, shared icons/controls/sections and their hooks.
Finish search and demo discovery, new/copy/rename/delete/reset with matching
confirmation and recovery, archive open/save, theme settings, panel modes/widths,
drawers/scrims, keyboard shortcuts and focus restoration. Match startup, empty,
loading, unavailable project, save failure, long names and compact states.

**F3 — Layout.** Port the complete Workbench tree/canvas/inspector orchestration,
not only component rectangles. Complete object groups/matrices/presets, direct
placement, transformations/relations/mirroring, constraints, selection/nudge,
align/snap, layers and camera/fit. Port outline tools, draft/control overlays,
snap guides, feature editor, fixed/linked refinements, cutouts and script editor
using existing domain services. Preserve preview/apply/cancel and Undo grouping,
located findings and 2D/3D switching. Split the large React Workbench by these
responsibilities rather than recreating one large Rust component.

**F4 — Parts.** Port LibraryWorkspace, PartsLibrary, parts inspector, assembly
editor/viewer, generator fields and model/footprint preview boundaries. Cover
search/categories, selection, import/create, supported parameter editing,
assemblies/modules, attached model loading/errors and keyboard navigation.
Existing generator and catalogue services remain underneath; rewriting generator
algorithms is outside this frontend phase. Preserve source/license/asset identities.

**F5 — PCB.** Port PCB workspace composition, WiringPanel, hardware readiness,
module overlays/previews/inspectors/profiles, input profile and board reference
panels. Include controller/topology/connectors, row/column/net/pin/jumper review,
physical boards and mounted modules, layer toggles, findings and location focus.
Compare the same multi-board and module projects with the React UI.

**F6 — Keymap and Keycaps.** Keymap sub-slices: layout/layer/key selection,
searchable binding/behavior editor, supported modifiers/tap/layer actions,
macros, encoders and firmware panel. Keycaps sub-slices: per-key/group selection,
size/profile/legend controls, fit/overlap findings and 2D/3D presentation. Preserve
legacy bindings, identities and UI drafts through existing engine contracts.
Do not add unsupported key behavior families or live device connectivity.

**F7 — Case/3D.** Port Case workspace/choice/inspector, generation controls,
mechanical assembly and hardware-instance panels, all supported settings and
construction editors. Match live/provisional/exact readiness, manual generation,
cancel/retry/error behavior, scoped selection, findings and camera/picking/layers.
Reuse the existing CAD and renderer; frontend mount/unmount/listener/worker/canvas
ownership must be verified. Theme the 3D presentation consistently with reference.
An M1 nominal PCB preview does not satisfy the full reference assembly UI.

**F8 — Export.** Inventory formats actually offered by the reference; port their
complete presentation and state flow: scope, options, readiness explanation,
progress, cancellation, retry, errors, return navigation and download delivery.
Use existing archive/geometry/PCB/manufacturing/firmware providers and accepted
snapshot contracts. UI parity does not authorize format changes. Qualify complete
cross-workspace journeys with the same project and service outputs.

**F9 — finish the port.** Audit all TSX files plus UI hook/state, CSS, font/icon,
theme, error-boundary and responsive behavior. Every row names its Dioxus owner,
reference scenario, evidence and final state. Reuse styles when suitable; CSS is
not application logic and need not be rewritten in Rust. Transfer valuable React
component and browser test coverage to public Dioxus scenarios before removal.
Verify dark/light/system, desktop/compact/short viewport/zoom/DPR, keyboard/focus,
actual assistive technology, loading/error/recovery, offline shell and required
performance/resource checks. Prepare a concrete default-frontend/retirement patch
with rollback; production entrypoint or data-writer cutover needs explicit approval.
Keep the reference recoverable until the replacement is accepted.

## How completion is measured

- **Inventory:** every production TSX file has a replacement or reviewed consolidation;
  each React test/benchmark has a coverage disposition. Also account for UI-owned
  hooks/controllers, CSS, fonts/icons/assets and theme preferences.
- **Behavior:** same input projects, actions, error cases, selection/history and
  service calls/outputs. A visible tab is not a finished feature.
- **Visual parity:** paired reference/candidate desktop and compact captures for
  each workspace, both themes and key open/selected/loading/error states. Preserve
  current geometry presentation and control placement, not just palette colors.
- **Interaction parity:** pointer thresholds/capture/modifiers, keyboard shortcuts,
  focus order/restoration, drafts, tab switching and menus all survive the port.
- **Architecture:** no required React UI remains; Dioxus presentation uses existing
  engine/session/provider authority without duplicate domain state or algorithms.
- **Evidence:** affected native/WASM/browser checks and Standards/Spec reviews pass;
  new checks or budgets do not replace existing approved gates without a decision.

Do not report a percentage by counting files alone: Workbench is much larger than
an icon helper. Milestone completion requires its whole user workflow and evidence.
The tracker records **planned → implementing → implemented → verified → accepted**;
blocked or failed gates remain visible. A first-shell demo is visible progress,
not a claim of complete frontend v1.

## Carried constraints and boundaries

[M1 acceptance](../../.scratch/m1-production/ACCEPTANCE.md) remains open. Its useful
source/build/offline/export and pointer evidence is reused only where unchanged.
Screen-reader interaction is blocked on the current host; the frozen reference
UI/live comparison has failures, the frozen CAD comparison is ineligible, and
some material/resource evidence is incomplete. Record which frontend gates are
affected without reopening unrelated engine optimization work in this run.

The 2026-10-01 clarification supersedes a backend-oriented full-Rust milestone
sequence for the active work. No generator rewrite, CAD kernel refactor, backend
performance project, new document format or native-host migration is part of this
frontend roadmap. Public service gaps discovered while porting a screen become
explicit dependencies with the smallest required adapter decision; never silently
replace an unfinished feature with a permanent placeholder.

## User correction: Layout layers and keycap rendering

The missing keycap outlines and layer/footprint controls are being restored now
as F3a, before the remaining F2 work. This corrects the first demo’s Layout
presentation without claiming the rest of F3 or 3D assembly is complete.
See `.scratch/dioxus-frontend-v1/issues/03a-layout-layers.md` and the retained
reference/red evidence under `evidence/layout-layers/`.
