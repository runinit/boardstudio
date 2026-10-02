# Mirrored-pair canvas ownership join

This is an implementation seam note for the F3.2 linked-pair child. It does not close F3.2, the linked-layout Inspector child, or public/browser acceptance.

## One active placement owner

The Editor has one canvas placement interaction at a time. Mirrored-pair ownership begins as soon as the setup form opens, remains held while catalogue definitions are preparing, while the ghost is active, and while the accepted edit is pending terminal save settlement. `MirroredPairMount::owns_canvas` covers those states. The Layout Objects menu, canvas selection/drag handlers, and competing matrix setup entry must not start another canvas interaction while that field is true.

The separate PartPlacement mount must expose a corresponding private ownership predicate covering its asynchronous preparation and active projection. Editor composition must mutually gate both owners: the part-placement canvas route is enabled only when it owns the canvas and the mirrored-pair owner does not; the pair route is enabled only when it owns the canvas and the part-placement owner does not. Opening either workflow while the other reports ownership is rejected. This is a narrow Editor admission rule, not a shared gesture abstraction.

A previously registered placement edit still settles its captured `Scope` and exact `OperationId`/`OutcomeSlot` after the UI owner becomes hidden. The resulting pair can be selected/revealed only when the accepted result is Ready/Saved, has a token different from the captured base token, has a revision greater than the base revision, and the current accepted token/revision exactly match that result. Scope/editor-generation/board checks must still match. Stale completions retire their pending slot without redirecting selection or opening the competing workflow.

## Canvas mapping

Pointer placement takes the canvas's existing `coordinates()` world point. Preview and commit use the same point, and Escape returns to the form without submitting an edit. Arrow placement uses the current Layout snap-fraction preference and the prepared matrix pitch; the pointer path follows the React path and remains at the raw world point. The gap field is the user-entered distance between key edges and is not overloaded as a placement snap setting.

## Source anchors

- React `app/src/ui/Workbench.tsx`: `createWorkbenchPlacementActions` call and `placeMatrixAt` click route; SVG key handling for Enter/Arrow/Escape; `matrixGhost` canvas preview; menu/guide placement suppression.
- React `app/src/ui/MirroredPairSetup.tsx`: defaults and `pairAt` mirror/link geometry.
- Dioxus `web/src/presentation.rs`: Editor owner, `coordinates`, Layout SVG pointer/keyboard handlers, accepted-result selection callback.
- Dioxus `web/src/presentation/objects/mirrored_pair_controller.rs`: async preparation, exact captured owner, registered outcome slot and terminal accepted-save check.
- Dioxus `web/src/presentation/objects/mirrored_pair.rs`: typed form/move/result projections.
- Existing Core operation: `core/src/model.rs::EditOperation::CreateMirroredPair`; no schema or operation widening.

## Verification boundary

The pure geometry and saved-result predicate have native unit coverage. The mounted source still needs independent review, the mutually exclusive PartPlacement join, packaged/browser paired creation/history/reopen/cancel evidence, and the existing F3.2 parent joins. Do not report full linked-layout parity from this seam alone.
