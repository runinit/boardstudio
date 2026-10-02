# 09: Use compact Objects and Inspect drawers

**What to build:** At compact widths, users can open, interact with and dismiss Objects and Inspect without losing keyboard focus.

**Blocked by:** 07: Pin, auto-hide and collapse desktop panels.

**Status:** ready-for-agent

- [ ] Match the side-specific compact breakpoints: Objects at 980 CSS px and Inspect at 820 CSS px, including the intermediate layout with only one compact side.
- [ ] Implement working toggles, scrim, close and Escape behavior, reference initial focus and opener focus return; closed content is inert and excluded from keyboard order.
- [ ] Cross breakpoints while a panel is open or focused; preserve desktop mode/width preferences without persisting drawer-open state or stranding focus/capture. Dismissal must not trigger unrelated canvas edits.
- [ ] Verify both panels, desktop-to-compact transitions and keyboard traversal using current Objects/Inspect content. Document revision/history/camera remain unchanged; new Layout tree content is checked again when integrated.

- [ ] Complete the tranche’s shared acceptance and independent review; record refactoring takeaways or explicitly record none observed.
