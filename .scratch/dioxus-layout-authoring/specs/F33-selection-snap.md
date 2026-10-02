# F3.3a — Layout selection and snap controls

**Published child:** [F3.3a](../issues/05-layout-select-snap-toolbar.md), now retained on the integration branch with its source contract and independent review packet. This document expands that existing child spec; it does not create a second ticket.
**Parent:** F3.3 — Transforms, constraints and snapping. This child does not alter parent criteria, graph edges, or acceptance joins.
**Reference:** React source pinned at `5a472a9426e6e38993361da402cd4ec730feb369`; see the preserved [source-backed behavior contract](../evidence/selection-snap/toolbar-parity-contract.md), [Spec review](../evidence/selection-snap/source-spec-review.md), and [Standards review](../evidence/selection-snap/source-standards-review.md).

## Problem Statement

The Dioxus Layout canvas currently hard-codes grid pitch/fraction and snap behavior, and its toolbar does not expose the React Select/Snap controls, guide, or grid status. A user cannot choose the semantic extent of a canvas selection or adjust how a drag is snapped. The missing preference wiring makes the existing Session snap implementation inaccessible.

The reference also carries semantic drag behavior for matrices, rows, columns, keys, and assemblies that is not represented by Dioxus's current world-position `MoveParts` route. This child must not imply that those grouped drags are equivalent; they remain explicit F3.3 work under RF-005.

## Solution

Port the React Select and Snap controls into the existing Layout toolbar/footer. Make the root Editor the sole owner of their transient preferences and admission checks. Reuse the existing scoped selection adapter and Session gesture flow; pass validated snap settings once into `GestureBegin` and render Session's snap-guide/status projection. Preserve React placement, labels, defaults, edit timing, and selection semantics.

This child covers Select semantic-context behavior and snapping for a real standalone component only. Matrix/row/column/key/assembly drag-operation parity, Transform, Align, Constraints, camera/fit, findings navigation, placement-specific snapping, and outline-point snapping remain owned by their existing parent/child workflows.

## User Stories

1. As a Layout user, I want the Select menu to offer Matrix, Row, Column, Key, and Part at the existing toolbar location, so that I can choose the same edit scope as in React.
2. As a Layout user, I want choosing a mode to update the current valid selection immediately, so that the tree, canvas, and Inspector stay synchronized.
3. As a Layout user, I want the chosen mode to govern the next unmodified canvas hit, so that a toolbar choice remains useful after I click another target.
4. As a Layout user, I want Shift/Ctrl/Cmd selection modifiers and range anchors to retain their current meaning, so that changing the selection kind does not change multi-select behavior.
5. As a Layout user, I want empty cells to remain addressable semantic Key contexts, so that I can select a cell without the application fabricating a component identity.
6. As a Layout user, I want Part selection to resolve to a real live component in a populated cell, so that the Inspector and edits target actual project data.
7. As a Layout user, I want the grid menu to offer Off, ⅛u, ¼u, ½u, 1u, 1 mm, 0.5 mm, and 0.1 mm with ¼u as the default, so that I can place objects at reference increments.
8. As a Layout user, I want unit increments to use the accepted owning matrix pitch and the existing 19.05 mm fallback for non-matrix targets, so that grid movement is predictable across scopes.
9. As a Layout user, I want geometry snap and envelope-gap snap controls plus an editable nonnegative gap override, so that placement responds to real geometry and configured clearances.
10. As a Layout user, I want blank gap input to use the active matrix edge gap or the 1 mm fallback, so that default behavior matches React.
11. As a Layout user, I want gap snap disabled when geometry snap is off while the gap draft remains editable, so that I can prepare a value without triggering disabled snapping.
12. As a Layout user, I want Alt to bypass grid and geometry snapping during a gesture, so that I can make precise unconstrained movement.
13. As a Layout user, I want a snap guide while a snapped drag is active and a clear guide after commit/cancel, so that feedback never refers to a completed gesture.
14. As a Layout user, I want the footer to show the selected part's world X/Y, grid increment, geometry-snap status and existing undo/save signals, so that the canvas status reflects the selected object and active preferences.
15. As a Layout user, I want switching board/workspace or receiving a stale callback to leave the new scope untouched, so that delayed UI events cannot retarget another board.
16. As a Layout user, I want snapping preferences captured at drag admission, while Alt remains live, so that changing a menu during an active gesture does not mutate that gesture.
17. As a maintainer, I want the toolbar to receive immutable values and emit typed intents, so that selection and snap state continue to have one Editor-owned authority.
18. As a maintainer, I want the behavior checked through paired React and Dioxus public journeys on the same fixture, so that helper tests cannot be mistaken for visible parity.

## Implementation Decisions

