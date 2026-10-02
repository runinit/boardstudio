# 12: Correct rectangular range selection and modifiers

**What to build:** Shift selects the reference rectangular key range, while Ctrl/Cmd toggles and tree/canvas selection remain consistent across scope changes.

**Blocked by:** 10: Browse and select the Layout object hierarchy.

**Status:** draft — awaiting breakdown approval

- [ ] First reproduce the current mismatch: a matrix rectangle whose correct members differ from the flattened linear interval. Preserve a failing regression for that expected reason before applying the repair.
- [ ] Match the reference matrix/row/column anchor, same-matrix eligibility, rectangular live-member filtering and fallback behavior. Repeated Shift selections preserve the reference anchor; matrix/board changes cannot reuse an invalid anchor.
- [ ] Match Ctrl/Cmd toggling for keys/components, ordinary selection-mode behavior and synchronized tree highlighting. Modifier selection must not begin a drag or mutate geometry/history.
- [ ] Use existing Session part-selection events through a proven private adapter; do not turn a synthetic empty-cell ID into a document part or silently change the public Session API.
- [ ] Verify disabled/empty cells, companions, cross-board/cross-matrix actions, stationary clicks and selection after canceled gestures in both desktop/compact views. Preserve F3a drag/Undo, panel and camera behavior.
- [ ] Astra High owns the evidenced behavioral repair; a different Astra reviewer covers the repaired source and evidence. The separate F3.3 small-drag threshold mismatch remains outside this ticket.

- [ ] Complete the tranche’s shared acceptance and independent review; record refactoring takeaways or explicitly record none observed.
