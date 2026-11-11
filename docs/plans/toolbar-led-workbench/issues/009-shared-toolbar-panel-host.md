# 009: Shared toolbar/panel host and lifetime/focus requests

Status: claimed
Type: build
Blocked by: [001](001-reconcile-design-adr-scope.md)
Parent: [map.md](../map.md)
Owner: coordinator; private `presentation/workbench_shell.rs`, shared composition/routing hunks

## Boundary

Build a private page module (`web/src/presentation/workbench_shell.rs`) at the existing
objects/toolbar/inspector seam, reusing ui-shared controls rather than adding public domain
interfaces. Its mounted tests live in the page crate, not just ui-shared. It owns one bounded
42px row, shared fitting/clip mechanics, anchoring/clamping, Objects/Inspector state and
focus/dismissal/inertness, not copied task visibility, document/workflow state or
universal domain selection. Accept existing workspace content/mounts, keep draft
controllers outside visibility switches and do not remount viewers. Private interface first;
no cross-crate visibility widening without approval. Layout's section/content work is 002;
domain-specific restoration is 003/010–014, not a global untyped draft bag. Preserve the
stable canvas child identity already provided by DockLayout. The two real content shapes
are unchanged Layout and Case `objects`/`inspector` Elements, using existing accepted-model
fixtures and typed inputs; no domain adapter edits are required in this slice. Follow the
[render-only preparation order](../refactoring.md#refactor-first-then-change-placement).

Generic tests use labelled synthetic content through the actual private host interface,
not test-only production methods or fake edit controllers. 009 closes its own mechanics;
002 proves the real Layout consumer before M1 can close. Real registered-draft and task
admission integration belongs to 005, not a dependency back from 009 into later adapters.

Implement the [panel transition/focus/lifetime contract](../architecture.md), remap shell
Show/hide commands, close panels on workspace/session transitions, preserve session-scoped
workspace owners and ensure explicit reveals are requests to one authority. Do not delete
old dock primitives yet; adapters migrate behind existing workspace routing, with no visible
duplicate old/new host for one workspace. Until the later adapters migrate, route commands
by target workspace to its single new or legacy host; never let Layout read old dock settings.
Keep the mixed candidate on this branch, without an automatic main merge. No lasting feature
flag or new narrow Sheet/pin alternative.

## Acceptance criteria

- [ ] Shared host handles Objects/Inspect mutual exclusion, keyboard/pointer dismissal and
      clamping at all sizes, without a backdrop, focus trap, resize/reframe or opacity effect.
- [ ] Show/hide command routes and workspace transitions cannot reopen stale panels/focus.
- [ ] The host reads task/detail visibility from its typed owner and requests hide/park;
      refusal cannot partly open another surface. Domain Close and navigation admission
      remain authoritative, with no mirrored task enum or synthetic Cancel.
- [ ] Fitting has one implementation, its generic measurement/probe contract covered here;
      the actual styled Layout action composition and motion are 002's acceptance.
- [ ] Two different real adapter content shapes fit the seam without translating their IDs
      into a generic Layout TreeContext; no fake test-only production interface.
- [ ] One build/full-suite owner, serialized shared-file edits, and domain ownership retained.

## Proof map

- Exclusion/dismissal — Objects open, Inspect activated, then same utility/Close/Escape/outside
  pointer (equivalent dismissal inputs parameterized) → only requested panel visible, closed
  content inert; focus returns except valid outside pointer keeps its target · mounted · planned
- Transient interface — labelled synthetic content on the production private interface,
  owner visible before a utility request → exercise hide allowed/refused and owner-close
  notifications → one visible surface, refusal leaves both states unchanged, own close
  does not reopen a previous panel; nested picker preserves its parent · mounted · planned
- Navigation admission — same interface with a domain route accepted/refused → request board/
  workspace change → accepted route clears host focus/old callbacks; refusal changes neither
  route nor surface, without a shell-created Cancel · mounted · planned
- Generic fitting — real Theme/toolbar stylesheet and font loaded before labelled fixture →
  change final content/width → one final measurement/allocation before movement, probe inert
  and no hidden focus; 002 adds the real Layout action oracle · mounted · planned
- Canvas invariance — capture rect/camera with each real content shape → open/switch/close →
  identical canvas rect/camera and no viewer remount/accepted revision · mounted · planned
- Roving/clamping — 320px/short visual viewport, section/action lane overflow → arrows/Home/End,
  open panel and resize → all triggers reachable, focused action visible, panel/Close inside
  viewport, no hidden probe focus or page overflow · mounted · planned
- Route lifetime — labelled caller-owned fixture input above the host (not a registered
  domain editor) before route change → switch workspace/return, then replace session →
  input survives the round trip, no panel auto-reopen; replaced session has no old callback/
  focus authority. Real draft/admission lifetime is 003/005 · mounted · planned
- Commands — migrated Layout and unmigrated Case/PCB selected in turn → invoke each retained
  Show/hide command → only that workspace's current host responds, no second authority,
  ghost Layout panel or stale focus; Layout never reads legacy pin/width · mounted · planned

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`,
`run-wasm-tests.py --files web/src/presentation/workbench_shell.rs` for the real private
host; add `check.py browser --only ui-shared` only if copied controls change. Exclusion,
transient-interface/navigation admission, generic fitting, inert/focus and canvas-width
proof-gap mutations are named there. Real registered-owner/task proofs and mutations are 005. Reuse existing panel/popup fixtures where
faithful. One fresh combined Spec+Standards review; no production implementation until 001.

## Implementation progress

Coordinator claimed 009 on `cafaf430`. First private production interface is in
`web/src/presentation/workbench_shell.rs`, declared by `presentation.rs`; no public API
or domain owner changed. The first mounted case checks utility exclusion/toggle, hidden
inert content and stable retained editor DOM through that interface.

- RED: `.evidence/toolbar-m1/009-exclusion-red.log` — Objects did not open after activation
  (the request callback was not implemented), one executed test, one expected failure.
- GREEN: `.evidence/toolbar-m1/009-exclusion-green.log` — one executed test, zero failures,
  after wiring the single `Option<ToolPanel>` request. This is only the first mechanics
  assertion, not closure or real Layout integration evidence.
- Initial runner compilation was blocked by missing generated `web/assets/tailwind.css`
  (`009-initial-red.log`, diagnosed in `009-compile.log`); this is not behavioural RED.
  The repository's `dx build --web --no-default-features --features page --cargo-args=--locked`
  in `web/` passed (`009-test-css-build.log`), then the mounted RED/GREEN ran. Builds were
  serialized under the required build lock, without overriding Cargo settings.

- Dismissal RED: `.evidence/toolbar-m1/009-dismiss-red.log` — two executed tests, one expected
  failure: opening Objects did not focus its search.
- Dismissal GREEN: `.evidence/toolbar-m1/009-dismiss-green.log` — two executed tests, zero
  failures. Objects focuses search; Close and Escape return focus to the utility; outside-
  pointer dismissal leaves focus on its valid target. Native keydown/pointerdown listeners
  are removed before their closures are released.
- Render-only preparation GREEN: `.evidence/toolbar-m1/009-render-only.log` — the focused
  Layout layers, Case mounted-live-scene and workbench-host page suites executed eight tests
  with zero failures after extracting the existing legacy panel wrappers into the private
  composition seam without moving raw content or canvas children. This is not final canvas
  invariance or adoption evidence.

Next: clamping/fitting and target-aware command routes; then the real Layout consumer in 002.
Actual task hide consumers must be inspected before adding any optional refusal variant.
Remaining proof rows, quick/lint, boundary mutations and combined review are not executed.

- Clamping RED: `.evidence/toolbar-m1/009-clamp-red.log` — 4 executed, 2 failed. Named failure
  `panel_clamps_to_layout_viewport_and_synthetic_content_scroll_keeps_close_reachable` (renamed from `panel_clamps_to_visual_viewport_and_keeps_close_reachable`) at `workbench_shell.rs:848:13`:
  `workbench-objects content scrolls vertically, got hidden`. Tall labelled synthetic content
  (60x40px rows) overflowed `.workbench-popout-content` (`overflow:hidden`) and was not
  user-scrollable. Second failure `dismissal_returns_focus...` was shared-WASM panic-hook
  cascade per runner (`re-run after fixing first failure`); no production change yet, only
  additive test.
- Clamping GREEN: `.evidence/toolbar-m1/009-clamp-green.log` — 4 executed, 0 failures, after
  CSS-only fix `web/assets/shell.css`: `.workbench-popout-content { overflow:auto;
  overscroll-behavior:contain }` (was `hidden`). New `clamp_fixture` reuses actual private
  `PanelHost` + existing `Find an object`/`Retained value` labels + production
  `workbench-base.css`/`tailwind.css`/`shell.css`/font; opens Objects, narrows synthetic fixture lane (`#clamp-fixture`) to 320px without changing the browser viewport,
  opens Inspect. Asserts panel+Close within `innerWidth/innerHeight` layout viewport bounds (not `visualViewport`), widths
  320/400px within 16px safe inset, tall content `scrollHeight>clientHeight` + `overflow-y
  auto/scroll` + last row reachable via `scrollTop`, no page `scrollWidth/scrollHeight`
  overflow, no `[data-backdrop]`, no modal ARIA, focus preserved (Objects->search,
  Inspect->h2). Host still single `Option<ToolPanel>`, focus restoration, outside-pointer,
  nested Escape precedence preserved; no API widening, no second registry, no veto.
- Correction: the test does not change the browser viewport and measures `innerWidth`/`innerHeight` layout viewport bounds, not `visualViewport`. Browser viewport 320px, short viewport, and `visualViewport` offsets remain unproven. Only synthetic fixture direct children (60x40px labelled rows) were tested. This does not close the anchoring/clamping portion of the Roving/clamping proof row, nor the ticket.
  Remaining explicitly planned: full roving (arrows/Home/End, focused-action visibility),
  transient-interface/navigation-admission, generic fitting, canvas invariance, route
  lifetime, commands, quick/lint, boundary mutations, combined review. No full gate
  (coordinator owns).

- Scroll-correct GREEN: `.evidence/toolbar-m1/009-scroll-correct-green.log` — 4 executed, 0 failures, after rename-only to `panel_clamps_to_layout_viewport_and_synthetic_content_scroll_keeps_close_reachable`. No production code/CSS change; assertions and tests preserved. Confirms synthetic content scrolling + layout viewport bounds only; browser viewport 320px, short viewport, and `visualViewport` offsets remain unproven, no anchoring/clamping proof row closure, only synthetic fixture direct children tested.
