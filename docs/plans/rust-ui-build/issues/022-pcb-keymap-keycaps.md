# 022: PCB, Keymap and Keycaps

Status: resolved
Owner: lane/022-pcb-keymap-keycaps (.worktrees/lane-022-pcb-keymap-keycaps)
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The three smaller Workspaces together, with Tabs for the keymap layers. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The PCB, Keymap and Keycaps rows of the inventory and their Workspace hosts have zero raw controls and zero `Board*` uses
- [ ] Keymap layer navigation is Tabs and passes the floor; existing tests pass after the rewrite rule
- [ ] Each Workspace works in both Themes and both Appearances at the three widths
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Comments

2026-11-05 (coordinator adjudication, terra-vs-spark experiment): terra preflight ran `python3 scripts/check.py quick` successfully as a process (harness executed, evidence written, no sandbox/target errors) but the run is RED on the `tooling` step: `scripts/test-check.py` fails with 21 unregistered `scripts/test-*.py` files. Adjudicated NOT a genuine backend failure: the lane branch's `scripts/` is byte-identical to the feat tip (`git diff feat/rust-ui-migration lane/022-pcb-keymap-keycaps --stat` shows the ticket claim only), and the 018 and 021 ticket outcomes document this exact tooling failure as pre-existing on feat. Verdict: terra backend CAN do build lanes → terra continues 022 (and the remaining lanes unless a genuine build failure appears). Evidence: lane `.evidence/022/preflight.log`.

2026-11-05 (coordinator adjudication, full-browser gate): the two full-gate failures in `.evidence/022/browser-final-recheck.log` are ruled PRE-EXISTING / OUT OF SCOPE, not caused by 022: (1) `test-drag-interactions.cjs` `waitForFunction` timeout queries save-state/drag code — the lane diff touches only keycaps/keymap/pcb crates, their hosts, and CSS (27 files, no drag/save-state/Layout sources), and `.m1-save-state` CSS is byte-identical on lane and feat; (2) `ToastCtx` in the Layout selected-key fixture matches the exact signature 023 documents as pre-existing on unchanged sources. 022-owned gates are green (repo/lint/native/build; browser 56/46/32 in the full run; isolated 6/6 on final source; 2 mutation kills; 36-view matrix). The full browser gate stays open for 026 (its criterion owns it). 022 may resolve and fast-forward to feat. The combined Spec+Standards review is deferred to the Phase 1 close-out review per the run brief (single review over base 6fc3956f5); flagged must-check item for it: the wasm-gated native `KeycapsKeyList` API compat risk noted in `## Outcome`.

## Current

Resolved; see [Outcome](#outcome) and [Comments](#comments).

## Outcome

2026-11-05: the PCB, Keymap and Keycaps migrations and their workspace hosts
are committed through `c1cc406a` on the lane.
The inventory scan found no visible raw controls or `Board*` uses in the owned
production rows; the native file picker remains hidden behind a named button.
Layer navigation uses Tabs; component tests cover roving focus, selection,
search, and edit/undo paths. The setup checkbox's label remains a 44px touch
target while its visual indicator stays square. No decision documents changed.

- `python3 scripts/check.py repo lint test build` passed at `50a51ed4`, before
  the one-line checkbox-width correction; `python3 scripts/check.py lint build`
  then passed on `c1cc406a` (`.evidence/022/final-{checks,lint-build}.log`).
- Preflight `python3 scripts/check.py quick` was RED on the pre-existing
  `tooling` registry issue (see [Comments](#comments)); the resolution edit's
  `python3 scripts/check-doc-links.py` passed.
- The full build/browser invocation with the cached Playwright dependency
  (`python3 scripts/check.py build browser`) failed at `50a51ed4`
  (`.evidence/022/browser-final-recheck.log`):
  `page.waitForFunction: Timeout 20000ms exceeded.` at `saved`
  (`scripts/test-drag-interactions.cjs:87`), which still queries the removed
  `.m1-save-state` on the feature tip; and the page suite's first panic was
  `Could not find context dioxus_primitives::toast::ToastCtx` in a Layout test,
  followed by three poisoned-panic-hook failures. These were adjudicated
  pre-existing and out of scope; see [Comments](#comments). The full browser
  gate remains open for ticket 026.
- In that run PCB passed 56/56, Keymap 46/46 and Keycaps 32/32. On the final
  source, `scripts/run-wasm-tests.py` ran five isolated PCB/Keymap/Keycaps/setup
  host groups and passed all six tests, including the Keycaps finding test
  affected by the prior cascade (`.evidence/022/final-host-isolated.log`).
  Two targeted `scripts/mutation-check.py` mutations, `file-action-opens-picker`
  and `same-transport-submits`, were killed and their sources restored
  (`.evidence/022/mutation-{file,setup}.log`).
- Thirty-six populated-workspace screenshots cover Original/shadcn, Light/Dark,
  and 1440, 960 and 390px (`web/target/test-results/022/`, with preference checks
  in `.evidence/022/visual-matrix-final.log`); the narrow inspector was also
  scrolled to its key controls. These are local, gitignored evidence.

The combined independent Spec + Standards review is deferred to Phase 1
close-out; it must assess the compatibility risk that the native
`KeycapsKeyList` API is now wasm-gated because the shared Combobox is wasm-only.
