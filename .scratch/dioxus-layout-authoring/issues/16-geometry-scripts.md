# 16: Port the Layout Geometry scripts editor

**Status:** implementation in isolated worktree

**Parent:** F3.8 Geometry script editor in [Layout ticket 03](../../dioxus-frontend-v1/issues/03-layout.md). Does not accept F3.5/F3.7 or close remaining F3.4 criteria.

Spec: [F3.8 Geometry scripts](../specs/F38-geometry-scripts.md). Owner: Layout author; root serially integrates the private mount callback and runs the combined affected page check.

- [x] Pin current React editor, routed entry, script draft defaults and existing Core-backed e2e oracle.
- [x] Add the Add object → Board geometry → Geometry scripts… route, returning from PCB to Layout and opening the Inspector.
- [x] Add accepted New script, active-script selection, Name/source/enabled drafts, nonblank Apply, Core findings and rejected-apply feedback through existing ReplaceDocument history.
- [ ] Verify the one changed public Dioxus journey on the next integrated candidate, including script output and Undo/Redo or save/reopen; retain exact source/provenance and screenshot.
- [ ] Combined affected WASM check and consolidated review remain with root.
- [x] Preserve RF-001/RF-006/RF-009; no new refactoring takeaway observed.

Do not add delete, live-run, new Rhai features, or a second script/session store. Core remains the only script execution and geometry owner.
