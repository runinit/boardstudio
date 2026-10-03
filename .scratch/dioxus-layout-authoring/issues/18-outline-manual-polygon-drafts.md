# F3.4e: Draw outline additions and cutouts

**Parent:** F3.4 — Outline editing and refinement UI. Parent criteria, graph edges, history and public acceptance remain unchanged.

**What to build:** Port the mounted React Draw addition and Draw cutout point-draft path for Board outlines. The designer adds snapped canvas points, removes the last draft point, finishes a valid polygon, or cancels without changing the accepted document. Generated source creates and activates a fixed copy containing the new feature; an active fixed version appends the feature to that version through the existing document-edit operation.

**Reference:** Pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/OutlineInspector.tsx`, and `app/src/ui/useOutlineEditor.tsx`; Core `CopyOutline.feature`, `ReplaceDocument`, Layout outline snapping and current `OutlineInspectorProjection` owner.

**Status:** mounted source candidate implemented; coordinator combined check and one public draft/commit/cancel journey remain pending.

**Spec:** [F3.4e manual polygon drafts](../specs/F34e-outline-manual-polygon-drafts.md).

**Focused source receipt:** [F3.4e source mapping](../evidence/f34e-manual-polygon-drafts-source-20261003.md).

- [ ] Draw addition and Draw cutout enter an owned draft without mutating accepted data; point clicks use the active Layout grid, geometry snap and Alt bypass.
- [ ] Show draft geometry and point count on the canvas/Inspector. Undo point removes only the last draft point. Cancel and Escape discard only this draft. Finish requires at least three finite non-collinear points and emits one accepted operation.
- [ ] Generated drafts use existing `CopyOutline.feature`, preserving Generated and activating a fixed copy. Fixed drafts append to only the currently active version through existing `ReplaceDocument` history/durability; the operation is admitted against current scope, token, revision, board and active tree context.
- [ ] Show pending/saved/failure feedback only from the existing exact accepted-operation settlement path. No parallel writable geometry state or new public contract/schema is introduced.
- [ ] Verify one paired public addition/cutout draft and cancel/commit history journey on the pinned fixture; broader F3.4/F3.7 acceptance and unchanged parent joins remain open.
- [ ] Preserve RF-001, RF-006 and RF-009. This source comparison adds no new refactoring finding.

**Out of scope:** Connect points/bridge authoring, feature list/edit routing, rectangle controls, refinement editing, full keyboard parity outside draft focus, broader capture/lifecycle matrix, parent closure, CAD/geometry changes, or public API/schema visibility.
