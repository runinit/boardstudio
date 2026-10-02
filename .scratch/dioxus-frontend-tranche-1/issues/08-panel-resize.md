# 08: Resize desktop panels with pointer and keyboard

**What to build:** Users can resize Objects and Inspect while keeping the canvas usable and retaining the chosen width.

**Blocked by:** 07: Pin, auto-hide and collapse desktop panels.

**Status:** ready-for-agent

- [ ] Match Objects bounds of 200–420 CSS px and Inspect bounds of 280–480 CSS px; match the reference 20 px ArrowLeft/ArrowRight steps with side-correct direction and accessible separator values.
- [ ] Respect the other visible pinned panel and reference minimum canvas width when constraining resize; verify both sides, extreme widths and viewport changes.
- [ ] Use one pointer owner with capture and deterministic cleanup on release/cancel/lost capture/unmount. Idle auto-hide cannot interrupt active resize. Match reference cancellation width behavior rather than inventing rollback.
- [ ] Persist resulting widths using existing panel preferences; pointer/keyboard resize changes neither document/history nor camera. Verify focus, capture cancellation and the final displayed width in the integrated browser.

- [ ] Complete the tranche’s shared acceptance and independent review; record refactoring takeaways or explicitly record none observed.
