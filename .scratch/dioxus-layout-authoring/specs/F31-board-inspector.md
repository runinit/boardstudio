# Layout board Inspector parity

Selecting the active board in Objects, or clearing the part selection in Layout, must show the pinned TypeScript board Inspector: the board heading and context, selection guidance, editable Board name, Outline status and Placed parts count. Candidate34761 shows only the board heading after selecting Main board. [Actual paired observation](../evidence/board-inspector-frontier-20261003/reference.json) pins the source and journey.

Board name commits on blur/Enter, trims whitespace and restores the accepted name for an empty or unchanged draft. Escape restores the accepted name. One accepted rename creates one edit/history step and updates the board selector, Objects row and Inspector. Undo/Redo and reopen retain the appropriate accepted name. Drafts and pending callbacks belong to the captured document/session/board/context; leaving that owner or a competing accepted revision cannot rename another board. Reuse Session edits and the accepted snapshot; do not add a second document store or a public Core API.

Mount a private projection and action in the Layout Inspector composition. Preserve component, matrix and outline Inspector precedence, and exclude other workbenches. Hide the generic Position form in the board context. Reuse accepted contour/readiness inputs for the reference Outline status and accepted visible-part count. Routed PCB reference authoring is a separate workflow; record its missing controls without a placeholder completion claim.

After the current PCB/empty-board packet qualifies, implement this next Layout-first child. Use one combined affected check and the changed paired rename/invalid/Escape/Undo/Redo/reopen journey; no new tests for this reversible UI port. One consolidated candidate review closes the child. F3.1/F3.7 and all other parent criteria remain intact.

RF-001 already records root composition and missing contextual projection coverage. This child mitigates a concrete missing board context; the broader projection/state-lifetime refactor remains deferred. Rollback removes this private leaf and mounts while preserving existing board operations and source evidence.
