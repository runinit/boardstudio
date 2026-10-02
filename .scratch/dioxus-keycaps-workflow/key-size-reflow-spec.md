# F6C.3 — Shared key-size drafts and linked resize/reflow

**Parent:** F6C — Keymap and Keycaps Dioxus workflows. This refines existing F6C.3; it does not edit the parent graph or its joins.

## Problem Statement

In the React application, the Layout Inspector exposes key-cap size controls whenever the current selection contains supported keycaps. Designers can inspect a single size or a mixed selection, adjust width or height in quarter-unit steps, choose Wide or Tall, and see adjacent keys reposition when a resize would make their caps overlap. Linked halves resize consistently, edits are undoable as one operation, and the visual projection follows the accepted document.

The current Dioxus port does not yet provide this shared key-size interaction. The Keycaps workbench has physical 2D dimensions and per-key size overrides, but those do not replace the Layout Inspector's group-aware size controls or its selection-aware neighbor reflow. Without that path, users must edit individual values without the reference's draft, axis-commit, spacing, centering, or linked-half behavior.

## Solution

Port the existing React key-size interaction as one private Dioxus slice in the Layout Inspector. Derive displayed units from the selected keycaps' effective sizes, matrix pitches, and edge gaps. Keep slider drafts local to the mounted control and commit through the existing accepted-document/history path. Preserve the current selection and matrix projections as authoritative; compute the resize/reflow plan from those accepted values and submit one durable edit.

Width and height remain independent axes. Pointer release, focus leaving a control, keyboard adjustment, and Wide/Tall actions retain the reference commit boundaries and duplicate-commit protection. Resizing selected keys repositions row and column neighbors only as needed, using the matrix's projected axes. Mirrored pairs use one canonical resize plan and preserve their shared geometry. The displayed 2D cap geometry and any remaining overlap warning derive from the accepted result.

The start gate is capability-based: F6C.1's active-board Keycaps projection and stable key/matrix identities, plus the already mounted Layout Inspector's accepted board, matrix cell projection, selection scope, linked-layout metadata when present, and existing edit/history path must be demonstrably available on the implementation fixture. F6C.3's existing F3.2 and F3.5 acceptance joins stay in the parent graph and stay open until their full criteria are met; they are not blanket prerequisites for beginning the proven fixture path.

## User Stories

1. As a keyboard designer, I want a Key size section in the contextual Layout Inspector when supported keycaps are selected, so that I can resize caps where I already inspect matrix and key properties.
2. As a keyboard designer, I want width and height displayed in units derived from the selected caps, pitch, and edge gap, so that the controls reflect effective physical cap geometry rather than raw millimeters.
3. As a keyboard designer, I want a clear size readout for a uniform selection, so that I can understand the current shape before changing it.
4. As a keyboard designer, I want a Mixed readout when the selection contains different sizes, so that the first selected key is not mistaken for the value of every selected cap.
5. As a keyboard designer, I want width and height sliders from 1u to 7u in 0.25u increments, so that I can make the same common size adjustments as in React.
6. As a keyboard designer, I want changing one axis to preserve the other axis for every selected key, so that widening does not unexpectedly change cap depth and vice versa.
7. As a keyboard designer, I want pointer and keyboard adjustments to commit at the same observable boundaries as React, so that edits feel predictable and do not create one history entry per slider tick.
8. As a keyboard designer, I want keyboard adjustment and blur to follow the reference's timer and sent-value/mixed-state rules, so that blur does not duplicate an already-sent change while a distinct subsequent value remains a separate accepted operation.
9. As a keyboard designer, I want Wide and Tall to derive one long/short pair from the current size draft and apply that common pair to every selected cap in one action, so that I can set a consistent selection shape without manually moving two sliders. For a Mixed selection, the draft starts from the first selected cap; the action intentionally normalizes the selected caps to that draft-derived pair.
10. As a keyboard designer, I want selected caps to retain their group center while neighbors move only enough to clear resized cap envelopes, so that resizing does not drift the design.
11. As a keyboard designer, I want row neighbors to move along the projected row axis and column neighbors along the projected column axis, so that splay, rotation, and mirror geometry remain coherent.
12. As a keyboard designer, I want linked halves to receive equivalent sizes while their canonical geometry is planned once, so that selecting one or both halves produces one consistent result.
13. As a keyboard designer, I want matrix-local offsets, disabled cells, rotations, and unrelated project data preserved, so that resizing caps does not rewrite unrelated layout state.
14. As a keyboard designer, I want remaining cap overlaps reported in the Inspector, so that cases that cannot be resolved by row/column reflow remain visible.
15. As a keyboard designer, I want no resize edit for empty, unsupported, unchanged, or stale selection input, so that the app does not manufacture sizes or add meaningless history records.
16. As a keyboard designer, I want one resize to undo and redo as one change, so that a multi-key reflow is practical to revise.
17. As a keyboard designer, I want slider drafts to reset when the active selection, board, project, or selection scope changes, so that old drafts cannot leak into a different target.
18. As a keyboard designer, I want saved sizes and reflowed positions to survive reload and project reopen, so that the result remains part of the keyboard design.
19. As a reviewer, I want identical pinned React and Dioxus journeys on the same fixture, so that control placement, event timing, resulting geometry, history, and persistence can be compared directly.

## Implementation Decisions

