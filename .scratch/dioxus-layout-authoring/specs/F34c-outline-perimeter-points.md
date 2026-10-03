# F3.4c — Active Board outline perimeter points

**Parent:** F3.4 — Outline editing and refinement UI. F3.4b supplies the active Board outline Inspector; this child does not change parent criteria or dependency edges.

**Reference:** Pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, `OutlineInspector.tsx`, `OutlineFeatureEditor.tsx`, `OutlineControlOverlay.tsx`, `outlineEditing.ts`, and `useOutlineEditor.tsx`; existing Rust `CopyOutline.edit`, `SetOutline`, accepted board outline scene and history.

## Problem

The Board outline Inspector identifies the active version but provides no Dioxus route to change a perimeter. React exposes a selected point with coordinate inputs, a numbered list, insert/remove actions and a return action. Core already provides the two edit routes required for the currently supported polygon path, but the Dioxus Layout owner does not connect them to the Inspector.

## User-visible behavior

From a current accepted Layout Board outline, the designer opens Edit perimeter points. The Inspector shows Point N of M, finite X/Y millimetre fields, the point list, Insert after, Remove point and Done. Selecting a point in the list changes the fields; accepted coordinate or point-list edits update the displayed geometry and selected index. Remove is disabled at three points. Done returns to the normal Board outline Inspector.

Support is intentionally bounded to the first polygon contour exposed by the current outline. Generated reads the first accepted `BoardOutlineScene.sourceContours` entry. A fixed outline reads the first polygon feature in the accepted active version. The action is absent when no such valid contour exists. Rectangles and PartEnvelope connections are not converted into polygons.

Generated remains a live source. Its first point edit creates and activates a fixed copy by submitting the existing `CopyOutline` operation with `OutlineContourEdit` for the selected contour. The source Generated contour stays unchanged. For an active fixed version, the editor changes only the matching polygon feature with the existing `SetOutline` operation. Both routes use ordinary accepted Session edits, Core validation/history, and local save behavior.

Coordinate fields keep a local draft keyed to the accepted coordinate. Enter and blur commit only finite values; Escape restores the most recent accepted baseline. The editor does not report Saved optimistically. Pending state disables further edits until the exact operation outcome, accepted geometry and durable revision agree. A stale action, replaced scope/version, in-progress gesture/preview, or failed durability does not submit or claim success.

## Ownership and capability boundary

The private outline lifecycle owner remains responsible for current scope/token/revision/generation checks, current Outline tree context, edit admission, operation observation, submission and durable feedback. The Inspector remains a read-only projection plus typed edit intents. It does not hold a second outline/document model.

The first Generated mutation maps to existing `CopyOutline.edit`; fixed polygon updates map to existing `SetOutline`, which updates the feature by its accepted ID inside version geometry. Existing Core operations and project serialization remain authoritative. No public API, schema, generated contract or renderer changes are required.

## Testing decisions

- Mounted production-owner tests exercise visible controls through the real hook and assert the submitted operation, target version/contour, exact changed points, and current accepted owner.
- Exercise rejected stale context and unchanged/invalid values as no-submit paths. Verify coordinate Escape restores accepted values, Enter/blur submit once, and removal cannot produce fewer than three points.
- Verify a Generated edit creates/activates a fixed copy while preserving the source contour, and a fixed edit targets only its active feature.
- Existing Core outline/version/history tests remain the domain oracle. A public paired journey must still exercise point mutation, Undo/Redo, save/reopen, contextual panes, and comparison with pinned React; source tests do not accept F3.4/F3.7.

## Out of scope

Canvas point handles, pointer drag and live preview, snap guide integration and keyboard arrow/Delete controls; rectangle parameters; PartEnvelope/connection vertices; manual additions/cutouts, bridge editing, protected gaps beyond F3.4b, refinements, case physical-instance context, parent closure, new geometry algorithms, or public API/schema visibility.

## Refactoring record

Preserve RF-001 (single Editor operation ownership), RF-006 (canonical accepted board/scope and geometry), and RF-009 (exact source/build/fixture evidence). This bounded slice exposes no separate refactoring finding.
