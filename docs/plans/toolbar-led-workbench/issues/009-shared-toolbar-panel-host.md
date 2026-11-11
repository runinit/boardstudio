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
- 2026-11-11 Viewport builder (002/009-viewport-proof): RED is real-page `scripts/test-workbench-viewport.py --page-url http://127.0.0.1:4173/boardstudio/` at 320x480 (`.evidence/toolbar-m1/viewport-320-red2.log`, EXIT 1). Browser viewport exactly 320x480, visualViewport 320x480 offset 0,0 scale 1, fonts loaded. Row 42px (passes). Roving/clamping entry portion FAILS: 6/7 entries not truly hit-testable (Select offscreen, Transform occluded by Project, Snap/View/Objects/Inspect occluded by empty-state; only Align passes) due to accordion pill flex-column stacking (see 002 progress for dimensions/cause). Clamping portion PASSES for real panels: Objects/Inspect open, bounds x8 y114 w304 h270 inside viewport (Objects min320vw-16 / Inspect min400vw-16 satisfied at 320w -> 304w), Close 64x40 at (238,119) true hit, no page overflow, no browser errors. This materially differs from prior synthetic 320px fixture-lane proof (which never set browser viewport or visualViewport): real browser viewport + visualViewport now exercised. No production/CSS edit per handoff. Proposed narrow fix: `flex-direction: row;` on `.layout-toolbar-accordion` (002-owned CSS). Next: re-run after fix, then roving arrows/Home/End + focused-action visibility + short visualViewport/keyboard offsets (still unproven; desktop emulated viewport does NOT prove keyboard/zoom). Proof-map rows stay planned; no closure claim.
- 2026-11-11 Row-fix builder (002-row-fix, 009 entry portion): GREEN after 002-owned `flex-direction: row;` on `.layout-toolbar-accordion` (see 002 progress for Gortex/build). Unchanged `scripts/test-workbench-viewport.py`: 320x480 `.evidence/toolbar-m1/row-fix-viewport-320.log` EXIT 0 (7/7 entries true hit, row 42px, visualViewport 320x480 offset 0,0 scale 1), 390x844/728x1024/1440x900 logs EXIT 0, panels/Close inside viewport, no overflow, 0 browser errors. Roving/clamping entry portion now passes for closed row at 4 real browser viewports (materially differs from prior synthetic fixture lane); clamping still passes. Remains planned: arrows/Home/End, focused-action visibility, each-section-open entries, short visualViewport/keyboard offsets/zoom, transient/navigation, generic fitting, canvas invariance, route lifetime, commands; proof-map rows stay planned; no closure claim.
- 2026-11-11 Open-sections builder (002-open-sections, 009 entry portion): RED after extending `scripts/test-workbench-viewport.py` with each-section-open oracles (see 002 progress for Gortex). Same real-page run `.evidence/toolbar-m1/open-section-320.log` EXIT 1 at 320x480 (visualViewport 320x480 offset 0,0 scale 1, fonts loaded, 0 anims): closed row still GREEN (7/7 hit, 4x4.0 spacing, 42px, no overflow), panels/Close still inside viewport with no overflow, but Roving/clamping entry portion FAILS with each section open (5 failures per section, 25 total): open trigger occluded by next trigger/Objects (hit false), Inspect center 322-326 outside 320 (inside false, hit null), stationary gaps -28.000px overlap (require >=4px), open lane clientWidth 3 revW 0 (require >=32px). One-open, 42px row, no page overflow hold. No production edit (002 defect stop). Clamping portion still passes for closed/open panels. Remains planned: arrows/Home/End, focused-action visibility, short visualViewport/keyboard offsets/zoom, transient/navigation, generic fitting, canvas invariance, route lifetime, commands; proof-map rows stay planned; no closure claim.
- 2026-11-11 Open-fix builder (002-open-fix, 009 entry portion): GREEN after shared-row shell.css fix (see 002 progress for diagnosis/Gortex/build: :has hides heading+chevron when open, active/all restore via measured data-fit-headers, row padding 5px 7px->5px 2px). Unchanged probe: 320x480 open-fix-320-green.log EXIT 0 (7/7 true hit closed+each-open, 32px, gaps 38/4, lane34, 42px, no overflow), 390x844/728x1024/1440x900 logs EXIT 0 (lanes 104/255-442/740, headings/chevrons/labels restored when space). Roving/clamping entry portion now passes for closed+each-open at 4 real viewports (visualViewport exact, fonts loaded, 0 errors); clamping still passes (panels/Close inside, no overflow). Focused wasm open-fix-wasm.log EXIT 0 (9/0). Remains planned: arrows/Home/End, focused-action visibility, short visualViewport/keyboard offsets/zoom, transient/navigation, generic fitting, canvas invariance, route lifetime, commands; proof-map rows stay planned; no closure claim.
- 2026-11-11 Lane-fix builder (002-lane-fix, 009 entry portion): GREEN after 002-owned layout-toolbar.css accordion-only fix (see 002 progress for Gortex/build: Select 32x32, Transform guidance 32px readable, View Canvas 96x32). Unchanged closed/open entry/spacing/panel oracles still pass: 320x480 lane-fix-320-green2.log EXIT 0 (7/7 true hit closed+each-open, gaps 38/4, lanes 34, guidance within row/track, usable Select/Snap/View, Align truthful no-selection, 42px, no overflow), 390x844/728x1024/1440x900 logs EXIT 0 (lanes 104/255-442/740, headings restored). Roving/clamping entry portion still passes at 4 viewports; clamping still passes. Focused-action visibility partially proved (usable enabled fully within + topmost, with native scroll proof for Snap), but arrows/Home/End, short visualViewport/keyboard offsets/zoom still unproved. Remains planned: arrows/Home/End, transient/navigation, generic fitting, canvas invariance, route lifetime, commands; proof-map rows stay planned; no closure claim.
- 2026-11-11 Lane-label builder (002-lane-label-fix, 009 entry portion): GREEN after 002-owned fixes (see 002 progress for RED/Gortex/build: Select responsive, Canvas max-width, Transform Tooltip). Unchanged entry/spacing/panel oracles still pass at 4 viewports: 320/390/728/1440 `lane-label-*-green*.log` EXIT 0 (7/7 hit closed+each-open, gaps, lanes 34/104/381-442/740, 42px, no overflow, panels/Close pass). Roving/clamping entry portion still passes; clamping still passes. Corrects lane-fix overstatement: focused-action visibility was NOT proved via native scroll (programmatic scrollIntoView only, keyboard focus scroll unproved – focusing offscreen action scrolling lane without moving banks remains unproved; native touch scrolling unproved). Strengthened oracle now proves full-clip (every ancestor+viewport) + hit + effective enabled via programmatic scroll (nearest/start/center/end), plus Transform tooltip keyboard (focus) + pointer (hover) with full-text match in-viewport hit. Remains planned: arrows/Home/End, keyboard focus scroll, touch, short visualViewport/keyboard/zoom, transient/navigation, generic fitting, canvas invariance, route lifetime, commands; proof-map rows stay planned; no closure claim.
- 2026-11-11 Roving-guidance builder (002-roving-guidance, 009 entry portion): RED was real-keyboard roving (see 002 progress for Gortex/build): 320 4 fails (extra Tab stop 2, Arrow miss, tooltip, Handles 64px in 34px), 390 3 fails, 728/1440 2 fails each (extra stop+arrow even when readable). Corrects lane-label overstatement: Transform p tabindex0 was extra stop with imperative focus proof (not single roving with real Tab/arrow), View Handles w64 not whole-usable at 320. Fix owned-only 002 (RS ToolbarButton roving guidance + CSS guidance/Handles chain, Test real-keyboard single-stop/arrow/Home/End/tooltip + strict select + Handles usable). GREEN after fix at 4 viewports `roving-guidance-*-green.log` EXIT 0 (single stop, ArrowRight reaches guidance BUTTON 32px visible, tooltip from reached focus at 320/390 truncated + hover, roving at 728/1440 readable, Handles w32 whole usable at 320, select exact 5, 7/7 hit, 42px, no overflow, panels pass). Roving/clamping entry portion now proves single tab stop + arrows/Home/End for Transform guidance via trusted keys (initial Transform trigger precise); clamping still passes. Focused-action visibility still programmatic scroll only (keyboard focus scroll for offscreen actions unproved, touch unproved). Remains planned: keyboard focus scroll, touch, short visualViewport/keyboard/zoom, transient/navigation, generic fitting, canvas invariance, route lifetime, commands; proof-map rows stay planned; no closure claim.
- 2026-11-11 Handles-tooltip builder (002-handles-tooltip, 009 entry portion): Corrects hover: Handles action usable (w32 hit) did NOT imply tooltip visible — genuine hover showed tooltip above clipped (zero intersection, center hits Saved locally). Fix owned-only 002 CSS fixed+anchor below (see 002 progress for Gortex/build): real 320x480 packaged hover+focus handles-tooltip-fix-320b.log shows tooltip 98x32 x151.9 y116.5 inViewport+inSection center hits tooltip, button 32x32 x201 hits Handles, track scroll141 preserved, row 42px, no overflow. Roving/clamping entry portion still passes (closed/open preserved, Transform tooltip/keyboard untouched — handles-scoped only); clamping still passes. Focused-action visibility still programmatic scroll only (keyboard focus scroll offscreen unproved, touch unproved). Remains planned: keyboard focus scroll, touch, short visualViewport/keyboard/zoom, transient/navigation, generic fitting, canvas invariance, route lifetime, commands; proof-map rows stay planned; no closure claim.
- 2026-11-11 Mechanics builder (009-transient/navigation/lifetime/commands/canvas): CLOSED five proof-map rows with mounted tests on the private host (Gortex exact worktree: impact before each edit, detect after; all builds under the required lock; no CSS/visual touch). (1) Transient interface CLOSED: RED `.evidence/toolbar-m1/009-transient-red1.log` (refused hide still opened the panel); production change `PanelUtilities on_hide_transient: EventHandler<()>` → `Callback<(), bool>`, honored (refusal aborts the open), Layout toolbar returns true, 3 fixtures return true; GREEN `009-transient-green1.log` (11/0): refused leaves owner+panels unchanged, allowed parks to one surface, own close after switch reopens nothing + focus to utility, nested-picker Escape preserves parent (second Escape closes). (2) Navigation admission CLOSED without production change: `accepted_route_...` (same-value set keeps surface+route; accepted Layout→Case detaches panel, clears host focus via browser removal handling, token/revision unchanged, old Escape callbacks dead, return reopens nothing); no domain veto exists yet so refusal = unchanged-route only — true admission integration stays 005; focus lands on body, not explicitly the new toolbar/heading (accepted as "clears host focus"). (3) Route lifetime CLOSED without production change: `caller_owned_input_...` (caller input above host survives workspace round trip + session replace, no auto-reopen, replaced session closes old panel + clears old focus/callbacks, new session functional); session replace = test-support reinstall + new document id (fresh installs restart epoch/token at 1, so the same-session epoch-bump path is unexercised). (4) Commands CLOSED: RED `009-commands-red4.log` (command close stranded focus on the invoke button, expected utility); production change `workbench_shell::toggle_tool_panel` (pub(super), focus-aware close) wired into Editor `toggle_objects`/`toggle_inspector`; GREEN `009-commands-green2.log` (10/0 at the time): Layout commands toggle only the new host (320/400px vs seeded 420/480 legacy widths, no dock, legacy compact untouched, canvas stable), Case/PCB commands toggle only legacy (data-mode flips, seeded 420 honored, no ghost workbench), return reopens nothing. Desktop branch only (runner 1280x900); compact branch present but unexecuted. Command-open while the Layout menu is open does not yet dismiss the menu (utility path does) — follow-up. (5) Canvas invariance CLOSED without production change: Layout shape pre-proven (`layout_panels_overlay_...`); new `case_content_shapes_keep_canvas_rect_and_camera_across_open_switch_close` mounts REAL Case `objects()` + full `inspector()` (physical-setup mount, no mechanical props/scene, board_document) through `PanelHost` with a labelled canvas stand-in (data-camera marker): open/switch/close keeps identical rect/marker/node + revision. GREEN `009-canvas-green2.log` (14/0 final across both files). Honest limits: real WebGL CasePanel viewer never mounted (no existing mounted coverage; headless risk); canvas stand-in proves seam-fit + sibling stability, real-viewer invariance rests on the Layout proof. No mutations run (coordinator owns per brief); named proof-gap mutations in verification.md still apply to the immediately-green rows. Remains planned: exclusion/dismissal formal closure, generic fitting, roving/clamping leftovers (keyboard focus scroll, touch, short viewport/keyboard/zoom), quick/lint, boundary mutations, combined review; no full gate (coordinator owns).

