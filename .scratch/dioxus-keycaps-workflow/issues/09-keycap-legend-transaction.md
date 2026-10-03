# 09: Qualify per-key legend inheritance and history

**Parent:** F6C.2 — Board, matrix, and per-key Keycaps settings.

**Outcome:** Establish the exact accepted edit, Undo/Redo, and save/reopen behavior for a key legend that inherits its binding, is intentionally blank, or is explicitly typed.

**Specification:** [Per-key legend inheritance and history transaction](../keycaps-legend-transaction-spec.md).

**Start after:** F6C.1 accepted Keycaps projection/selection and the existing per-key editor/edit owner are available. The paired fixture and existing F6C.2 settings evidence can be reused; no parent acceptance gate is removed.

**Acceptance criteria**

- [x] Use the retained c6 fixture in separately named React and Dioxus sessions; capture fixture hash, active project/board, selected key/layer, accepted revision/history state where the public surface exposes it, source/build provenance, and viewport. Use actual Undo/Redo and saved/reopen outcomes where Dioxus does not expose a numeric revision.
- [x] With SW1 bound to `A`, prove the inherited state is `null` and visibly renders `A`; **Blank keycap** stores `Some("")` and visibly renders a dash; **Use binding legend** restores `null`.
- [x] Record accepted value, revision/history state, and visible feedback after real actions. Establish a valid React history owner/action before interpreting any empty-history response.
- [x] Type and commit a distinct legend using the reference blur behavior, then qualify Undo/Redo for that accepted edit in both applications.
- [x] Qualify Undo/Redo for blank-versus-inherited transitions according to the behavior established in React. Preserve the binding and matrix profile; do not change the independent per-key profile or other overrides.
- [x] Save/reopen explicit-blank and inherited states and verify the distinction persists in both applications.
- [x] The paired behavior matched; the prior inconclusive “History is empty” observation did not reproduce after establishing accepted history.
- [x] Reuse the existing per-key edit/history lifecycle. This child adds no writable state or API and closes no F6C.2/F6 acceptance join.

**Status:** focused paired browser journey passed on pinned React and candidate 34765; see [receipt](../evidence/keycap-legend-transaction-20261003/receipt-34765.md). The earlier React “History is empty” observation did not reproduce with a verified accepted-history baseline. Dioxus does not expose a numeric revision in its public DOM, so its transaction boundary is evidenced by accepted values, Undo/Redo, saved state, and reload. This closes only this child behavior check; F6C.2, F6, F6C.4, INT.2, and all other parent acceptance joins remain open.
