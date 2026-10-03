# F3.5b: Layout findings footer and live navigation

**Parent:** F3.5 — Inspector, relationships and findings in [Layout ticket 03](../../dioxus-frontend-v1/issues/03-layout.md).

Spec: [F3.5b Layout findings navigation](../drafts/17-layout-findings-navigation.md). Owner: Layout author; the private footer/Inspector mount remains in this isolated branch and root serially integrates the shared composition.

**Status:** implementation in isolated worktree from `dd71c27ba1f54fd0cfdfebc3b7f56c4c89304a5a`; paired changed journey and combined affected check remain pending integrated candidate.

**Scope note:** The pinned shared React footer also exposes `Layout findings` in PCB and Case contexts. This child qualifies the Layout route only; other workspace finding navigation stays with its consumers and must not be presented as complete or routed through the Layout owner.

- [ ] Add the real Layout findings count/action to the shared canvas footer in both Layout 2D and 3D, derived from the current accepted scene and document.
- [ ] Add the contextual `Layout findings` Inspector page with React grouping/severity/messages/action labels, Back/Escape, opening-heading focus and footer-trigger focus restoration.
- [ ] Reuse existing finding target/group/bounds presentation helpers without making Layout depend on `KeycapsFitSource` or Keycaps fit lifecycle.
- [ ] Route same-board Outline/Part/Matrix/Board actions through existing selection/tree and fit callbacks, with exact current Layout owner validation and focus.
- [ ] Retain cross-board requests through the existing board-navigation callback; resume only for the same accepted document/session/token/revision and expected board.
- [ ] Missing, unsupported and stale targets remain readable without an action; other workspace/Case/mechanical findings remain owned by their current consumers.
- [ ] Keep F3.5 and all 62 canonical parent statuses/acceptance criteria unchanged. Preserve broader contextual Inspector, relation, numeric draft, history and compact/keyboard gates.
- [ ] One changed paired Layout-findings browser journey on the combined candidate; root owns the combined affected compile/check. Reuse unchanged proof and do not run a new local suite for reversible UI.
- [ ] Preserve RF-001–015 and record a scoped refactoring takeaway or “No new refactoring takeaway observed.”

**Existing parent acceptance:** All F3.5 criteria remain open, including broader empty/single/multiple selection Inspector behavior, drafts, relation state, target navigation, focus/keyboard/compact behavior and paired history journey. This child adds no canonical task, dependency or parent acceptance edge.
