# F3.4e — Manual outline geometry and linked connections

**Parent:** F3.4 — Outline editing and refinement UI. This child implements the mounted manual geometry and linked connection workflow. It does not change the canonical parent acceptance criteria or dependency edges.

**Reference:** Pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, `OutlineInspector.tsx`, `useOutlineEditor.tsx`, `OutlineDraftPreview`, `outlineSnapping.ts`; existing Rust Layout scope/selection owner, canvas interaction arbiter, Core `CopyOutline.feature`, `ReplaceDocument`, preview/save lifecycle and history.

## User journey

With a Board outline selected, the designer starts Draw addition, Draw cutout, or Connect points. The canvas accepts successive primary clicks and displays the draft path and points. Each point uses the existing Layout grid and outline geometry snap policy; Alt bypasses both snap modes for that click. The Inspector reports the point count and offers Undo point, Cancel drawing and Finish drawing. Enter finishes and Escape cancels while the draft canvas has keyboard focus. Connection endpoints attach to an eligible included component within the pinned React 10 mm distance rule; other control points remain fixed in board coordinates.

Polygon finish requires at least three finite points enclosing a non-zero area; connections require at least two finite points. Each finish submits one existing accepted document operation. With Generated active, `CopyOutline.feature` creates and activates a fixed copy containing the authored polygon while preserving the Generated source. With a fixed version active, a polygon is appended only to that version through the existing `ReplaceDocument` edit. Connections require Generated and an existing automatic envelope and update that accepted envelope through `SetOutline`. Draft state is presentation-only.

The saved geometry list selects authored shapes. Polygon selection opens the existing perimeter point editor; rectangle selection exposes board-space center coordinates, width, height, and corner radius. Center edits preserve a part anchor by converting the edited world coordinate back to local space. Fixed versions can remove later authored features, while the first authored perimeter is protected. Saved connections expose width and point-coordinate editing; attached points are transformed through their current part pose and edited back in part-local coordinates. Generated connection points expose the pinned attachment picker, preserving the visible point position when attached or detached. Insert is disabled after the final endpoint and inserts a fixed midpoint; removal preserves the two-point minimum. Removing a connection changes only the existing envelope connection list. Fixed-version feature changes use `ReplaceDocument` and are scoped to the active version.

Each generated gap row also offers a focus action. It revalidates the exact outline owner and accepted gap before fitting the gap bounds into the Layout camera; this is view state and does not enter document history. Keeping or removing protected gaps remains an accepted settings edit.

## Ownership and failure behavior

`outline_lifecycle.rs` owns tool selection, draft points, feature construction, action admission and outcome feedback. The existing `OutlineInspectorProjection` carries the accepted scope, document token/revision, generation, board and selected tree owner. The canvas composition mounts a private overlay beside the existing perimeter overlay and uses the same `CanvasInteractionArbiter`, SVG coordinate conversion and outline snapping helper. Core/session remain the only accepted-document/history/persistence authorities.

The action is dropped if its scope, accepted token/revision, selected Board/Outline context, readiness or active outline owner no longer matches. Draft points clear on owner/version/revision replacement, explicit cancel, Escape or submission. A rejected/persistence-failed operation leaves accepted geometry authoritative and uses the existing Inspector feedback route.

## Acceptance

- A paired public route starts the changed manual draft/editor workflow, adds visible snapped points, removes a draft point, cancels without revision change, and finishes valid polygon and connection edits.
- Generated remains available and unchanged after its first authored polygon; the new feature exists on the activated fixed copy. A fixed version appends without mutating another version.
- Accepted operation, Undo/Redo and save/reopen reflect exactly one finished polygon. Invalid and cancelled drafts do not enter history.
- The existing perimeter, version selection, settings, gap repair, and outline finding routes remain intact. Full F3.4/F3.7 and 62-parent acceptance remain separate.

## Explicit limits

This child does not implement polygon/rectangle attachment controls because pinned React exposes attachment only for generated connections. It also does not implement connection point canvas dragging, advanced refinements, full point-guide presentation, or every draft focus/unmount race. Core validation remains the authority for malformed geometry. Existing F3.4b/c/d evidence remains reusable for unchanged behavior, but this child does not close F3.4.

Preserve RF-001 (one Editor operation authority), RF-006 (accepted Board/scope and geometry), and RF-009 (exact fixture/source/provenance evidence). No new refactoring takeaway was identified in this source comparison.
