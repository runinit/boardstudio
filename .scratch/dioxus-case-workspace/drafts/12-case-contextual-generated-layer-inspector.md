# Case contextual generated-layer Inspector

**Parent workflow:** F7.2 Case contextual workspace and F7.4 mechanical settings.

## Problem

The Case tree can select a generated mechanical layer, but the current Dioxus right Inspector keeps showing the full MechanicalSettings form. The pinned React application changes that pane to a layer-specific editor. This makes a tree selection visibly meaningful and provides the relevant dimension/findings without requiring the user to search a large global form.

## Reference behavior

At React `5a472a9426e6e38993361da402cd4ec730feb369`, `MechanicalAssemblyPanel.tsx` branches on `selectedLayer`. For supported generated structural layers, it shows an `Assembly settings` return action, a contextual title, only the applicable dimension fields, the resolved thickness readout from the matching current generated body when that body exists, current mechanical error findings, and the existing display controls. When internal gasket is configured, Retainer/Bottom also exposes `Edit shared closure hardware`; activating it clears the selected layer and returns to Assembly settings. The React Case tree selection and Inspector use the same current board/physical assembly.

## Current candidate behavior

At Dioxus `9d34f3672f4f05cb3771bc5cd358508b13e9c31f`, `case_workspace.rs` passes the selected layer through the Case Inspector and provides scoped color/reset/visibility controls. `mechanical_settings.rs` only uses selected-layer state to highlight `ResolvedStack`; all global sections remain mounted. On the shared layered-Sofle archive, selecting Plate therefore does not produce the React layer-specific Inspector.

## Bounded design

Render a contextual Inspector mode for the existing standard generated structural layer IDs: `plate`, `pcb`, `plate-foam`, `bottom-foam`, `bottom`, and `retainer`. Keep authored Case body editing in its current owner. Keep gasket/support editing in Issue05/F7.4c. Use the current selected layer, accepted mechanical projection, Scope and existing settings request/outcome path. Do not make a second copy of MechanicalConfiguration or change ownership of the Case tree, Runtime admission, display preferences, or history.

## Acceptance

- Selecting a current supported generated structural layer switches Inspect from the global settings view to the matching contextual layer view. It shows the React label, the existing `Assembly settings` action, only the source-prescribed numeric fields, current mechanical error findings, and the same Display color/reset/visibility controls. When the matching current generated body exists in the accepted Case projection, show `Resolved thickness` using that body's producer-supplied thickness; do not derive or fabricate a fallback. With internal gasket configured, Retainer/Bottom also exposes `Edit shared closure hardware`; activating it clears the selected layer through the existing selection action and returns to Assembly settings.
- The field mapping matches React: Plate → plate thickness; PCB → PCB thickness; Plate foam → plate-foam thickness; Bottom foam → bottom-foam thickness; Bottom → bottom thickness, wall thickness and clearance; Retainer/top case → wall thickness and clearance. Use the existing accepted dimensions, units, input validation, per-field draft, submit and failure handling.
- The view is derived from the current accepted source and selected layer. Unknown IDs, stale scopes, wrong-board mechanical configuration, unresolved generated output, and a selection from another instance cannot expose an editor for stale values. Clearing selection or using `Assembly settings` returns to the global settings Inspector.
- Show the current mechanical error findings in React's `Fit issues` section via existing finding navigation; the source shows all mechanical error findings there rather than filtering by selected layer.
- Selecting a layer keeps its existing display preferences available. Color, reset and visibility remain view preferences and do not create document edits. Clicking a layer or display action does not change Case selection/document state beyond the existing selected-layer signal.
- Numeric edits commit through the existing scoped MechanicalSettings request/controller and canonical/physical-instance operation path. Preserve request identity, latest accepted configuration merge, feedback, normal undo/redo, saved status and reopen behavior. No new Runtime API, public model/schema change, CAD implementation, or second history owner.
- The paired browser journey uses the same `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df` archive in React and Dioxus, at matching board/instance and viewport. Resolve the stack, select Plate, change thickness through the contextual field, return to Assembly settings, select Bottom and verify its three fields, exercise a selected-layer finding where the fixture provides one, change/reset color and visibility, Undo/Redo the one document edit, save/reopen, and verify the same current layer context is used after reselecting it. Record source/build/profile identity and screenshots.
- Retain desktop/compact visibility and keyboard focus coverage from the Case contextual acceptance; do not claim F7.2, F7.4, F7.3 or F7.8 parent acceptance from this child.

## Capability start gate and parent joins

Start after the existing Issue09 Case selection-to-Inspector route and Case16 standard layer tree/shared-viewer mount are integrated, and the current MechanicalSettings field request/outcome capability is proven for the selected Scope. Use exact integrated capability evidence; do not wait on all of F7.3 or F7.4. The mechanical layer contextual view requires a current resolved layer projection, but does not own its production.

The canonical F7.4 full acceptance join to INT.2 remains unchanged. F7.3 remains joined to INT.2 and BND.1; F7.2/F7.8 and all other parent gates remain untouched. The 62-task graph is unchanged.

## Ownership and review

Feature ownership is the private Case Inspector/MechanicalSettings contextual leaf and its tests. Root retains Case workspace/shared panel composition, module registration and global styles. Do not edit root-owned presentation/runtime files. Ordinary bounded form implementation is Luna Medium; the existing per-field identity path is reused. Sol 6.1 High independently reviews the frozen contract and source/evidence.

## Refactoring ledger

No new refactoring takeaway observed in this bounded pass. Preserve relevant RF-001 and RF-006 references. Root owns the shared post-port ledger; report any distinct source-backed finding to root before changing it.
