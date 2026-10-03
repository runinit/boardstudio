# 08: Show Keycaps assignment status in the contextual Inspector

**Parent:** F6C.2 — Board, matrix, and per-key keycap controls.

**What to build:** The Keycaps Inspector shows the TypeScript-equivalent `Keycaps` heading and assigned/total physical key count, driven by the active board's accepted Keycaps projection.

**Blocked by:** None for implementation; the F6.1 accepted board/key projection and existing keybinding edit/history path are already available. F6C.2 remains a parent acceptance join.

**Specification:** [Keycaps Inspector summary](../keycaps-inspector-summary-spec.md).

**Status:** implemented-candidate-pending-paired-browser-acceptance

- [ ] Show `Keycaps` and `<assigned>/<total> assigned` at the top of the Inspector when an accepted Keycaps projection is available.
- [ ] Reuse the accepted projection count and match TypeScript semantics: every projected key whose effective binding is not `&none` counts as assigned, including transparent bindings; total equals projected physical keys.
- [ ] Keep the count scoped to the active board; board changes and accepted binding changes update it without selection-dependent or panel-local counter state.
- [ ] A mounted workbench/Inspector regression observes the displayed count after accepted update, Undo and Redo; a pure formatting helper test is not sufficient.
- [ ] Paired same-fixture React/Dioxus browser journey verifies the initial count, an assignment edit, Undo, Redo, board switch, save/reopen, and transparent/unassigned counting behavior with source/build/fixture provenance.
- [ ] Preserve existing Keymap edit ownership, Keycaps settings, empty state, all 62 parent criteria and RF-001; this ticket closes no parent join.
