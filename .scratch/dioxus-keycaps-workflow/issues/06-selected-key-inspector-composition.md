# 06: Match the contextual Selected key Inspector section

**Parent:** F6C.2 — Board, matrix, and per-key Keycaps settings.

**Outcome:** Search, key selection and the selected key's override editor live in the same default-open contextual disclosure as the pinned React Keycaps Inspector.

**Specification:** [Selected-key Inspector composition](../keycaps-selected-key-inspector-spec.md). **Source inventory:** [Keycaps workbench parity inventory](../evidence/keycaps-workbench-inventory-20261002.md).

**Start after:** F6C.1 accepted Keycaps projection/selection and the existing per-key editor mount are proven. The existing Board colors and Matrix profiles tickets remain independent.

**Acceptance joins:** F6C.2 own-slice acceptance; no parent join is changed.

## Acceptance criteria

- [ ] Match React section title exactly (`<reference> · key` / `Select a key`), default-open state, key search, selected-key picker and override fields within one disclosure.
- [ ] Remove the extra Dioxus-only standalone Matrices list and Selected keycap summary; keep matrix settings in Matrix profiles and do not introduce read-only summary fields that React omits.
- [ ] Preserve canvas/list/dropdown selection behavior and dynamic title without adding a second selected-key owner.
- [ ] Verify pointer and keyboard collapse/reopen and preserve the user's open/closed choice across key changes.
- [ ] Paired same-fixture browser evidence confirms both extra regions are absent while all React controls remain available.
- [ ] Reuse the current per-key Editor owner and prove accepted edit, Undo/Redo and save/reopen through paired same-fixture browser journey.
- [ ] Preserve the full F6C.2 parent, F6C.4, INT.2, and RF-001 criteria; this child does not close them.

**Status:** implementation candidate prepared; independent source review and paired browser acceptance remain open. This child does not close F6C.2 or any parent join.
