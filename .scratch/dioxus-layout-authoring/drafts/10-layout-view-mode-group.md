# Draft F3.6 view-mode child — Match the Layout view group

**Parent:** F3.6 — Camera, fit and Layout 2D/3D assembly. This is one local operational child of the existing canonical parent; it does not create or change a task-graph row.

**Reference:** React source `5a472a9426e6e38993361da402cd4ec730feb369`. **Dioxus baseline:** integrated command-pill source `31240d3a77d7225fefb0c9b5fbf1e2924ee28d9f` plus the coordinator's close-control integration `4c555ead` (full integration identity to be recorded by the coordinator). Paired menu journey and source seam inventory are recorded separately under evidence 09 and 10.

## What to build

Add the separate Design view group shown by React, in this order: **2D**, **3D assembly**, and **Footprints**. It sits beside the command pill and is not part of the command-menu owner. The Footprints choice is visible in 2D only; it reuses the accepted F3a visibility state and layer projection. Keep its value when entering 3D and restore it when returning to 2D.

In 2D, keep the existing Layout canvas and its session camera. In 3D assembly, render the selected board from the canonical Layout document through the single F7-owned common viewer/private adapter. The active mode remains Layout/Design: this is a view change, not navigation into Case. Do not pass the physical-instance Case document, call the Case-only capture projection, clone renderer code, introduce another renderer, or widen a public API. The exact canonical-board input and picking mapping must be the F7.1/F7.3-approved private seam; if that seam is not yet established, keep this child blocked and return the missing seam for contract review.

On entry to 3D, follow React's view-change path: cancel active placement and gesture/draft work, close command menus, clear transform-tool state, then change the view. Returning to 2D restores the existing Layout camera and selection without creating an edit, revision, or Undo entry. Changes in selected board, project/session owner, or workspace invalidate stale viewer output and release the prior viewer owner. View state remains presentation/session state and is not saved in the project document.

The 3D route must use the shared viewer's supported camera, fit, preset, orbit/zoom, picking, status and lifecycle behavior. Layout supplies canonical board identity, scene input and the mapping from a picked board reference to current Layout selection. It does not add Case-only bodies, handles, mechanical drafts, or physical-instance scope.

## Dependencies and ownership

- **Start after:** F3.1 selection and board scope; F7.1 common-viewer contract and renderer feasibility decision.
- **Acceptance join:** F7.3 shared viewer/private adapter, plus the existing F3.6 camera/view parent acceptance.
- **Parent:** existing F3.6 in the canonical task graph. Keep F3.6 and F3.7 open until their complete acceptance passes. Do not duplicate or edit any of the 62 canonical parent records for this local operational child.
- **Capability reuse:** existing F3a `LayerVisibility.footprints`, existing Layout session camera in `layout_camera`, existing Layout canvas/cancellation owner, and the one F7 common viewer. The React `AssemblyViewer` is the behavior oracle, not code to clone.
- **Boundary:** Layout-view controls and canonical consumer wiring only. F3.6 still owns the broader 2D camera/fit behaviors and the completed Layout view lifecycle. F7 owns renderer behavior and its adapter. Any real ABI gap returns to a separately reviewed contract; no new API is presumed.

## Acceptance criteria

- [ ] On one identical saved archive hash, compare React and Dioxus Layout at 1280×577 and 375×667, in Light and Dark. Show the separate 2D / 3D assembly / Footprints group, command pill, and corresponding 2D and 3D scenes. Record archive, source, build and browser identities.
- [ ] The view group has React's order, labels, pressed states, and visibility: Footprints appears only in 2D. Its existing F3a value survives 2D → 3D → 2D, and the same Footprints layer is visible/hidden on return.
- [ ] 2D → 3D uses the F7-approved canonical selected-board scene from the Layout document, without entering Case scope or consuming a physical-instance projection. A second-board fixture proves board isolation. Picking maps supported references to the matching Layout part/selection.
- [ ] 3D entry cancels the active placement/gesture/draft, closes any command menu, and clears Transform tool state with no partial document edit. Returning to 2D restores the prior Layout camera and current valid selection. View changes alone do not alter accepted values, document revision, or Undo/Redo history.
- [ ] Board change, project/session replacement, and leaving Layout invalidate prior async output, prevent stale picks, and dispose/unmount the viewer owner. Repeated switches remain clean.
- [ ] Verify the shared viewer's supported orbit/zoom, fit and preset controls, theme palette, resize/device-pixel-ratio, loading/error/context-loss and recovery behavior as supported by F7.3. No Layout-local clone of those controls or lifecycle.
- [ ] No duplicate Footprints state, renderer, camera store, saved view field, Case physical-instance routing, new public API, or F7-owned renderer behavior. Preserve F3a and F7 accepted contracts.
- [ ] Keep complete F3.6 camera/fit acceptance and F3.7 integrated Layout parity open; this child does not claim either parent complete.

## Out of scope

F3.6's complete 2D navigation and fit acceptance; F7 common renderer internals; physical Case assembly; Keymap/Keycaps/Parts viewer consumers; new scene schemas or public API; view-state persistence; any change to project document format; full Layout/F3.7 acceptance.

## Open contract condition

The existing Dioxus Case wrapper and `captured_case_document` route are physical-instance scoped and are not a valid substitute for Layout's canonical selected-board scene. Implementation starts only after F7.1/F7.3 identify the approved same-crate private input seam and pick mapping. If that seam cannot supply canonical Layout board assembly without API widening or duplicated renderer behavior, send the precise gap to architecture review and keep this child blocked.
