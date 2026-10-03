# F6C.2 specification — Per-key legend inheritance and history transaction

## Problem

Keycaps distinguishes an inherited legend (`null`, rendered from the active key binding) from an intentionally blank legend (`Some("")`). The paired source evidence proves the visible `A` versus dash transition and persistence of a blank value, but it does not yet establish the React history boundary for these two values. One earlier attempt surfaced “History is empty” without proving that the action had entered React's history owner, so it cannot be treated as a defect or as an expected outcome.

## Intended behavior

Preserve the existing per-key edit owner and command/history path. “Use binding legend” stores `null`; “Blank keycap” stores `Some("")`; typing a legend and committing it stores that exact string. The active binding, matrix profile, per-key profile, and other override fields remain unchanged by a legend-only command. Accepted values update the physical Keycaps view and survive the normal save/reopen path. Undo and Redo behavior must match the pinned React result, established with a valid action sequence and the active project's actual history state.

## Acceptance journey

- Start from the retained c6 fixture in separate, named React and Dioxus sessions. Record fixture hash, active project and board IDs, selected key/layer, accepted revision, history availability, source/build provenance, and viewport. Set SW1's active-layer binding to `A` using the existing Keymap owner and confirm the Keycaps legend is inherited (`null`, empty editor with `A` placeholder).
- In each application, invoke **Blank keycap** and capture the accepted legend value, revision/history change, visible dash, and any operation feedback. Use the actual Undo/Redo controls and record whether each restores inherited `A` or explicit blank. Establish that the relevant command was accepted before interpreting an empty-history response.
- From the inherited state, type a different legend, commit using the reference's blur behavior, and record the same accepted value/revision/history/visual transition. Undo and Redo that accepted edit through the normal owner.
- Invoke **Use binding legend** from the explicit-blank state and verify it restores inheritance. Undo and Redo the transition if the pinned React owner records it. For each transition, verify the matrix profile, independent per-key profile, binding, and other key overrides remain intact.
- Save/reload or reopen after the final explicit-blank and inherited states in each app, and confirm the serialized distinction survives. Record visible feedback for a rejected or superseded operation only if the journey reproduces one; do not add a synthetic fault case solely for this UI slice.
- If React's valid owner workflow does not place one of these actions in history, record the observed behavior and the exact boundary (action, accepted revision and history state). Do not require a Dioxus behavior unsupported by the reference and do not label the prior inconclusive “History is empty” observation as a defect.

## Scope and boundaries

- Reuse the existing `SetKeycapKey` path, Editor-owned admission/outcome handling, accepted snapshot and document history. Do not add a second writable state, schema field, public API, or panel-local history owner.
- Keep board, key, and layer identity exact across the paired action. Late or rejected results must not update a different accepted project/board/selection.
- This is a focused acceptance child for F6C.2 stories 25–27. It does not re-run all per-key fields, matrix choices, native color-picker behavior, 3D lifecycle, or the parent acceptance suite.
- Keep F6C.2, F6, and all existing acceptance joins open. Preserve all parent criteria and existing RF observations; this child proposes no new refactoring program.

## Evidence limits

Current receipts establish visible inheritance-versus-blank rendering, Dioxus Undo/Redo and reload on one route, and blank persistence in React. The React history attempt is inconclusive because it did not establish that a legend edit entered history. A new paired journey is required before making a cross-application history claim.
