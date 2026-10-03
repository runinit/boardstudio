# Case contextual gasket support sizing

**Parent workflows:** F7.2 Case contextual workspace and F7.4 mechanical settings.

## Problem

The Case Objects tree exposes current resolved gasket supports as selectable rows, but selecting one leaves the full mechanical settings Inspector visible. The React Case Inspector instead opens that support's cut-length and pad-width controls, so designers cannot resize a gasket from its selected context.

## User outcome

Selecting a current gasket support opens a scoped `Gasket N` Inspector with `Cut length` and `Pad width`. Editing either value saves through the existing MechanicalSettings request/controller path. A linked upper/lower pair changes together; an unlinked support changes alone. `Assembly settings` returns to the full mechanical Inspector, while `All gasket settings` opens the Gaskets layout settings directly.

## Implementation decisions

- Reuse the accepted exact-scope mechanical scene's support projection, existing Case selection, MechanicalSettings request identity, controller admission, canonical/physical commit mapping, operation feedback and history.
- Route `All gasket settings` through the existing selected-layer callback with the current `gaskets` layer ID; do not add another navigation or document-state owner.
- Add a private support-resize request patch that carries only source-projected support anchors. Update the existing saved gasket layout, preserving unrelated anchors and support metadata.
- Apply React's minimums (5 mm cut length and 0.5 mm pad width) and per-field draft, blur/Enter commit, Escape rollback and saved/failure feedback.
- Treat stale previous geometry as read-only and keep display preferences separate from the document edit.
- Leave gasket layout, material, closure-hardware and drag controls to existing Issue05/F7.4c and Issue07/F7.5 ownership.

## Acceptance

- A selected current lower or upper support opens the same contextual title and numeric fields as React.
- Resizing a linked support updates both current pair anchors; an unlinked support updates only itself. The controller merges into the latest accepted mechanical configuration through the existing operation path.
- Invalid input and stale scope cannot commit. Saved and failed outcomes remain attached to the submitted field. Undo/Redo and save/reopen retain the accepted layout.
- Assembly settings and selection changes restore the global Inspector. Display actions remain view-only.
- `All gasket settings` from a selected support opens the current Gaskets settings section without changing the accepted document, support data, or history.
- Preserve the existing Case parent joins and record no new refactoring finding unless source evidence supports one.

## Out of scope

Gasket layout/material/closure settings, geometry or resolver changes, viewer dragging, public API/schema changes and parent acceptance.