- Keep the existing root Editor as owner of one `LayoutSelectionKind`, one `LayoutSnapSettings`, and a scope/generation-bound retained cell anchor. The private leaf renders a projection and emits typed callbacks; it owns no selection store, writable signal, Runtime access, accepted document, or CAD state.
- Keep semantic selection kind separate from `application::SelectionMode` modifier semantics. Convert the current tree/canvas hit using accepted membership; Matrix/Row/Column use live scene membership, Key resolves a real primary where available, Part resolves a real live component, and an empty cell remains semantic with zero selected IDs.
- Seed the retained cell coordinate from a real accepted hit. Retain it only across valid scope-kind conversions and invalidate it on owner/context change or missing/out-of-range membership. Do not reconstruct an omitted coordinate as zero on each toolbar selection.
- Root validates Layout workspace, full `Scope`, presentation generation, accepted token/revision, and target membership before preference or selection changes. Preserve existing Shift/Ctrl/Cmd paths and anchors.
- Snap settings are transient presentation preferences shared by current Layout toolbar consumers. Validate supported finite increments. Keep gap text draft editable even with geometry snap disabled; parse a finite nonnegative override, while blank/invalid text uses React's current fallback to accepted matrix `edgeGap.x`, then 1 mm. Encode through the existing gesture contract: zero Off, positive pitch fractions, negative millimeter steps; `GestureBegin` alone captures pitch/fraction/geometry/gap. Samples/end carry positions and live Alt. Do not alter events, normalize twice, or rewrite an active gesture.
- Use the current accepted owning matrix's pitch/gap where applicable; otherwise preserve React's 19.05 mm pitch and 1 mm gap fallbacks. Send `gap=None` unless geometry and envelope-gap snap are enabled. Read and clear the existing Session snap guide; do not calculate a second guide in presentation code.
- Preserve reference control placement in the Layout toolbar and footer. Do not mount Transform, Align, Relationships, Fit, or Findings controls in this child without their complete behavior.
- Preserve the existing parent graph: F3.3 keeps `start_after=[F3.1]` and `acceptance_after=[F3.2]`. The child is independently schedulable after the capability gates below; its public acceptance remains evidence for, not a replacement of, the full parent joins.
- No Core geometry algorithm, public API/type/visibility, schema, Session protocol, independent document state, or React redesign is authorized.

## Testing Decisions

- Highest seam: same-fixture public-browser comparison on a saved REVIUNG41 project, with captures and action/state trail for both applications. Exercise actual toolbar, canvas, Inspector, footer, saved state and history rather than a hidden test-only route.
- Selection journey: choose Matrix, Row, Column, Key, Part; inspect tree/canvas/Inspector synchronization and next unmodified canvas hit; include a populated and empty cell, no-current-selection choice, scope switch/stale callback, and unchanged Shift/Ctrl/Cmd behavior.
- Snap journey: drag a real standalone component (not a matrix member) using ¼u, 1 mm and Off; verify accepted coordinates, preview, guide/status, 1 Undo step, Alt bypass, geometry/gap toggles, override/fallback, and cancel/pointer-cancel/lost-capture/board-switch cleanup. Include stationary click and small nonzero movement regressions required by F3.3.
- Save/reopen must preserve committed geometry; cancelled edits must leave accepted geometry and history unchanged. Verify keyboard/focus, desktop and compact placement, light and dark themes, and browser console for changed controls.
- Test private selection conversion/anchor and snap projection as useful support. Native/WASM checks cover affected Rust modules; public journey remains the acceptance seam.
- Prior art includes `app/e2e/outline-versions.spec.ts`, existing paired Layout matrix evidence, the current Session gesture tests, and React `Workbench.tsx`/`useWorkbenchSelection.ts` behavior. The authored source module has Spec and Standards clearance; those reviews inspected but did not run Cargo or browser verification.

## Out of Scope

- Matrix/row/column/key/assembly drag parity and local coordinate snapping; direct position/rotation; matrix stagger/splay/origin; Align and constraints; arrow-key canvas nudge; Fit/camera and findings navigation.
- Perimeter, placement, and point-edit snapping integration under F3.2/F3.4; changing global shell, theme, panel, project, PCB, Case, Keymap, Keycaps or Parts behavior.
- New geometry algorithms, public APIs, document formats, migration or backend behavior.

## Further Notes

### Capability-level start gate

The child can start when the coordinator confirms these existing capabilities at the current integration revision: selected real IDs and active-board/session scope from F3.1; T1-10 semantic tree contexts and current membership resolver; root Editor lifetime/current-scope subscriptions and stable composition callbacks; existing `GestureBegin/Sample/End/Cancel` flow with `normalize_drag`, Session guide projection, accepted document/scene, and undo/save/reopen. The toolbar helper source is already reviewed at `9db0579a4848b85f730448d98e92767cf75cd202`; its root mount obligations remain implementation work. This gate does not declare F3.1 or F3.2 accepted.

### Paired acceptance route

On the identical saved REVIUNG41 fixture, start in Layout with no selection; choose Column and hit a real matrix cell, then move through Matrix → Row → Column → Key → Part and inspect the selected IDs and contextual Inspector after each step. Repeat with an empty cell and verify no synthetic ID; switch board between capture and callback to prove stale admission is rejected. Select a real standalone board component, test ¼u, 1 mm, and Off with geometry/gap options and Alt, compare preview/final world coordinate, footer and guide, Undo once, cancel another movement, then save/reopen. Capture desktop/compact and light/dark states in each app. Do not drag a matrix member as proof of snap parity.

### Refactoring ledger handoff

Update the existing RF-005 entry (“Geometric edit planning lives in frontend helper policy”) with this source-confirmed React matrix/local-snap versus Dioxus world-`MoveParts` boundary; retain RF-001 as shared composition context if root-mount ownership causes additional evidence. Do not mint an RF ID for the known boundary, edit the root ledger from the feature ticket, claim a public regression without paired mutation evidence, or propose a broad refactor as a blocker. The feature author reports a scoped handoff to the coordinator for the canonical register.
