# F3.4b — Active Board outline Inspector settings

**Parent:** F3.4 — Outline editing and refinement UI. This child follows F3.4a and consumes T1-11's existing Outline/version route; it does not change parent criteria or dependency edges.
**Reference:** Pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/OutlineInspector.tsx` and `app/src/ui/Workbench.tsx`; existing Rust outline model and Core operations.

## Problem

The current Dioxus `OutlineVersionInspector` shows an accepted contour preview and Copy/Delete actions, but React's Board outline Inspector also exposes saved-version selection/rename, generator/settings controls, Keep gap and cleanup/clearance settings. These are supported by the current document model, geometry scene and Core edit/history path but are not reachable in the Dioxus Layout UI.

## Observable behavior

The user can work from the existing Objects → Board → Outline selection. The Inspector displays current accepted board/version data; Generated remains derived and follows current inputs, while settings and geometry for a fixed version come from that accepted version. The Inspector can activate Generated or a saved version, rename a saved version, create a missing automatic `part-envelope` feature, update the applicable settings, and protect/remove accepted Generated gaps. Each change is a normal edit with exact terminal feedback and Undo/Redo support.

Match React's details:

- `OutlineInspector.tsx` provides the Active outline selector, version name editor, Copy/Delete actions, Edit perimeter points button, Generate automatic outline action, Generated margin and bridge width, corners/size for Generated or fixed versions, Gap repair/Keep gap, and Advanced cleanup/clearance controls.
- This child implements the selector/rename and supported settings/repair controls only. F3.4a continues to own Copy/Delete lifecycle. The perimeter-point editor button is not rendered until a real Dioxus point-edit route exists. Manual geometry, bridge editing and refinements remain distinct F3.4 work.
- Selecting the Outline parent row does not activate or clear a version; selecting Generated/saved version invokes the existing accepted `SelectOutline` path. The Inspector selector uses that same operation from the Outline context.
- Protected gaps store the accepted gap ID and source-linked control points on their generating `PartEnvelope`; unchecking removes only the same stable/protected ID family. Fixed versions do not offer Generated-only protection/cleanup controls.
- Numeric fields use local drafts, validate finite supported values, commit on blur/Enter and restore the accepted value on Escape. No edit is submitted on each keystroke.

## Ownership and operation lifecycle

The private leaf receives a read-only accepted projection and typed intents. `outline_lifecycle.rs` remains the single Editor-owned operation/outcome owner. It admits intents against current Layout workspace, full `Scope`, accepted document/session/token/revision, board and generation; it validates membership and readiness, allocates/registers a fresh operation observer before submission, sends existing `Event::Edit`, and correlates the exact terminal outcome. Pending settlement remains alive when the Inspector hides. No optimistic active-version or saved flag is introduced.

Use existing Core operations: `SelectOutline`, `RenameOutline`, `SetOutline`, and the existing edit/document operation for adding a generator or changing fields held by `BoardOutline` fixed-version state. Do not expose public API/schema visibility. Core remains the sole geometry and history authority. If inspection proves a new public operation is essential, keep the adapter private first and document why existing operations cannot express the requested behavior; include compatibility and generated-contract tests before using it.

## Verification

- Mounted production tests exercise the rendered Inspector callbacks and exact admitted owner for selector, rename, settings, generated-feature creation, protected gap and cleanup edits.
- Assert wrong-board/missing-version/stale-generation/no-longer-current gap actions submit nothing, and exact rejection/persistence/recovery outcomes do not claim Saved.
- Assert finite/range validation, accepted baseline refresh, Enter/blur commit and Escape restoration.
- Existing Core outline/version/repair/history tests remain the domain oracle. Public paired behavior requires a real multi-version board project: edit a Generated setting and gap, Undo/Redo, switch fixed/Generated, rename, save/reopen, and compare accepted document/scene plus the visible Inspector to pinned React.
- Keep desktop/compact placement, keyboard focus, console and current tree/navigation behavior in the integrated F3.4/F3.7 gates. These source checks do not accept those parents.

## Non-goals

Perimeter-point/bridge/cutout/addition editing, pointer capture and point snapping; gap-focus camera/highlight routing; refinement propagation; findings navigation; Case physical-instance context; F3.4/F3.7 parent completion; new Core geometry algorithms, schema fields or public visibility.

## Refactoring record

Preserve RF-001 (Editor composition/operation ownership), RF-006 (canonical board/scope and geometry source), and RF-009 (exact build/source/fixture and visible-versus-authoritative evidence). No distinct refactoring issue is presumed; add one only if source evidence demonstrates a separate durable problem.
