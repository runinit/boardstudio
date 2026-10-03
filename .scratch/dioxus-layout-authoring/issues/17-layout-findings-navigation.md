# F3.5b: Layout findings footer and live navigation

**Parent:** F3.5 — Inspector, relationships and findings in [Layout ticket 03](../../dioxus-frontend-v1/issues/03-layout.md).

Spec: [F3.5b Layout findings navigation](../drafts/17-layout-findings-navigation.md). Owner: Layout author; the private footer/Inspector mount is integrated as a child implementation. Root serially joins shared composition; parent acceptance remains open.

**Status:** source is integrated in candidate `17ba32b984c13f3621a2145efe1dd478862ab5cd`; combined strict check passed. The 34765 paired route journey passed for the available same-board Outline finding. A source-backed footer icon/label alignment correction is queued; verify its rendered appearance on the next candidate.

**Scope note:** The pinned shared React footer also exposes `Layout findings` in PCB and Case contexts. This child qualifies the Layout route only; other workspace finding navigation stays with its consumers and must not be presented as complete or routed through the Layout owner.

- [x] Add the real Layout findings count/action to the shared canvas footer in both Layout 2D and 3D, derived from the current accepted scene and document.
- [x] Add the contextual `Layout findings` Inspector page with React grouping/severity/messages/action labels, Back/Escape, opening-heading focus and footer-trigger focus restoration.
- [x] Reuse existing finding target/group/bounds presentation helpers without making Layout depend on `KeycapsFitSource` or Keycaps fit lifecycle.
- [x] Route same-board Outline/Part/Matrix/Board actions through existing selection/tree and fit callbacks, with exact current Layout owner validation and focus.
- [x] Retain cross-board requests through the existing board-navigation callback; resume only for the same accepted document/session/token/revision and expected board.
- [x] Missing, unsupported and stale targets remain readable without an action; other workspace/Case/mechanical findings remain owned by their current consumers.
- [ ] Keep F3.5 and all 62 canonical parent statuses/acceptance criteria unchanged. Preserve broader contextual Inspector, relation, numeric draft, history and compact/keyboard gates.
- [ ] Verify the footer icon and label render inline on the CSS follow-up candidate. The route journey is retained in [public 34765 receipt](../evidence/17-layout-findings-navigation/public-34765.md); the available fixture has one same-board Outline finding and exposes no cross-board action. Root owns combined checks; do not run a new local suite for reversible UI.
- [ ] Preserve RF-001–015 and record a scoped refactoring takeaway or “No new refactoring takeaway observed.”

**Existing parent acceptance:** All F3.5 criteria remain open, including broader empty/single/multiple selection Inspector behavior, drafts, relation state, target navigation, focus/keyboard/compact behavior and paired history journey. This child adds no canonical task, dependency or parent acceptance edge.
