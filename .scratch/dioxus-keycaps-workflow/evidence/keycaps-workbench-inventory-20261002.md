# Keycaps workbench parity inventory — 2026-10-02

This source audit distinguishes existing Keycaps work from the remaining vertical frontiers. It preserves the F6C parent and its 62-item graph; this document creates no parent completion evidence.

The paired Layout size/reflow replay is recorded separately in [Dioxus replay evidence](size-reflow-paired-dioxus-20261002.md); it verifies F6C.3 behavior only and is not Keycaps workbench acceptance.

## React contract

The pinned reference is React HEAD `5a472a9426e6e38993361da402cd4ec730feb369`. `app/src/ui/KeycapPanel.tsx` composes Board colors, Matrix profiles, Selected key, Clearance findings, and Export keycap STEP. It passes `FindingList` an `onShowFinding` callback. `FindingList.tsx` presents target labels and adds `Show outline` or `Select affected geometry` when `findingTarget` resolves a target. `Workbench.tsx:961-994` handles navigation: it selects the target board/workbench and geometry, fits the camera, reveals the relevant Inspector, records the focused finding, and focuses an Inspector control. `Workbench.tsx:1325` supplies the shared 2D, 3D assembly, and Footprints choices; Keycaps uses the 2D canvas and shared assembly view. `createKeymapWorkspace.tsx:84-86` connects the Keycaps STEP action and filters findings to the keycap resolver.

## Current Dioxus surface

The current implementation composes board settings, matrix-profile settings, matrix and key lists, a selected-key summary/editor, and current/stale/error/retry fit findings. Board colors and Matrix profiles are disclosure sections. The Keycaps canvas projects physical caps and exposes keyboard/pointer selection and canvas zoom/pan. Workspace composition dispatches Keycaps separately from Layout. Fit findings are severity-sorted, deduplicated and grouped, and their labels are projected from document targets.

The demonstrated F6C.4 gap is actionable target navigation: `KeycapsFitFinding` renders severity, message and the fitted-outline explanation but has no button and `InspectorInput`/`KeycapsFitInspector` has no navigation handler. In source commit `9d34f367`, the root has tree-navigation events and a configured-board callback, but no general finding callback matching React `Workbench.showFinding`. Labels alone do not satisfy the parent story “navigate from a warning to its target.” The separate overlap warning in Layout is F6C.3 and does not replace Core fit finding navigation. This is a private composition capability gap; a Keycaps child must coordinate the minimal root-owned wiring and must not invent a parallel selection/camera owner.

## Remaining slices and boundaries

| Area | Current evidence | Remaining child frontier | Existing owner/gate |
| --- | --- | --- | --- |
| Board colors, matrix profiles, per-key fields | Dioxus mounted editors and Editor-owned operations exist; issue01/02 specify accepted update, lifecycle and disclosure behavior. | Finish their paired browser edit/history/reopen gates; retain existing issues 01/02. | F6C.2, then F6C.4 starts; no duplicate settings ticket. |
| Keycap fit findings | Current resolver, request currentness, failure/pending retention, retry, status text, dedupe/order/grouping exist. | Add target buttons and route each finding through the existing accepted navigation authority; preserve stale/current revision feedback and safe missing-target behavior. | New child 05 under F6C.4; F6C.4 acceptance remains INT.2. |
| Selected-key organization | React wraps search, selected-key picker and selected override fields in one default-open disclosure; Dioxus currently renders key list, selected summary and editor separately, plus a standalone Matrices list. | Child 06 places the React controls in the same dynamic-title disclosure, removes the two Dioxus-only regions, and checks keyboard/collapse/selection behavior without moving edit ownership. | Existing F6C.2 / accepted F6C.1 selection start; RF-001 carry-forward. |
| Keycap 2D/3D and footprints | React offers 2D, 3D assembly and Footprints in the shared view control; Dioxus uses the shared view controls and F7 viewer integration. | Verify same-board paired toggle, footprint visibility and revision-current 3D state. Do not build another viewer. | F6C.5 and F7.1/F7.3. |
| Keycap preview and STEP | React offers an explicit `Export keycap STEP` action. The F6 boundary audit already found the CAD worker/provider integration frontier. | Wire actual cap/legend meshes, retry/error/stale behavior and downloadable STEP through the existing private CAD seam. | F6C.5/F7.3/F8.2/BND.1; no duplicate issue. |
| Top/left/right contextual shell | React shares its workbench shell while changing contextual controls/panes by mode and selection. Dioxus has private per-workbench composition dispatch. | Paired browser audit should verify Keycaps route label, left object tree, top view controls, Inspector sections, focus and narrow viewport. | F2 shared-shell integration; only create a Keycaps child for a demonstrated gap. |

## Refactor carry-forward

No new structural finding is asserted by this inventory. Keep the already tracked RF-001 contextual/shared Inspector composition and RF-005 distinction between frontend overlap semantics and Core swept-clearance findings. The immediate missing finding action is a parity defect to complete before v1, not deferred refactoring. The F6C.4 navigation action must reuse the accepted root/workspace navigation owner; adding a second target resolver or independent selection authority would create a new design defect.

## Source paths

- React: `app/src/ui/KeycapPanel.tsx`, `app/src/ui/FindingList.tsx`, `app/src/ui/findings.ts`, `app/src/ui/Workbench.tsx`, `app/src/ui/createKeymapWorkspace.tsx`, `app/src/ui/InspectorSection.tsx`.
- Dioxus: `web/src/presentation/keycaps_workspace.rs`, `web/src/presentation/keycaps_fit.rs`, `web/src/presentation/keycaps_settings.rs`, `web/src/presentation/keycaps_scene.rs`, `web/src/presentation/workspace_composition.rs`, and `web/src/presentation.rs` workspace callback assembly.
- Parent: `.scratch/dioxus-frontend-v1/issues/06-keymap-keycaps.md` F6C.4 story 31, source map, and unchanged `INT.2` join.
