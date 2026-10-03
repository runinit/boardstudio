# Case contextual gasket support unlink

**Parent workflows:** F7.2 Case contextual workspace, F7.4 mechanical settings, and F7.5 Case viewer direct manipulation.

## Problem

The React Case viewer exposes `Unlink selected support` while `Edit gaskets` is active and a gasket support is selected. Dioxus now has current support selection and scoped sizing, but it has no corresponding viewer action.

## User outcome

In Case, a designer enters `Edit gaskets`, selects a current generated gasket support, and unlinks its mirrored support pair from the viewer toolbar. The selected and paired anchors retain their IDs, positions, sizes, outline ownership, and other metadata; only their existing `unlinked` flags change. The existing MechanicalSettings controller persists the change in one document-history operation.

## Behavior pinned from React

- `Edit gaskets` is in the 3D assembly toolbar. `Unlink selected support` appears there only in edit mode with an active support.
- The operation marks the active support and its mirrored `pairId` partner unlinked. The pair continues to represent upper and lower pad geometry for each support; unlinking concerns the reflected support pair, not separating top/bottom pads.
- No reset-position action is coupled to unlinking. Gasket position dragging remains in the separate F7.5 gesture slice.
- The reference wording is `Edit gaskets` and `Unlink selected support`. Its Inspector hint says `Matching upper and lower pads. Drag this gasket directly to another side in Edit gaskets.`

## Implementation decisions

- Reuse the Case viewer toolbar, current selected generated support projection, existing MechanicalSettings identity/request/controller and canonical or physical-instance commit policy.
- Preserve all anchor fields and set `unlinked=true` for the selected anchor and its currently linked pair. Do not change the gasket engine, saved format, provider, or public API.
- Keep the action disabled for previous geometry, non-editable owners, an in-flight settings change, or an already-unlinked support. Stale selection/scope is rejected by the current controller guards.
- Keep upper/lower pad geometry and its source wording distinct from the mirrored support pair linkage state.
- Issue07 continues to own handle dragging, mount placement, gesture lifecycle and comprehensive gesture acceptance; this ticket only extracts its standalone unlink control and commit behavior.

## Acceptance

- A current selected Gasket row exposes `Edit gaskets` and then `Unlink selected support` in the viewer toolbar, matching React placement and labels.
- Unlink preserves each selected/pair anchor's identity, location, dimensions and outline metadata while setting the existing linkage flag. The new resolved Case geometry reports the support as unlinked without separating its upper/lower pads.
- The change goes through the scoped MechanicalSettings controller, active owner checks, accepted document history, and existing instance/canonical save mapping. Undo restores the linked pair; Redo and reopen retain the unlink state.
- Stale/previous support rows, wrong workspace/Scope, changed accepted token, and a pending settings operation cannot apply the action.
- No geometry algorithm, reset-position, drag gesture, re-link control, provider rewrite, public API/schema change, or new draft/history owner is introduced.
- Use the existing affected check once and verify the changed same-archive unlink journey on the integrated candidate. Do not add a routine UI test matrix.
- Preserve RF-001/RF-006; no separate refactoring finding was observed in this bounded source pass.

## Out of scope

Gasket dragging, reposition/reset, remirroring an already-unlinked support, mount edits, resolver behavior, CAD generation semantics and full F7.5 acceptance.