- Keep key-size controls in the Layout Inspector's current contextual matrix/key selection path, not in a new global panel or a duplicate Keycaps-side resize editor.
- Derive selection membership from the accepted active-board selection and existing keycap placements. Only supported keycap items participate; do not infer items from a stale selection or another board.
- Convert millimeters to displayed units using the reference rule: round `(effective size + matrix edge gap) / pitch` to the nearest quarter unit. Show the first selected value while explicitly marking heterogeneous selections as Mixed.
- Keep slider drafts ephemeral. Reset them when their selection identity/scope or accepted source changes. Commit only on the characterized pointer/key/blur boundary, deduplicating the same axis commit across pointer-up and blur.
- Preserve the existing unit range (1–7u) and 0.25u increment. Wide/Tall uses the current control draft's larger and smaller dimensions, then applies that common pair to every selected cap; for Mixed selection the initial draft comes from the first selected cap. It commits both axes as one user action.
- Preserve React's plan behavior: calculate selected cap dimensions in physical units; reflow affected members in the same matrix row or column along the corresponding projected axis; retain existing cell rotations/offsets; preserve the selected group center; and compute target IDs for selected caps, moved neighbors, and affected matrices.
- For linked layouts, resolve one canonical source and deduplicate the mirrored member even if both halves are selected. Let the existing accepted layout resolution propagate the source movement; do not issue two competing edits.
- Submit the complete plan once through the existing private UI edit/history adapter and existing accepted-document command path. Do not add a persisted draft schema, a new public Rust API, or a second document store.
- Continue using the existing overlap projection and Inspector warning after acceptance; this slice does not promise to eliminate every overlap or add a new collision model.
- Keep the F6C.3 F3.2 and F3.5 parent acceptance joins intact. Begin only after the named capability start gates are evidenced; parent completion and final parity still require every existing join.

## Testing Decisions

- The public acceptance seam is a paired browser journey through Layout Inspector, physical keycap canvas, accepted project history, and reload/reopen using the same pinned React source and same Dioxus build/fixture. Compare control presence and placement, names/ranges, Mixed behavior, pointer and keyboard commit timing, resulting sizes and positions, overlap feedback, Undo/Redo, and persistence.
- Exercise key, row, column, and matrix selection; uniform and mixed sizes (including two differently sized caps selected together before Wide/Tall); empty and unsupported selections; rotated/splayed columns; disabled cells; a mirrored pair selected from either side and both sides; and an unrelated neighboring matrix.
- Record the pinned React's actual accepted-operation sequence for pointer release, keyboard timer settlement, blur, and Wide/Tall. Require Dioxus to match the observed count and order, including duplicate suppression only when the source's sent-value/mixed-state signature identifies the follow-up as the same action; do not assume every gesture or edit-session maps to exactly one operation. Assert target geometry and accepted revision for each operation and no edit/revision for no-op input.
- Add focused Rust tests around the private resize planner for unit conversion, per-axis sizing, row/column reflow, group-center preservation, matrix-local offset conversion, canonical mirror deduplication, and preservation of unrelated document data. Tests should assert outputs visible at the accepted project/geometry seam, not component internals.
- Add mounted browser regressions for draft reset after selection/project change and keyboard/pointer commit timing; pure planner tests cannot establish reactive draft ownership or visible control behavior.
- Reuse existing React key-size, resize-plan, reflow, matrix geometry, mirrored-layout, Core accepted-edit, and save/history tests as prior art. They support the port but do not replace paired Dioxus browser evidence.
- Run format, affected native tests, release WASM/page checks, strict checks required by the coordinator, the source-stamped build after review, and the paired browser journey. Record reviewer verdicts and refactoring handoff without closing F6C.3 or its joins.

## Out of Scope

- Changing keycap profile defaults, per-key override semantics, legends, board colors, matrix profile choices, or clearance resolver findings; those are F6C.2/F6C.4 slices.
- Creating matrices, custom layouts, mirrored pairs, or standalone components; those remain F3 authoring behavior.
- Rewriting geometry policy in Core/CAD, adding a persisted schema or public edit operation, changing validation rules, or changing the application document/history model.
- A new global inspector or a second key-size editor in the Keycaps workbench.
- 3D assembly viewer, keycap mesh generation, STEP export, or CAD worker lifecycle; those remain F6C.5/F7/BND.1.
- Closing the F6C.3/F6 parent, F3.2/F3.5 joins, or any shared run state from this child result alone.

## Further Notes

The pinned React path is the Layout Inspector's `KeySizeControls` wired through `MatrixInspectorPanel` and the Workbench resize callback. It computes a complete document plan in frontend interaction helpers; this policy is not a directly callable Rust resize/reflow service. The Dioxus port should keep the planner private and consume the accepted matrix projections and existing edit/history authority.

The scope is deliberately one ticket: slider drafts, per-axis commit boundaries, neighbor reflow, linked-half deduplication, and the single history plan are one inseparable accepted-document interaction. Splitting axes or the control from reflow would create an intermediate resize behavior that differs from the reference on normal multi-key boards.

### Refactoring handoff

Carry forward **RF-005** with source-backed evidence that key-size policy spans selection normalization, pitch/gap conversion, geometric reflow, linked-layout deduplication, local matrix-offset projection, and whole-document edit planning in the React UI layer. This migration must preserve that behavior in the private Dioxus presenter/controller; after parity, assess a stable domain-owned resize/reflow planner and a narrower accepted-edit representation. Keep the assessment deferred until measured and reviewed; no new Rust public API is implied. Also retain RF-001 for shared Layout Inspector/composition ownership and RF-009 for paired UI/evidence provenance if implementation reveals cross-workflow coupling.
