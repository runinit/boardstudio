# F6K.1c: Match the Keymap Layers disclosure in the Inspector

**Parent:** F6K.1 — Keymap projection and selection (`.scratch/dioxus-frontend-v1/workflows/F6.json`).

**Source specification:** [`10-keymap-layers-disclosure-spec.md`](../drafts/10-keymap-layers-disclosure-spec.md).

**What to build:** The Keymap Inspector presents its layer list and controls in an accessible “Layers” disclosure that starts expanded, can be collapsed without changing project data, and matches the pinned React control placement.

**Capability gate:** The existing F6K.1 Keymap panel, accepted layer projection and identity, and F6K.1b layer-operation callbacks are mounted and source-backed. This child reuses that private surface; it does not require a new Core operation, provider, public API, or parent acceptance join to start.

**Blocked by:** None for this bounded presentation child. F6K.1, F3.1, and other shared acceptance joins remain parent gates and are not new ticket dependencies.

**Acceptance:**

- [ ] Show a native, keyboard-operable disclosure with summary “Layers”, expanded on panel mount. Its descendants are hidden when collapsed.
- [ ] Preserve the user-selected open/closed state through layer selection and ordinary accepted-projection rerenders. A fresh panel mount defaults open, matching the React mount behavior.
- [ ] Keep the current layer list, Add layer, Layer name, permitted Remove layer action, layer-operation feedback, and precedence/transparency help text inside the disclosure without changing their existing callbacks or saved-data behavior.
- [ ] For Base, omit both the Remove layer action and the Dioxus-only “The first layer cannot be removed.” paragraph. Preserve the existing non-Base removal behavior.
- [ ] Match the owned Layers-section spacing/placement against pinned React using the same layered fixture. Do not absorb shared context/view toolbar, Objects panel, or footer differences.
- [ ] Toggling the disclosure does not change accepted project contents, revision, or undo/redo history.
- [ ] Add production-component regression coverage for default-open, collapse/reopen, rerender retention, accessible expanded state, Base/non-Base content, and no document/history mutation.
- [ ] Retain paired browser evidence and RF handoff. This child does not close F6K.1, F3.1, F3.3, F6K.2, or any parent acceptance join.

**Ownership:** Existing private Keymap Inspector panel and its local presentation styles. No shared composition, Runtime, Core, storage, or public contract changes.

**Profile:** Luna Medium author/verifier; Sol 6.1 High independent source/spec reviewer. Planning review is clear for frozen commit `ac474b4175c7d013b5cd73aa59ad5fe1309073c3`; report: `/home/chris/.local/share/boardstudio/reviews/keymap10-layers-disclosure-planning-review-ac474b41-sol-20261002.md`, SHA-256 `730649254a20f54ac4c1ca31519d8365d78af8f9c11015ab1056e3c33f4a135f`.

**Status:** implementing; source implementation is underway. Parent and shared acceptance remain open.
