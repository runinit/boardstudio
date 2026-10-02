# 01: Edit board-level keycap defaults

**Parent:** F6C.2 — Board, matrix, and per-key keycap controls.

**What to build:** The Keycaps Inspector lets a user edit the active board's default keycap color, legend color, and minimum clearance, with the same grouping, placement, labels, ranges, feedback, and edit behavior as TypeScript. Changes update the physical view and fit result, and participate in normal history and project persistence.

**Blocked by:** F6C.1's board-scoped Keycaps projection and the existing accepted-document edit/history path. This ticket does not wait for F6.2's unrelated acceptance joins; F6C.2 and all F6 parent joins remain open.

**Status:** ready-for-agent

- [ ] The three board controls appear in the contextual Keycaps Inspector and match TypeScript placement, names, input type, supported range/step, and interaction/commit boundary.
- [ ] Every edit targets the currently active board and uses the existing board-keycap edit operation; switching boards never leaks defaults between them.
- [ ] Reuse the Editor-owned scoped edit admission/outcome lifecycle: capture and revalidate accepted token, revision, full scope and active-board membership; settle exact outcomes before suppressing hidden Keycaps UI; add no panel-local edit owner.
- [ ] Browser evidence maps React color/number `onChange` behavior to Dioxus native events by exercising typing, spinner increments and color selection, and records accepted revision/history timing instead of assuming the handler names are equivalent.
- [ ] Changing board keycap color updates keys without explicit color; an explicit per-key color remains effective.
- [ ] Changing legend color updates the board-level setting without changing key legends or bindings.
- [ ] Changing clearance updates fit resolution for the active source and presents pending, accepted, and failure states without replacing newer accepted results.
- [ ] Undo, Redo, save, reload/reopen, and board switching preserve TypeScript-equivalent values and visuals on the same fixture.
- [ ] Empty/unavailable board states are explicit and do not manufacture persisted defaults.
- [ ] Paired TypeScript/Dioxus browser evidence records source build, fixture hash and project/board IDs, control placement, edit result, Undo/Redo, and reopen.
- [ ] Required independent Standards/Spec review and RF handoff are recorded without changing parent task status or shared graph.
