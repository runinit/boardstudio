# F3.2c: Add a library component to the board or selected key

**Parent:** F3.2 — Matrix and component authoring.

**What to build:** In Layout, a designer can find an available library component and add it in the current context: to the active board or named layout as a positioned standalone part, or to the selected matrix key as its supported switch assembly/attached component. Each route uses the existing editor behavior, validation and history.

**Blocked by:** Canonical F3.2 start gate F3.1 (Layout tree, board scope and selection), as recorded in the 62-task graph. Existing definitions are sufficient for fixture-backed UI work; this ticket does not wait on completion of the Parts milestone.

**Status:** Published; implementation starts after canonical F3.1 is satisfied. No sibling-ticket dependency is added.

- [ ] Add object → Parts shows the reference available-definition groups and search results from the accepted library/catalogue input. When standalone placement is selected, expose Board/ungrouped or a named layout on the active board. Empty and no-match states explain what can be done.
- [ ] For standalone placement, selecting a definition starts a visible pending part using the current view-center convention. Preserve the existing reversible-Ergogen normalization for the active construction. Pointer placement uses the established snap/Alt behavior; arrow keys move the preview by the existing nudge/snap increment; Enter or pointer placement commits and Escape cancels. Changing board, leaving the workspace, or invalidating the target cancels stale placement safely. Commit preserves definition, part, board, outline-envelope and optional layout membership, selects the new stable identity and reveals its inspector.
- [ ] Preserve the two distinct reference entry paths. Add object → Parts always begins standalone canvas placement, even if a matrix key is selected. In the Parts workspace, choosing an applicable definition with a selected matrix-key scope instead follows the capability rule: a switch/input definition sets the key assembly and another eligible definition appends an attached assembly. Do not route that action through standalone placement or invent a key/component identity. Preserve linked-half/local override semantics. This child owns both insertion paths; the separate matrix/cell inspector child owns later inspection, replacement and removal of attached members.
- [ ] Rejected operations retain accepted document/history and show a correctable error. Verify available/search/no-match states, board/layout standalone placement, selected-key assembly and component insertion, pointer/keyboard/Alt placement, cancel, stale board/workspace transitions, identity/membership, Undo/Redo and saved archive reopen on a multi-board/layout fixture. Pair the same actions with React; cover compact/focus and both themes.
- [ ] Keep existing-part transforms in F3.3 and keycap sizing/reflow in F6C.3. Reuse current core/session edit and validation behavior—no new Part types, public API/schema/member visibility, ID-generation authority, or second mutable document store.
- [ ] Feature ownership is a private component-placement UI module. Root owns current accepted definition projection, ID allocation, board/layout membership edit callback, selection navigation, page/canvas mount and global CSS. A helper-only implementation does not complete this ticket; verify the integrated public route.
- [ ] Record evidence and update an existing RF only if the implementation demonstrates a distinct quality issue, otherwise state “No new refactoring takeaway observed.”

**Parent acceptance:** F3.2 remains open for its full matrix and component criteria. This ticket adds no dependency beyond the canonical F3.1 start edge and does not change F3.4/F3.7 or Parts acceptance joins.

**Suggested routing:** Luna High author for async catalogue/scope behavior; separate Luna verifier; Astra independent Spec/Standards review.
