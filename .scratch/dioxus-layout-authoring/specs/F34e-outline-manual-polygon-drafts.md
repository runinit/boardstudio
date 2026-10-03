# F3.4e — Manual outline polygon drafts

**Parent:** F3.4 — Outline editing and refinement UI. This child implements the Draw addition and Draw cutout polygon draft controls. It does not change the canonical parent acceptance criteria or dependency edges.

**Reference:** Pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, `OutlineInspector.tsx`, `useOutlineEditor.tsx`, `OutlineDraftPreview`, `outlineSnapping.ts`; existing Rust Layout scope/selection owner, canvas interaction arbiter, Core `CopyOutline.feature`, `ReplaceDocument`, preview/save lifecycle and history.

## User journey

With a Board outline selected, the designer starts Draw addition or Draw cutout. The canvas accepts successive primary clicks and displays the draft path and points. Each point uses the existing Layout grid and outline geometry snap policy; Alt bypasses both snap modes for that click. The Inspector reports the point count and offers Undo point, Cancel drawing and Finish drawing. Enter finishes and Escape cancels while the draft canvas has keyboard focus.

Finishing requires at least three finite points enclosing a non-zero area. It submits one existing accepted document operation. With Generated active, `CopyOutline.feature` creates and activates a fixed copy containing the authored polygon while preserving the Generated source. With a fixed version active, the feature is appended only to that version through the existing `ReplaceDocument` edit, preserving ordinary session history and durability. The draft is presentation state and never becomes a second document authority.

## Ownership and failure behavior

`outline_lifecycle.rs` owns tool selection, draft points, feature construction, action admission and outcome feedback. The existing `OutlineInspectorProjection` carries the accepted scope, document token/revision, generation, board and selected tree owner. The canvas composition mounts a private overlay beside the existing perimeter overlay and uses the same `CanvasInteractionArbiter`, SVG coordinate conversion and outline snapping helper. Core/session remain the only accepted-document/history/persistence authorities.

The action is dropped if its scope, accepted token/revision, selected Board/Outline context, readiness or active outline owner no longer matches. Draft points clear on owner/version/revision replacement, explicit cancel, Escape or submission. A rejected/persistence-failed operation leaves accepted geometry authoritative and uses the existing Inspector feedback route.

## Acceptance

- A paired public route starts each authored polygon tool, adds visible snapped points, removes a draft point, cancels without revision change, and finishes one valid addition/cutout.
- Generated remains available and unchanged after its first authored polygon; the new feature exists on the activated fixed copy. A fixed version appends without mutating another version.
- Accepted operation, Undo/Redo and save/reopen reflect exactly one finished polygon. Invalid and cancelled drafts do not enter history.
- The existing perimeter, version selection, settings, gap repair, and outline finding routes remain intact. Full F3.4/F3.7 and 62-parent acceptance remain separate.

## Explicit limits

This child does not implement Connect points/bridge creation, saved-geometry feature selection/removal/editor routing, rectangle dimensions, source-linked attachment editing, advanced refinements, complete point-guide presentation, or every draft focus/unmount race. Existing F3.4b/c/d evidence remains reusable for unchanged behavior, but this child does not close F3.4.

Preserve RF-001 (one Editor operation authority), RF-006 (accepted Board/scope and geometry), and RF-009 (exact fixture/source/provenance evidence). No new refactoring takeaway was identified in this source comparison.
