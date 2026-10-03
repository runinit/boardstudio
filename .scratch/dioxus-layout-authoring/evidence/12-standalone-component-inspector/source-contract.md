# F3.5a standalone component Inspector — source contract

**Audit base:** `255ccc151fb2133eb966467e23000046dd690331` (isolated author branch; initially clean).
**Pinned React source:** `5a472a9426e6e38993361da402cd4ec730feb369`.
**Reference fixture:** `.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

## Observed React behavior

With the fixture loaded at `http://127.0.0.1:5173/`, selecting standalone `left-U1` in Layout changes the tool pill to `Select: Part` and opens Inspector tabs `Properties` and `Relations`. Properties show the component reference, definition name/kind, Component layout (`Board / ungrouped` and existing layout options), Position X/Y in millimetres, Board outline contribution and Layout constraint. The panel provides `Edit electrical connections`. Relations shows `Relationships`, a current relationship summary/empty state, `Edit placement relationship`, and the matrix-geometry note. These are contextual to a selected component; the board-level panel instead shows Board name.

The React source path for this component branch is `app/src/ui/Workbench.tsx`: selection is resolved to `activePart`/`activeDefinition`; component Properties render at the selected component branch; `assignPartLayout` updates `document.layouts[].partIds` through one `replace-document`; position emits `move-parts`; part outline controls emit a selected-Part document replacement; the constraint editor emits existing `set-constraint` / `remove-constraint`; the PCB action calls `changeMode('PCB')`. `ConstraintEditor.tsx` restricts source selection to visible parts other than the selected target and checks selected-board membership before save. `OutlineInspector.tsx` provides Include, board margin/explicit margin and body-overhang contribution controls. The Relations tab is a presentation summary and routes back to Properties; it does not author or solve a new relationship family.

## Existing Dioxus and domain seams

- `web/src/presentation.rs` holds the current scoped tree selection, checks it against the current read model, computes `show_position_inspector`, and mounts the Layout inspector composition. Component context is available via existing `TreeContext::Component` and `selection::{context_is_current,resolve_context}`. The current `Inspector` is generic X/Y position UI and reads the selected IDs directly from the Runtime model.
- `web/src/presentation/layout_workspace.rs` is the existing Layout inspector composition. The current contextual heading is a projection, not the component Properties/Relations form. The separate Parts author has reserved follow-up wiring changes to `layout_workspace.rs` and `presentation.rs`; this author must wait for that frozen delta before editing either shared file.
- Accepted component data already includes `Part::{id,definition_id,reference,pose,locked,outline}` and the selected board/layout/document projection. `PartOutline` already stores `excluded`, optional `margin`, and `allow_body_overhang`.
- Core already defines `MoveParts`, `ReplaceDocument`, `SetConstraint`, and `RemoveConstraint`. Core apply handles layout/constraint edits, and replacement re-resolves the accepted document. No domain/schema/API widening is indicated.
- Existing accepted owner and selection route must remain authoritative; draft state may live in the private Inspector leaf, but mutation callbacks must verify current document/scope/owner and selected component before submitting.

## Start condition and ownership

The capability-level start condition is met in the inspected source: the current Layout route carries a selected component identity under a scoped owner; the read model exposes its accepted part and board/document context; existing edit and workspace navigation ports cover the required behavior. This does not claim full F3.1, F3.5 or F3.7 acceptance. Canonical dependency remains F3.1 only, unchanged in the 62-parent graph. F3.2d matrix/cell structural editing, F3.3 transform/constraint tool ownership, F3.4 board-outline geometry/version editing, F6C.3 keycap sizing/reflow, and full F3.5/F3.7 joins remain separate.

Likely private leaf: `web/src/presentation/objects/layout_component_inspector.rs`. Mount/callback wiring is deliberately pending the Parts author’s exact shared-file delta and a frozen boundary review. No app source was changed during this planning packet.
