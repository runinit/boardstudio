# 07: Pin, auto-hide and collapse desktop panels

**What to build:** Objects and Inspect support the reference desktop panel modes and remember their preferences.

**Blocked by:** 01: Isolate the existing workspace UI for parallel work.

**Status:** ready-for-agent

- [ ] Implement both panels with the reference default pinned layout and pin/auto-hide/collapse controls, rail reveal behavior, independent panel state and reference menu dismissal/focus return.
- [ ] Honor the reference idle-hide behavior, including pointer hover and focus within the panel/rail. Hidden content is excluded from focus and accessibility exposure; opening panels preserves the canvas view. Preserve this policy when resize is added, with active-resize suppression verified in ticket 08; ticket 07 does not depend on completing resize.
- [ ] Read/write the existing browser preference meanings for mode and width. Clamp valid widths; malformed or unavailable optional storage falls back to usable in-memory defaults without inventing a warning workflow. Do not persist transient reveal/menu/focus state.
- [ ] Verify state after reload and across workspace switches, both panels together and long content. Mode/visibility actions leave document revision, history and camera unchanged.

- [ ] Complete the tranche’s shared acceptance and independent review; record refactoring takeaways or explicitly record none observed.
