# F3.4e: Manual outline geometry and linked connections

**Parent:** F3.4 — Outline editing and refinement UI. Parent criteria, graph edges, history and public acceptance remain unchanged.

**What to build:** Port the mounted React manual geometry workflow for Board outlines: addition/cutout drafts, Connect points drafts, saved authored-geometry selection and removal, rectangle dimensions, and selected connection width/point editing and removal. Generated source creates and activates a fixed copy for authored polygon edits; automatic connections continue through the generated `PartEnvelope`. An active fixed version edits only its own stored geometry through existing `ReplaceDocument` operations.

**Reference:** Pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/OutlineInspector.tsx`, and `app/src/ui/useOutlineEditor.tsx`; Core `CopyOutline.feature`, `ReplaceDocument`, Layout outline snapping and current `OutlineInspectorProjection` owner.

**Status:** mounted source packet implemented. Coordinator owns the combined source check and changed public journey; no browser acceptance is claimed in this source receipt.

**Spec:** [F3.4e manual polygon drafts](../specs/F34e-outline-manual-polygon-drafts.md).

**Focused source receipt:** [F3.4e source mapping](../evidence/f34e-manual-polygon-drafts-source-20261003.md).

- [ ] Draw addition and Draw cutout enter an owned draft without mutating accepted data; point clicks use the active Layout grid, geometry snap and Alt bypass.
- [ ] Show draft geometry and point count on the canvas/Inspector. Undo point removes only the last draft point. Cancel and Escape discard only this draft. Finish requires at least three finite non-collinear points and emits one accepted operation.
- [ ] Generated drafts use existing `CopyOutline.feature`, preserving Generated and activating a fixed copy. Fixed drafts append to only the currently active version through existing `ReplaceDocument` history/durability; the operation is admitted against current scope, token, revision, board and active tree context.
- [ ] Connect points drafts require Generated plus an automatic envelope, use existing grid/geometry snapping, bind endpoints to eligible component origins within the React 10 mm rule, and submit one `SetOutline` edit against the accepted generated envelope.
- [ ] Saved authored geometry is selectable. Polygon point editing routes through the existing perimeter editor; rectangles expose width, height, and corner radius. Fixed-version removal protects the first authored perimeter and removes later features through `ReplaceDocument`.
- [ ] Selected rectangle center coordinates are editable in board space; an existing part anchor is preserved by converting each changed coordinate back into that part's local frame.
- [ ] Saved connections expose width and world-coordinate point editing while preserving valid part-local attachments; removal edits only the existing connection list on its accepted `PartEnvelope`.
- [ ] Gap repair rows retain their protected-gap control and provide a source-contour focus action that fits the accepted gap into the Layout camera without creating a document edit.
- [ ] Show pending/saved/failure feedback only from the existing exact accepted-operation settlement path. No parallel writable geometry state or new public contract/schema is introduced.
- [ ] Coordinator verifies the changed draft/connection/editor journey on the pinned fixture. Broader F3.4/F3.7 acceptance and unchanged parent joins remain open.
- [ ] Preserve RF-001, RF-006 and RF-009. This source comparison adds no new refactoring finding.

**Out of scope:** advanced refinement editing, complete point-guide/attachment-picker parity, every capture/lifecycle race, parent closure, CAD/geometry changes, and public API/schema visibility. Core validation remains authoritative; invalid connection geometry is surfaced by the existing operation outcome rather than repaired with a second bridge algorithm.