- 2026-11-11 Coordinator closure evidence (pre-review): all proof rows have mounted receipts (`009-coord-verify.log`, 14/0; later suites 17/0). Mutations through `scripts/mutation-check.py`, specs in `docs/plans/toolbar-led-workbench/mutations/`: exclusion, probe-inert, transient refusal, return focus, generic fitting — killed at named assertions; navigation admission and reframe mutants are caught by sibling assertions of the same proof rows (`caller_owned_input_survives_round_trip_and_session_replace_clears_old_host` "returning reopens no panel"; `show_hide_commands_route_to_each_workspace_current_host` canvas-rect assert), not by the originally named test, whose failure only cascaded — recorded honestly as caught-at-row-oracle, not named-test kills. Real-browser matrix 320/390/728/1440 passed (`.evidence/toolbar-m1/closure-viewport-*.log`, `checker/viewport-1440.log`). `check.py lint` passes; `check.py quick` fails only on `layout_field_preview_tests` (0/6, recorded in ticket 005 as the Add/setup integration red). Touch/wheel lane scroll unproved. Combined review pending.

- 2026-11-11 Correction: the earlier Commands closure claim omitted the transient-owner reveal contract. Show/hide commands now share the utility request path; real Layout menu dismissal and a refusing synthetic owner are covered by `009-command-transient-green.log` (18/0). The preceding RED receipts are `009-command-transient-red.log` and `009-command-refusal-isolated-red.log`. This corrects that Commands claim only; it does not close the ticket or its remaining gates.
