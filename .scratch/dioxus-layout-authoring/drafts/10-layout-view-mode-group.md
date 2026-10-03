# Draft F3.6 view-mode child — Match the Layout view group

**Parent:** F3.6 — Camera, fit and Layout 2D/3D assembly. This is one local operational child of the existing canonical parent; it does not create or change a task-graph row.

**Reference:** React source `5a472a9426e6e38993361da402cd4ec730feb369`. **Dioxus source base:** `8cfd6bb79e9e10b788e007fd428145b1e37095d1`, including the integrated command-pill and close-control joins. The paired command-pill journey remains evidence only; it does not claim full F3.3 parity. Paired menu journey and source seam inventory are recorded separately under evidence 09 and 10.

**Current paired comparison:** candidate build/source `frontend-authoring-layers-integrated-20261002` / `7d09d0a60fbb2cc12e259541614b9483ee618d29`; the current public view-group placement mismatch is recorded in the F7.3b layer-control contract refinement and paired evidence receipt.

## What to build

Add the separate Design view group shown by React, in this order: **2D**, **3D assembly**, and **Footprints**. At desktop and compact widths, place it directly below and left-aligned with the Layout command pill, as in the pinned React workbench; it is not part of the command-menu owner. The Footprints choice is visible in 2D only; it reuses the accepted F3a visibility state and layer projection. Keep its value when entering 3D and restore it when returning to 2D.

In 2D, keep the existing Layout canvas and its session camera. In 3D assembly, render the selected board from the canonical Layout document through the single F7-owned common viewer/private adapter. The active mode remains Layout/Design: this is a view change, not navigation into Case. Do not pass the physical-instance Case document, call the Case-only capture projection, clone renderer code, introduce another renderer, or widen a public API. The exact canonical-board input and picking mapping use the bounded same-crate capability in existing child 11 (`F7 canonical Layout source and picks`), whose F7.1 private contract and start review are clear. This source/pick capability provides implementation readiness for this child; it does not complete the existing F7.3b issue 03 viewer consumer or its remaining acceptance.

On entry to 3D, follow React's view-change path: cancel active placement and gesture/draft work, close command menus, clear transform-tool state, then change the view. Returning to 2D restores the existing Layout camera and selection without creating an edit, revision, or Undo entry. Changes in selected board, project/session owner, or workspace invalidate stale viewer output and release the prior viewer owner. View state remains presentation/session state and is not saved in the project document.

The 3D route must use the shared viewer's supported camera, fit, preset, orbit/zoom, picking, status and lifecycle behavior. Layout supplies canonical board identity, scene input and the mapping from a picked board reference to current Layout selection. It does not add Case-only bodies, handles, mechanical drafts, or physical-instance scope.

## Dependencies and ownership

- **Start after:** F3.1 selection and board scope; the F7.1 common-viewer contract and renderer feasibility decision; and the callable private source/pick capability in existing child 11. F3.1 and the F7.1 source/renderer decisions are established. The private child 11 source/pick implementation exists, but this F3.6 source/model readiness gate remains open until an accepted Layout component model is loaded and decoded through the production Runtime route. The child 11 unit smoke covers accepted BoardReference asset selection and verified-byte handoff to ModelDeliveryAdapter with stub bytes/decoder, not production Runtime model consumption. Coordinator-authorized implementation proceeded in parallel; that does not satisfy this readiness gate. The mounted current-pick journey remains an acceptance check below, not an additional pre-start gate.
- **Acceptance join:** existing F7.3b issue 03 shared-viewer consumer checks and the existing F3.6 camera/view parent acceptance. Child 11 does not close issue 03: its BoardReference pose/elevation, generated-keycap suppression, conditional same-document Case overlays, lifecycle and remaining consumer checks stay open.
- **Parent:** existing F3.6 in the canonical task graph. Keep F3.6 and F3.7 open until their complete acceptance passes. Do not duplicate or edit any of the 62 canonical parent records for this local operational child.
- **Capability reuse:** existing F3a `LayerVisibility.footprints`, existing Layout session camera in `layout_camera`, existing Layout canvas/cancellation owner, and the one F7 common viewer. The React `AssemblyViewer` is the behavior oracle, not code to clone.
- **Boundary:** Layout-view controls and canonical consumer wiring only. F3.6 still owns the broader 2D camera/fit behaviors and the completed Layout view lifecycle. F7 owns renderer behavior and its adapter. Any real ABI gap returns to a separately reviewed contract; no new API is presumed.

