# 10: Match the Keymap Layers disclosure in the Inspector

**What to build:** The Keymap Inspector presents its layer list and controls in an accessible “Layers” disclosure that starts expanded, can be collapsed without changing project data, and matches the pinned React control placement.

**Blocked by:** None for this bounded presentation child. It reuses the existing F6K.1 Keymap panel, accepted projection, layer identity, and operation callbacks. F6K.1, F3.1, and other shared acceptance joins remain parent gates and are not new ticket dependencies.

**Status:** draft; awaiting independent planning review.

- [ ] Show a native, keyboard-operable disclosure with summary “Layers”, expanded on panel mount. Its descendants are hidden when collapsed.
- [ ] Preserve the user-selected open/closed state through layer selection and ordinary accepted-projection rerenders. A fresh panel mount defaults open, matching the React mount behavior.
- [ ] Keep the current layer list, Add layer, Layer name, permitted Remove layer action, layer-operation feedback, and precedence/transparency help text inside the disclosure without changing their existing callbacks or saved-data behavior.
- [ ] For Base, omit both the Remove layer action and the Dioxus-only “The first layer cannot be removed.” paragraph. Preserve the existing non-Base removal behavior.
- [ ] Match the owned Layers-section spacing/placement against the pinned React Inspector using the same layered fixture. Do not absorb shared context/view toolbar, Objects panel, or footer differences.
- [ ] Toggling the disclosure does not change accepted project contents, revision, or undo/redo history.
- [ ] Add production-component regression coverage for default-open, collapse/reopen, rerender retention, accessible expanded state, Base/non-Base content, and no document/history mutation.
- [ ] Retain paired browser evidence and RF handoff. This child does not close F6K.1, F3.1, F3.3, F6K.2, or any parent acceptance join.
