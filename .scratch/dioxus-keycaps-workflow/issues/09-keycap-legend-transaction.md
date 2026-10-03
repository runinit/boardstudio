# 09: Qualify per-key legend inheritance and history

**Parent:** F6C.2 — Board, matrix, and per-key Keycaps settings.

**Outcome:** Establish the exact accepted edit, Undo/Redo, and save/reopen behavior for a key legend that inherits its binding, is intentionally blank, or is explicitly typed.

**Specification:** [Per-key legend inheritance and history transaction](../keycaps-legend-transaction-spec.md).

**Start after:** F6C.1 accepted Keycaps projection/selection and the existing per-key editor/edit owner are available. The paired fixture and existing F6C.2 settings evidence can be reused; no parent acceptance gate is removed.

**Acceptance criteria**

- [ ] Use the retained c6 fixture in separately named React and Dioxus sessions; capture fixture hash, active project/board, selected key/layer, accepted revision, history state, source/build provenance, and viewport.
- [ ] With SW1 bound to `A`, prove the inherited state is `null` and visibly renders `A`; **Blank keycap** stores `Some("")` and visibly renders a dash; **Use binding legend** restores `null`.
- [ ] Record accepted value, revision/history state, and visible feedback after each real action. Establish a valid React history owner/action before interpreting any empty-history response.
- [ ] Type and commit a distinct legend using the reference blur behavior, then qualify Undo/Redo for that accepted edit in both applications.
- [ ] Qualify Undo/Redo for blank-versus-inherited transitions according to the behavior actually established in React. Preserve binding, matrix profile, per-key profile, and unrelated overrides throughout.
- [ ] Save/reopen explicit-blank and inherited states and verify the distinction persists in both applications.
- [ ] If the paired source result differs, report the precise action and owner boundary as the reproduced defect; do not infer a defect from the prior inconclusive “History is empty” message.
- [ ] Reuse the existing per-key edit/history lifecycle. This child adds no writable state or API and closes no F6C.2/F6 acceptance join.

**Status:** queued for paired owner-valid browser qualification; implementation path already exists. The prior visual and persistence receipts are useful evidence but do not qualify React's legend history boundary.
