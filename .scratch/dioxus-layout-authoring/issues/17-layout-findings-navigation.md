# F3.5b: Layout findings footer and live navigation

**Parent:** F3.5 — Inspector, relationships and findings in [Layout ticket 03](../../dioxus-frontend-v1/issues/03-layout.md).

Spec: [F3.5b Layout findings navigation](../drafts/17-layout-findings-navigation.md). Owner: Layout author; the private footer/Inspector mount is integrated as a child implementation. Root serially joins shared composition; parent acceptance remains open.

**Status:** source is integrated in candidate `17ba32b984c13f3621a2145efe1dd478862ab5cd`; combined strict check passed. The 34765 paired route journey passed for the available same-board Outline finding. A source-backed footer icon/label alignment correction is queued; verify its rendered appearance on the next candidate.

**Scope note:** The pinned React footer exposes the same `Layout findings` entry across its non-Export workspaces. This child shares the accepted scene page and preserves the selected workspace when opening it; activating a supported target uses the existing Layout route. Unsupported Case-body/mechanical targets remain without an action, and this does not claim broader workspace-specific finding navigation is complete.

- [x] Add the real Layout findings count/action to the shared canvas footer in all enabled non-Export workspaces, derived from the current accepted scene and document; opening preserves the selected workspace.
- [x] Add the contextual `Layout findings` Inspector page with React grouping/severity/messages/action labels, Back/Escape, opening-heading focus and footer-trigger focus restoration.
- [x] Reuse existing finding target/group/bounds presentation helpers without making Layout depend on `KeycapsFitSource` or Keycaps fit lifecycle.
- [x] Route same-board Outline/Part/Matrix/Board actions through existing selection/tree and fit callbacks, with exact current Layout owner validation and focus.
- [x] Retain cross-board requests through the existing board-navigation callback; resume only for the same accepted document/session/token/revision and expected board.
- [x] Missing, unsupported and stale targets remain readable without an action; unsupported body/mechanical targets remain readable without being routed through the Layout owner.
- [ ] Keep F3.5 and all 62 canonical parent statuses/acceptance criteria unchanged. Preserve broader contextual Inspector, relation, numeric draft, history and compact/keyboard gates.
- [x] On candidate 34767, verify the footer icon and label render inline and that opening the shared entry in Keymap preserves the Keymap tab until the same-board Outline action routes to Layout. Retain [the changed journey receipt](../evidence/17-layout-findings-navigation/public-34767.md) and the earlier [34765 Layout route receipt](../evidence/17-layout-findings-navigation/public-34765.md). The fixture exposes no cross-board action; no broader route is claimed.
- [x] Preserve RF-001–015; no new refactoring finding was observed. Root owns canonical RF ledger incorporation.

**Existing parent acceptance:** All F3.5 criteria remain open, including broader empty/single/multiple selection Inspector behavior, drafts, relation state, target navigation, focus/keyboard/compact behavior and paired history journey. This child adds no canonical task, dependency or parent acceptance edge.
