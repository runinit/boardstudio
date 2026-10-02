# F6C.2 Keycap settings and contextual editor parity

## Problem Statement

The Dioxus Keycaps workbench can render the board's physical keycaps and exposes a selected-key override editor, but users cannot edit board-level keycap defaults or matrix-level profile settings. The workbench therefore shows only a partial view of saved keycap behavior and cannot reproduce the everyday settings workflow available in the TypeScript application.

## Solution

Port the existing TypeScript Keycaps settings into the Dioxus contextual Inspector. Preserve control grouping, labels, choice sets, inheritance, empty states, edit timing, validation, and visible feedback. Implement the Board colors and Matrix profiles sections as separately demoable vertical slices. Keep the existing per-key editor within the F6C.2 parent scope and verify it alongside the new sections during parent acceptance.

## User Stories

1. As a keyboard designer, I want to set the board's default keycap color so that keys without individual overrides have the intended appearance.
2. As a keyboard designer, I want to set the board's default legend color so that legends remain readable across the board.
3. As a keyboard designer, I want to set minimum keycap clearance so that fit checks use the spacing I intend.
4. As a keyboard designer, I want each matrix to inherit a keycap profile so that a row or matrix can use a consistent sculpted shape.
5. As a keyboard designer, I want to adjust a matrix's first profile row and wall thickness so that generated keycap specifications match its geometry.
6. As a keyboard designer, I want to choose a matrix socket or inherit it from switch profiles so that supported MX, Choc, and Alps switches resolve correctly.
7. As a keyboard designer, I want matrix settings to remain separate from per-key overrides so that exceptions do not unexpectedly change neighboring keys.
8. As a keyboard designer, I want the Inspector to show settings for the active board and its matrices so that switching boards never edits another board's values.
9. As a keyboard designer, I want an empty-matrix explanation when a board has no matrices so that I understand standalone switches use their individual overrides.
10. As a keyboard designer, I want each committed setting to update the physical Keycaps view and fit findings so that the result is visible immediately.
11. As a keyboard designer, I want Undo and Redo to restore settings and their visual effects so that experiments are reversible.
12. As a keyboard designer, I want settings to survive save and reopen so that edits are durable.
13. As a keyboard designer, I want rejected edits to explain the problem and retain the last accepted value so that invalid settings do not silently appear saved.

## Implementation Decisions

- Use the existing board-default, matrix-profile, and per-key edit operations and standard document history. Do not add schema fields, validation behavior, public APIs, or new persistence paths.
- Route all three edit scopes through the existing Editor-owned admission/outcome lifecycle. Admission captures and revalidates the accepted snapshot token, revision, full canonical `Scope`, active board identity, and matrix membership where applicable. Keep admission mounted and unconditional at the Editor scope, settle each exact `OutcomeSlot` before suppressing work for hidden workspace/pane state, and do not create a second edit owner inside the Keycaps panel.
- Board controls operate on the active board only. Matrix controls operate on the matrix identified by stable matrix identity, not its display name.
- Preserve the TypeScript profile and socket choices, supported numeric bounds and increments, and the distinction between inherited (`null`) and explicit values.
- Keep matrix defaults and per-key overrides independent. Effective rendering and fit resolution continue to use existing precedence and core resolution behavior.
- Preserve TypeScript control grouping and contextual placement in the Keycaps Inspector. A deliberate platform adjustment requires explicit evidence and a recorded decision.
- Match the reference's observable edit timing, not a framework handler name. React's synthetic `onChange` behavior for color/select/number inputs must be mapped to Dioxus events by browser evidence: type numeric values, use number spinners, and choose a color; record when the accepted revision and history entry change. Do not assume native Dioxus `onchange` is equivalent, or add blur-only staging unless the paired journey proves it matches.
- Child tickets may start when the F6C.1 board-scoped projection and accepted edit/history capability needed by that child are proven. This capability gate does not remove F6C.2's parent dependency or any F6 parent acceptance joins.

## Testing Decisions

- Use paired TypeScript and Dioxus browser journeys on the same verified saved project and board. Capture the active project ID, board ID, revision, build provenance, and screenshots with each result.
- Exercise control placement, displayed effective values, a durable edit, changed 2D appearance or finding state, Undo, Redo, save, reload/reopen, and board switching.
- For matrix controls, cover each supported profile and mount option, inherited mount, numeric bounds, missing matrices, standalone switches, and matrix/per-key precedence.
- For board controls, cover both color defaults, clearance, an individual color override, board switching, and preservation of other board values.
- Record browser event timing for text entry, spinner increments, and color picker selection, including accepted revision/history count; assert stale or rejected operations preserve the previous accepted values.
- Switch workspaces/panes while edit outcomes are pending and verify exact outcomes settle before hidden-pane suppression; switch project, board, and matrix scope to prove late results cannot apply elsewhere.
- Test actual user-visible behavior through the public browser surface. Unit tests support edit mapping and edge cases but do not replace paired acceptance.
- Follow existing Keycaps projection, edit lifecycle, and browser fixture tests; retain the independent Standards and Spec reviews and refactoring handoff required by the migration workflow.

## Out of Scope

- Keycaps 3D preview, shared viewer ownership, and STEP export.
- New keycap profiles, sockets, dimensions, resolver rules, validation semantics, or Core/CAD algorithms.
- Shared shell redesign or global Inspector extraction.
- Keymap label/binding editing and Layout key resizing/reflow.
- Closing F6C.2, F6C.4, or any parent integration join merely because one child ticket passes.

## Further Notes

The current Dioxus fixture view provides an evidence-backed gap: it lists matrices and offers physical key selection but no board defaults or matrix profile fields. TypeScript shows these under Board colors and Matrix profiles. Existing per-key override code is a separate path and remains subject to the parent F6C.2 paired journey.
