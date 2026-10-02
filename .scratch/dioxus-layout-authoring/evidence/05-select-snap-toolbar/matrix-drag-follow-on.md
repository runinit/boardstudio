# Proposed distinct F3.3 child: matrix-scoped drag and local snapping

This follow-on is deliberately not part of F3.3a. It is a candidate bounded F3.3 child for issue-graph review; it does not alter the canonical F3.3 criteria or joins.

## Exact behavior gap

- React `app/src/ui/createCanvasInteractions.ts:148–250` classifies the drag as matrix, row, column, a matrix key/member, or standalone part. Matrix scope changes `Matrix.origin`; row and column scope change `rowOffsets`/`columnOffsets`; a single matrix cell/member updates the cell or assembly local offset. It transforms row/column/key deltas with `localMatrixDelta` and snaps in the accepted `matrix.pitch` basis. Alt bypasses snapping. Matrix/cell paths use grid snapping rather than the standalone geometry/gap snap branch.
- Dioxus `web/src/presentation.rs` captures selected world `Position`s and sends `Event::GestureBegin`. `application/src/interactions.rs::normalize_drag` snaps world positions, and `Session` commits `EditOperation::MoveParts`. Core `MoveParts` routes linked primary keys through `core/src/matrix/layout.rs::move_keys`, but it does not provide the complete React matrix/row/column/cell/assembly metadata policy for independent layouts; unhandled parts become absolute pose overrides. The effect is different saved matrix data and different local-axis snap for rotated, mirrored or splayed matrix content.

## Candidate child boundary

Implement only Layout direct pointer manipulation for existing matrix contexts and members: matrix origin; row/column local offsets; primary key/cell offsets; and existing matrix assembly local offsets. Translate previews and one completed user gesture through the current accepted Runtime/Session/Core route, using existing `SetMatrix` or existing operation semantics and preserving a single transaction/Undo. Reuse the existing source local transform/snap rule and existing scene/read-model geometry; do not add a domain algorithm, duplicate mutable document, public API or Session event fields. Standalone free-part MoveParts/geometry/gap behavior stays with F3.3a.

Keep pointer capture, final up sample, panel-reflow click guard, Shift/Ctrl/Cmd selection, Alt bypass, Escape/cancel, board/project change and one-step Undo as required parent acceptance. Do not fold Transform menu commands, Align, constraints, or the separate drag-threshold correction into this child.

## Capability and acceptance evidence required before publication

Start can use source-backed F3.1 scope/membership plus an actual accepted matrix fixture; no whole-F3.1 acceptance barrier is implied. F3.2 remains the canonical acceptance join for the F3.3 parent. Before issue publication, prove the existing private/current adapters can issue preview and commit `SetMatrix` without widening Runtime/Session/Core APIs, and pin real rotated, mirrored, splayed, linked and independent matrix fixtures.

Paired public journeys should separately drag matrix, row, column, primary key, attached assembly and standalone part, then compare accepted matrix metadata, part/member poses, local snap results, cancel, Undo/Redo and save/reopen against React. Include multi-selection/linked member behavior and verify no stale context can publish after board/project/workspace change. Screen evidence and operation/history records are required; a source-only mapper or one green standalone drag cannot close this child.

## RF relation

This is a concrete F3 workflow manifestation of existing RF-005 (“Geometric edit planning lives in frontend helper policy”), not a new RF ID. Preserve the source evidence and impact in the post-port ledger; defer deciding whether interaction planning belongs in a private application service until parity and measurements justify structural change.
