# 19: Create and select a board from contextual Objects

**What to build:** Restore New board beside the board picker, create one saved empty board with reference defaults, select it, and retain Undo/Redo/reopen behavior through existing Session history.

**Blocked by:** None (existing accepted edit/history/navigation capabilities are proven; full parent completion is not a start gate).

**Status:** implementation-in-progress

- [ ] New board appears beside the board picker in the reference non-Case design workspaces.
- [ ] One click adds Board N with thickness1.6, empty parts/nets and a generated4mm envelope, then selects it after successful save.
- [ ] Undo/Redo/reopen preserve the edit and existing boards/components.
- [ ] Captured stale owner or failed operation cannot navigate another board/project or report success.
- [ ] One combined affected check, changed paired browser receipt and consolidated candidate review; unchanged evidence reused.

Spec: [Board and object creation](../drafts/F5.1c-board-and-object-creation.md). F5.1/F2/F3 parent criteria stay open. No new refactoring takeaway beyond RF-001's existing contextual composition issue is observed in this bounded addition; Session remains the only document/history owner.
