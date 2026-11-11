# 007: Six-workspace responsive, keyboard and visual acceptance

Status: ready-for-human
Type: build
Blocked by: [003](003-inspect-popout.md), [004](004-objects-popover.md), [005](005-retain-objects-capabilities.md), [006](006-topbar-simplification.md), [010](010-pcb-workspace-adapter.md), [011](011-keymap-workspace-adapter.md), [012](012-keycaps-workspace-adapter.md), [013](013-case-workspace-adapter.md), [014](014-parts-workspace-adapter.md), [015](015-case-preview-owner-contract.md)
Parent: [map.md](../map.md)
Owner: coordinator for cross-workspace acceptance and shared CSS/focus corrections

## Boundary

Validate the integrated shared shell against [architecture literals](../architecture.md#responsive-and-keyboard-contract-proposed)
and [inventory](../inventory.md), not just Layout. Common host mechanics get the full width
matrix; every adapter gets one faithful wiring/workflow pass, with extra cases only for
independent failure stories. Use agent-browser for real-page keyboard/touch/visual checks.
No generic accessibility scanner pass substitutes for focus, targets or contrast measurements.
M1 must already meet these criteria for Layout plus global-header/mixed-shell navigation;
that milestone runs its own stable integrated gate and hands-on stop. This ticket remains
open for the later five adapters and is not a reason to delay the first runnable Layout
candidate. Reuse M1's unchanged proofs with receipts rather than repeat the whole battery.

## Acceptance criteria

- [ ] 320/390/728/desktop, folded and every Layout section open: exactly 42px, all seven entry
      points, action scrolling/focus reveal, no page overflow or overlap. Check short viewport,
      on-screen-keyboard clamping, long names and 200% zoom.
- [ ] Coarse-pointer strip targets ≥32×32 with ≥4px spacing; other touch controls retain 44px
      floor. Any unmet mandatory accessibility criterion blocks, not a follow-up waiver.
- [ ] Keyboard walkthrough: topbar → row → both panels → canvas → footer, roving arrows/Home/End,
      dismissal/Escape priority, outside pointer focus, no hidden focus or unintended edit.
- [ ] Both Themes × light/dark, fresh text/icon/focus/SVG-chrome measurements: normal text ≥4.5:1,
      qualifying large text ≥3:1, relevant nontext UI/state/focus pairs ≥3:1. Record disabled/
      decorative exceptions explicitly; prototype's incomplete SVG scan is not evidence.
- [ ] Fresh packaged screenshots compared to frozen references, differences explained for real
      content/workspaces; no frozen reference edited and no claim of CAD/fabrication certification.

## Proof map

- Width/zoom — Layout each section plus one pass per other adapter at representative narrow
  width → open/scroll/keyboard-focus utilities/actions → literal height/reachability, no
  page overflow; panels inside visual viewport without changing camera/bounds · mounted · planned
- Focus/touch — keyboard-only and coarse-pointer pass, panel/menu/pick states nested → operate
  all retained entries and dismiss in order → one Escape owner, correct focus return, no
  inert descendant focus, ≥32/4 strip targets and 44px elsewhere · mounted · planned
- Rendered contrast — Original/shadcn × light/dark, real Theme stylesheet and actual toolbar/
  panel/SVG-role content mounted → measure composited foreground/background/focus pairs →
  specified text/UI ratios met, including SVG-chrome pairs · mounted · planned
- Packaged visual parity — fresh real page in both Themes/Appearances → capture and compare
  geometry/chrome/background pairs with measured mounted states and frozen direction →
  same actions/states, no additional unmeasured compositing or stale assets; record actual
  page measurements where backgrounds differ · packaged · planned
- Adapter integration — each workspace plus project/Export and Add/browse return routes →
  pick, explicitly inspect, edit smallest real field/action, Undo where applicable → named
  inventory destinations and no draft/owner/route loss · packaged · planned
- Reduced motion — reduce enabled before page entry then rapid switches → open/close row and
  panels → zero running workbench animations, no opacity effects or late label reveal · packaged · planned

## Checks and mutations

[Verification](../verification.md): coordinator runs `python3 scripts/check.py browser`
(fresh packaged assets + drag smoke) and records agent-browser/accessibility measurements.
Affected lint/checks precede it. Contrast proof-gap mutation must flag the named bad pair;
other mechanics reuse owning slices' RED/mutation evidence, not duplicate batteries.
Unresolved mandatory gaps keep 008 blocked. One coordinator runs the full stable M1 gate
before its hands-on stop and the final 008 gate after retirement; not one competing build
per delegate. M1 does not close this all-workspace ticket.