## Acceptance criteria

- [ ] On one identical saved archive hash, compare React and Dioxus Layout at 1280×577 and 375×667, in Light and Dark. Show the separate 2D / 3D assembly / Footprints group and corresponding scenes. In 2D, show the Select / Transform / Align / Snap command pill; in 3D, replace it with React's Layout / PCB assembly context label. Hide the 2D canvas and WorkbenchLayers overlay in 3D; restore them when returning to 2D. Record archive, source, build and browser identities.
- [ ] At both required viewports, place the view group directly beneath and left-aligned with the Layout command pill, matching the pinned React toolbar; do not anchor it to the opposite side of the canvas.
- [ ] The view group has React's order, labels, pressed states, placement, and visibility: Footprints appears only in 2D. Its existing F3a value survives 2D → 3D → 2D, and the same Footprints layer is visible/hidden on return. The command pill appears in 2D and is replaced by the Layout / PCB assembly context label in 3D; the 2D WorkbenchLayers overlay hides in 3D and returns in 2D.
- [ ] 2D → 3D uses the F7-approved canonical selected-board scene from the Layout document, without entering Case scope or consuming a physical-instance projection. A second-board fixture proves board isolation. Picking maps supported references to the matching Layout part/selection.
- [ ] 3D entry cancels the active placement/gesture/draft, closes any command menu, and clears Transform tool state with no partial document edit. Returning to 2D restores the prior Layout camera and current valid selection. View changes alone do not alter accepted values, document revision, or Undo/Redo history.
- [ ] Board change, project/session replacement, and leaving Layout invalidate prior async output, prevent stale picks, and dispose/unmount the viewer owner. Repeated switches remain clean.
- [ ] Verify the shared viewer's supported orbit/zoom, fit and preset controls, theme palette, resize/device-pixel-ratio, loading/error/context-loss and recovery behavior as supported by F7.3. No Layout-local clone of those controls or lifecycle.
- [ ] No duplicate Footprints state, renderer, camera store, saved view field, Case physical-instance routing, new public API, or F7-owned renderer behavior. Preserve F3a and F7 accepted contracts.
- [ ] Keep complete F3.6 camera/fit acceptance and F3.7 integrated Layout parity open; this child does not claim either parent complete.

## Out of scope

F3.6's complete 2D navigation and fit acceptance; F7 common renderer internals; physical Case assembly; Keymap/Keycaps/Parts viewer consumers; new scene schemas or public API; view-state persistence; any change to project document format; full Layout/F3.7 acceptance.

## Readiness and consumer boundary

The existing Dioxus Case wrapper and `captured_case_document` route remain physical-instance scoped and are not a valid substitute for Layout's canonical selected-board scene. Existing child 11 supplies the reviewed same-crate canonical source, common-viewer route and guarded pick mapping without API widening or duplicated renderer behavior. Its implementation receipt includes a focused adapter smoke through accepted BoardReference asset resolution and SHA-verified byte handoff to ModelDeliveryAdapter, but its bytes and decoder are stubs. The production Runtime model-consumption readiness gate therefore remains open. Implementation work was coordinator-authorized in parallel; it does not imply the gate has passed. The mounted shared-viewer pick remains part of this child's later acceptance. F7.3b issue 03 remains open for complete shared-viewer consumer acceptance, and this child must still pass its F3.6 parent join before either parent can close.
