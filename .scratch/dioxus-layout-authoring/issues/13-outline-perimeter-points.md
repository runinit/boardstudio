# F3.4c: Edit an outline perimeter

**Parent:** F3.4 — Outline editing and refinement UI. Parent criteria, graph edges, history and public acceptance remain unchanged.

**What to build:** Let a designer open the active Board outline perimeter editor for a supported polygon contour, select a point, inspect and change its X/Y coordinates, insert a point after it, and remove it when at least three points remain. Editing Generated creates and activates a fixed copy on the first accepted change while preserving Generated; later edits update the active fixed copy. Done returns to the Board outline Inspector. Each accepted change participates in Undo/Redo and local project saving.

**Blocked by:** F3.4b — Active Board outline Inspector settings (complete in the candidate source).

**Status:** implementing

- [ ] Offer Edit perimeter points only when the current accepted outline has an editable polygon contour. Use the first accepted source contour for Generated and the first Polygon feature of the active fixed version. Keep unsupported contour shapes out of this child without a dead action.
- [ ] Show the selected point number, X/Y millimetre controls, a numbered point list, Insert after, Remove point and Done. Selection follows list activation and accepted edits. Removal is disabled when three points remain.
- [ ] Coordinate fields are finite-number drafts. Enter or blur commits one change; Escape restores the latest accepted coordinate. A result replaces the accepted values only after exact operation outcome and durable revision confirmation.
- [ ] The first Generated change uses the existing CopyOutline edit path to create an active fixed copy containing the changed contour. Preserve the unmodified Generated source. Editing an active saved version updates only its matching polygon feature through the existing SetOutline path.
- [ ] Admit every edit against the current Layout workspace, full Scope, accepted document/session/token/revision, generation, board and active version, saved/readiness state, and absence of preview/gesture/pending work. Reject stale actions and report exact failure states.
- [ ] Add mounted production-owner tests for Generated first edit/copy, fixed-version point update, insert/remove boundaries, coordinate commit/cancel, stale source rejection, and accepted/durable result verification. Preserve Core history and existing document serialization.
- [ ] Retain public paired point-edit/Undo-Redo/save-reopen and contextual Inspector checks as F3.4/F3.7 gates; this child does not close either parent or the 62-parent graph.
- [ ] Preserve RF-001, RF-006 and RF-009 evidence; record no new refactoring issue unless this child demonstrates a separate durable architecture problem.

**Spec:** [F3.4c perimeter point editor](../specs/F34c-outline-perimeter-points.md).

**Capability boundary:** Core already supports active fixed-version feature updates through SetOutline and a first fixed copy with a selected contour edit through CopyOutline. This child composes those existing operations through the private Layout owner; it adds no public type, schema, renderer or geometry algorithm.

**Out of scope:** Pointer-drag handles and live preview, canvas point overlay and snap guides, keyboard arrow/Delete editing, rectangle parameter editing, PartEnvelope/connection editing, manual additions/cutouts, bridge editing, gap/refinement tools, parent closure, or changes to public API/schema visibility.
