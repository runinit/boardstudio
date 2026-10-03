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

## Current Objects composition correction — 2026-10-03

Matched Sofle archive SHA `5b17071a…0776df` on React5173 and Dioxus34767, 1280×940/light, shows the Dioxus board tree displaced by about159px: physical-instance and grouping controls plus a duplicate project/count heading appear inline. React uses Add object, Board selector and the board tree; Group objects lives in Objects options. The exact paired screenshots and F3.1 accounting are in `../dioxus-layout-authoring/evidence/f31-canonical-criteria-reconciliation-20261003.md`.

Move the existing grouping preference into the existing panel options menu, retain its storage key/callback and keep one private panel-owned preference shared with the tree. Remove physical-instance selection from canonical Layout/PCB/Keymap/Keycaps Objects; physical Case assemblies retain their contextual selector. Remove the duplicate project/count heading. Match the options icon and preserve the menu on compact panels, with close/Escape returning focus to their existing toggle. No document/schema/history/provider change.

Use one combined affected compile/check and one changed paired pane/menu journey: tree placement, Rows/Columns switching and persistence without document revision, menu Escape focus, and compact reachability/close. Reuse existing selection/history and scope evidence. Ordinary composition needs no new UI tests. T1-12 owns the separate rectangular range behavior. No parent closes from this source change alone. Record the shared pane ownership takeaway under RF-001.
