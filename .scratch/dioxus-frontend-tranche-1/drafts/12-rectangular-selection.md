# 12: Correct rectangular range selection and modifiers

**What to build:** Shift selects the reference rectangular key range, while Ctrl/Cmd toggles and tree/canvas selection remain consistent across scope changes.

**Blocked by:** 10: Browse and select the Layout object hierarchy.

**Status:** implementation in progress — focused Session semantics are implemented; parent acceptance/review remains open.

- [x] First reproduce the current mismatch: a matrix rectangle whose correct members differ from the flattened linear interval. Preserve the expected-red result in the ticket evidence before applying the repair.
- [ ] Match the reference matrix/row/column anchor, same-matrix eligibility, rectangular live-member filtering and fallback behavior. Repeated Shift selections preserve the reference anchor; matrix/board changes cannot reuse an invalid anchor.
- [ ] Match Ctrl/Cmd toggling for keys/components, ordinary selection-mode behavior and synchronized tree highlighting. Modifier selection must not begin a drag or mutate geometry/history.
- [ ] Use the generic Session part-selection event for existing callers and a typed matrix-cell event through the private adapter where the clicked primary key must be distinguished from its projected Matrix/Row/Column extent. Never treat an empty-cell ID or companion as a range member.
- [ ] Verify disabled/empty cells, companions, cross-board/cross-matrix actions, stationary clicks and selection after canceled gestures in both desktop/compact views. Preserve F3a drag/Undo, panel and camera behavior.
- [ ] Astra High owns the evidenced behavioral repair; a different Astra reviewer covers the repaired source and evidence. The separate F3.3 small-drag threshold mismatch remains outside this ticket.

- [ ] Complete the tranche’s shared acceptance and independent review; record refactoring takeaways or explicitly record none observed.

**Focused progress (2026-10-03):** The original Session `SelectParts/Range` path reproduced the flattened-order mismatch with a 3×3 matrix fixture containing a disabled center and a diode companion. The typed matrix-cell path now validates the accepted scope and active board, derives the rectangle from live primary scene cells, keeps the clicked primary cell as the anchor even when the projected extent starts elsewhere, and preserves that anchor on repeated Shift selection. Successful Shift selection changes the selected tree context to the target Key; invalid anchors fall back to the current projected scope. One focused application regression passes. The mounted key hit routes Matrix/Row/Column projections through that event; existing `SelectParts` callers remain compatible. The full modifier/component/tree/desktop-compact acceptance and independent review remain open; this does not promote the tranche or claim overall F3.4 closure. See [receipt](../evidence/t12-range-selection-20261003/receipt.md).
