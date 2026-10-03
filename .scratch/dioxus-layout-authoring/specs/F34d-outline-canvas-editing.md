# F3.4d — Direct outline perimeter editing

**Parent:** F3.4 — Outline editing and refinement UI. F3.4c remains the accepted Inspector route for supported polygon points; this child adds canvas interaction without changing parent criteria or dependency edges.

**Reference:** Pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, `OutlineControlOverlay.tsx`, `outlineSnapping.ts`, `OutlineSnapGuides.tsx`, and `useOutlineEditor.tsx`; current Rust `CopyOutline.edit`, `SetOutline`, `SelectOutline`, Core preview/commit/history, Layout snap preferences, and canvas pointer arbiter.

## Problem

The Rust Layout perimeter editor can select and edit a point from the Inspector, but it has no canvas handles. The React workflow directly selects and drags supported perimeter points, displays a snapped live preview, and lets the focused point move by keyboard.

## User journey and behavior

From the current accepted Layout Board outline, opening Edit perimeter points shows the existing point list and an accessible handle at each point on the canvas. The current Inspector selection and canvas handle selection stay synchronized. The first accepted outline source contour and the first Polygon feature on the active fixed version use the existing F3.4c ownership and edit routes. Other contour types remain unsupported.

Pressing a handle captures that pointer and previews the moved point against the existing React snap policy: active grid, optional geometry alignment, edge/collinear/perpendicular guides, screen-distance tolerance and guide hysteresis. Alt temporarily disables grid and geometry snapping. The last pointer sample is included on release. Releasing a moved handle submits one ordinary Core commit using the same transaction as its previews, yielding one Undo entry. A no-movement release submits nothing. Escape, pointer cancellation, lost capture, workspace/board/version/document/session replacement or unmount clears preview and capture without a commit.

Focusing a handle selects its point. Arrow keys move it by the effective outline grid; Shift multiplies the step by ten. Delete and Backspace remove the selected point only when at least four remain, preserving the existing three-point minimum. Each nudge or removal is one Core commit and one Undo entry. The existing Inspector coordinate fields and list continue to work.

Generated remains a live source. Drag, nudge, or removal first previews the existing `CopyOutline` edit and then commits one fixed copy with the changed first contour; source geometry remains unchanged. Fixed edits preserve feature ID, anchor and operation through `SetOutline`. Save status appears only after the committed operation, accepted shape and durable revision agree.

## Ownership and browser boundaries

`outline_lifecycle.rs` retains current accepted scope/token/revision/generation checks, target resolution, operation construction, pending outcomes and durable feedback. The Layout canvas owns transient selected-point and pointer samples. It uses the existing canvas interaction arbiter so perimeter drags cannot compete with part placement or mirrored-pair interactions. Rust Core owns preview geometry, commit validation and history. `LayoutSnapSettings` owns the active snap preferences; a private Rust helper ports the pinned TypeScript snap policy verbatim without changing the policy.

The existing `EditPhase::Preview` scene crosses the current core worker boundary for live preview. A drag uses one captured transaction ID and revision; only its final sample may commit. Cancellation retires only that transaction's displayed preview, suppresses its matching in-flight reply, and cannot clear a later preview from another drag on the same accepted source. Existing IndexedDB durability remains authoritative.

The implementation adds a narrowly scoped application `Event::ClearPreview` settlement operation; no external product API, project schema, generated contract, renderer protocol or geometry algorithm changes are required. The existing Core `CopyOutline`, `SetOutline`, and `SelectOutline` operations remain authoritative.

## Acceptance

- A production Layout journey on the pinned fixture selects a canvas point, drags with grid and geometry guides, observes the matching preview, releases, and verifies one changed accepted point and one Undo/Redo transaction.
- Escape, pointer cancel, lost capture and board/document replacement restore the accepted geometry, release capture, and produce no commit.
- A first Generated change previews and commits a fixed copy while preserving Generated. A fixed version changes only the selected polygon feature.
- Arrow, Shift+Arrow, Delete and Backspace use the selected point, existing grid policy and minimum-three-points rule. Each accepted key action has one Undo entry.
- Existing Inspector point selection/editing, point insertion/removal boundaries, scope/readiness checks and durable result feedback remain intact. Parent F3.4/F3.7 paired save/reopen and contextual workflow gates remain open.
- Preserve RF-001, RF-006, RF-009; record this browser worker preview boundary and React bridge retirement evidence in the existing ledgers.

## Out of scope

Rectangle handles/parameters, PartEnvelope connections, manual additions/cutouts, bridge editing, gaps, refinements, new snapping algorithms, parent closure, React removal, global browser/accessibility/performance qualification, public API/schema visibility, and renderer or CAD changes.

## Refactoring and retirement record

Temporary TypeScript bridge: pinned React `OutlineControlOverlay` remains the parity reference until this and the existing F3.4c Inspector journey pass paired production browser, history and save/reopen acceptance. The bridge can retire only when the Layout perimeter handles, drag cancellation, snap policy, keyboard editing, Undo/Redo and fixture round trip pass in the integrated Dioxus candidate; accountable owner is the F3.4 migration coordinator. No React domain policy is to be retained or duplicated after retirement. The Rust snap helper has no TypeScript runtime caller and is retained as the migrated Layout owner.

RF-001 (single Editor operation ownership), RF-006 (canonical accepted Board/scope and geometry), and RF-009 (exact source/build/fixture evidence) remain in force. Current work demonstrates no separate refactoring finding.
