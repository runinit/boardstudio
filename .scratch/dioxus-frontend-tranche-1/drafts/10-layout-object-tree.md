# 10: Browse and select the Layout object hierarchy

**What to build:** Objects shows the active canonical board’s matrices, row/column groups, keys, cell components and standalone components with synchronized canvas selection.

**Blocked by:** 01: Isolate the existing workspace UI for parallel work.

**Status:** draft — awaiting breakdown approval

- [ ] Use stable IDs, reference labels/counts, row/column grouping and independent disclosure controls. Preserve linked-half/matrix grouping and existing five-layer visibility behavior.
- [ ] Select board/matrix/row/column/key/component contexts through one private adapter while selected real part IDs remain in the existing Session. Empty key slots can carry semantic context but never become fabricated document part IDs.
- [ ] Keep tree and canvas single-selection synchronized, update the actual selected context and filter to current-board, enabled/live members. Selection/disclosure does not change document revision/history.
- [ ] On board/project change, cancel active scoped gestures and discard invalid context/selection/anchor before exposing the new board. No late callback can select an object in a newer scope; baseline scope safety ships in this ticket.
- [ ] Verify populated and empty cells, cell components, standalone parts and a split-board fixture through desktop/compact public keyboard and pointer actions. Preserve working canvas edit/Undo and Layers/Footprints behavior.
- [ ] Prove the private semantic-context/session-ID mapping before implementation. New matrix/part creation, full inspector forms and outline authoring stay in their separate existing tasks.

- [ ] Complete the tranche’s shared acceptance and independent review; record refactoring takeaways or explicitly record none observed.
