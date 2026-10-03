# F6K.1d: Collapse the active Keymap editor section

**Parent:** F6K.1 — Keymap projection, layers, selection, and active-layer 2D view.

**Source specification:** [`11-keymap-editor-disclosures-spec.md`](../drafts/11-keymap-editor-disclosures-spec.md).

**What to build:** The active Selected key, Macros, or Encoders panel content appears inside one expanded-by-default native Inspector disclosure. Designers can collapse or reopen it without changing the selected editor, keymap data, or history.

**Capability gate:** The existing Keymap panel already mounts the Keys, Macros, and Encoders tabs and their current private editor content. This child changes only the presentation wrapper around those callable panes; no new domain capability is required to start.

**Blocked by:** None for this presentation-only child. F6K.1, F3.1, and the existing F6K.2/3/4 parent acceptance joins remain unchanged.

**Acceptance:**

- [ ] Keep the Keys, Macros, and Encoders editor tabs visible and unchanged. Wrap only the active pane in a native, keyboard-operable disclosure that starts expanded whenever that pane mounts.
- [ ] Match the pinned React summary labels: “Select a key” or `<reference> · <layer>` for Keys, “Macros” for Macros, and `Encoders · <layer>` for Encoders. Keep the summary current as selection/layer changes arrive; a new accepted key/scope owner remounts expanded, while layer-only changes preserve a deliberate collapsed state.
- [ ] Collapse/reopen hides and restores only the active pane content. While a pane remains mounted under the same accepted selection owner, its explicit open/closed choice survives accepted-snapshot, active-layer, and feedback rerenders. React keys the Inspector by accepted key/scope owner, so changing selected key or scope remounts Keys expanded. A pane remounted after switching tabs also starts expanded.
- [ ] Keep all current editor controls, values, callbacks, nested encoder direction/push disclosures, and edit/history behavior inside their existing owners. Do not change layer/tab selection or reset drafts solely because the outer disclosure was toggled.
- [ ] Match React's native summary/focus/marker spacing and avoid duplicate inner “Macros” or “Encoders” headings where the outer summary supplies that title.
- [ ] Verify the paired Keys/Macros/Encoders collapse/reopen journey on the pinned React and current Dioxus candidate; verify layer title refresh while Keys is collapsed under the same selection owner and expanded reset after changing selected key/scope, with no document revision or Undo/Redo change from toggling.
- [ ] Keep the F6K.1 parent, F3.1 join, F6K.2/3/4 criteria, and broader F6 acceptance open. This child does not claim editor behavior or full Keymap parity.

**Ownership:** Private Keymap panel composition and scoped Keymap presentation styles. Do not change shared Runtime/Core composition or existing editor callback ownership.

**Status:** source implementation is frozen in `7448286adf2b2adb087503b09e9d9e3517c4a77c`. The first paired public click journey on candidate `ab532f275cb14385db3e4e52e5adcb5fb7c3ee18` found that static `open: true` reopened each section after a click. Candidate `9c16db41155a7b72be187789d225f78c2fbfe9af` confirmed per-pane toggles and tab remount defaults, then exposed that React reopens Keys when the accepted selection owner changes. The source now resets Keys disclosure open on a key/scope owner transition while preserving layer-only state. Formatting and diff checks pass; the correction awaits packaging and the final paired replay under the reduced-test instruction. Evidence: `../evidence/keymap-editor-disclosures-20261003/paired-browser-journey.md`.

**Refactoring handoff:** Update RF-009 follow-up evidence or record no new architectural finding. Preserve the existing F6K.1/F6K.2/3/4 acceptance boundaries.
