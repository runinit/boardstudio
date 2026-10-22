# 009: Dock primitives

Status: claimed
Type: build
Blocked by: [002: Complete Theme system](002-complete-theme-system.md), [003: Base component set](003-base-component-set.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [ADR-0035](../../../adr/0035-rebuild-the-workbench-shell-on-rust-ui-patterns.md), [spec](../../rust-ui-migration/spec.md) section 3, [shared contract](../map.md#shared-contract)

## What to build

The two building blocks of the new docks exist and work on their own: a non-modal Sheet and an in-flow Sidenav dock with its own resize.

## Acceptance criteria

- [x] Sheet opens from the edge it is given with no backdrop and no scroll lock, so the canvas stays visible and interactive; Escape closes it and returns focus
- [x] Sidenav's `collapsible: None` form is an in-flow dock that sits under the header (not fixed to the viewport) at the desktop widths
- [x] A dock resizes by pointer drag and by arrow keys, within its minimum and maximum, and each side saves its own width (no shared key)
- [x] Repository browser tests cover resize, persistence per side and the Sheet's non-modal behaviour
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

The first Standards + Spec closure review requested changes. The correction below is implemented; the ticket remains claimed pending substantive targeted re-review and local-landing approval.

### Scope and ownership

- Bound checkout: `/home/chris/.t3/worktrees/ts-boardstudio2/lane-009-dock-primitives`, branch `lane/009-dock-primitives`.
  Verified clean initial HEAD and local feature base `20dd558763bf05681a305c227f58f5eca9375601`; claim saved in `5744f38f8` before implementation.
- Added `ui/sheet.rs`: controlled `Sheet`, `SheetTrigger`, `SheetContent` and `SheetClose`, with four edge directions, named non-modal dialog semantics,
  focus entry/return and an owned Escape listener removed on close/unmount. No backdrop, scroll lock, focus trap or inline script.
  A nested Select consumes its first Escape; the next Escape dismisses the Sheet. Like other overlay primitives, closed content unmounts; domain draft ownership stays with the caller.
- Added only Sidenav's in-flow `collapsible: None` form, exposed as `Sidenav` (no unused collapse-mode API). It has no viewport-fixed wrapper or breakpoint hiding.
  The shell chooses visibility and may pass available width to preserve canvas space. The resize separator supports both sides, pointer capture/release/cancel/loss,
  arrows (20px; Shift 40px), Home/End, bounds and ARIA values. Pointer preview stays local; completion changes only the supplied settings signal's width.
- Reused public `panels::PanelSide`, `PanelSettings` and `use_panel_settings`. The existing `boardstudio:v2:panel:left` and `:right` keys, width bounds,
  decoder and storage-warning owner remain authoritative. Saved modes are preserved; auto-hide fallback and shell integration belong to 011.
- Styling adapts pinned rust-ui source `7120636e68556ddd023a4ec8e083663c7b745318`, consumes existing tokens, keeps focus rings and borderless cards, and adds `data-slot`s.
  Only surgical module exports and two copy-rule registry entries were needed; `ui-shared/src/lib.rs`, 007/008 control modules, icons, fonts and production shell callers were untouched.
- Standalone mounted fixture: `web/crates/ui-shared/examples/dock_primitives.rs`; trusted browser regression: `scripts/test-dock-primitives.py`.
  The fixture is not mounted by production. No dependency, alternative preference store, private-member visibility change or source-document decision change.

### Evidence

- Established the lane protocol in this existing T3 checkout: `target -> /home/chris/.cache/boardstudio-target/009-dock-primitives`; copied only the ignored
  setup/cache assets described by `scripts/new-lane.sh`. Verified target and active builds before building; commands ran serially with project concurrency.
- Initial missing-behaviour RED: the standalone fixture build failed with E0432 for absent `ui::sheet` / `ui::sidenav`, in `.evidence/009-initial-red.log`.
  Additional realistic REDs: Escape left focus on the canvas (`009-sheet-focus-red.log`), and standalone Sheet padding extended the bottom panel to x=1488 in a 1440px viewport (`009-geometry-red.log`).
  Fixed persistent transition tracking and explicit border-box sizing; the same assertions pass.
- `cargo test --locked -p boardstudio-web-ui-shared --lib`: 15 passed (`.evidence/009-native-merged.log`), including existing persistence-decoder bounds/malformed-value tests and the new active-pointer/side resize-delta test.
- `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules python3 scripts/test-dock-primitives.py`: passed after the 007 merge (`.evidence/009-browser-merged.log`).
  Real mouse/keyboard input checks both side widths and live geometry, capture/release/loss, no storage writes during preview, distinct keys across reload, preserved modes, persisted-bound clamping,
  arrows/Home/End, in-flow placement, four Sheet edges, Tab leaving the Sheet, canvas clicks/scroll, nested Select dismissal and focus return on Escape, Close and external close.
- The fixture loads both the existing legacy stylesheet and newly generated Tailwind, with bundled fonts. Screenshots in `.evidence/009-{original,shadcn}-{light,dark}.png`
  and `009-narrow-sheet.png` cover the primitive fixture at 1440x900 and 390x844; these are not production-shell or human Workspace acceptance.
- Impeccable detector over both new UI modules: no findings (`.evidence/009-design-detector.json`). Scoped rustfmt and `git diff --check` passed.
- Merged locally landed 007 (`7b1bb75386f0b43dcc5d7a85530bb5789cdb7156`) without conflicts at `8c254bd0ff4f80cd0a369404ea8d66c1a488e355`;
  its menu implementation and dedicated tests were preserved. All affected checks below passed on that exact merged code; the subsequent review-ready commit changes this ticket only.
- `BOARDSTUDIO_CHECK_JOBS=1 NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules python3 scripts/check.py browser --only ui-shared --receipt .evidence/009-shared-browser-merged-receipt.json`:
  49 passed (47 ordinary + 2 isolated panel tests), no failures/ignored/incomplete results. Receipt and summary record the partial scope; full browser gate was not run.
- `cargo clippy --locked -p boardstudio-web-ui-shared --lib --target wasm32-unknown-unknown -- -D warnings`: passed (`.evidence/009-clippy-wasm.log`).
  `python3 scripts/check-doc-links.py`: passed, 565 Markdown files (`.evidence/009-doc-links-final.log`).

### Remaining gates

Coordinator closure review and local landing are pending. No origin push or dev merge. Global full/quick, CAD `worker.onerror`, first paint, human Workspace checks and final migration merge remain open.

### Closure correction (2026-10-22)

- The reviewed snapshot `c8a02f3936ab2a8a7a88f23e6300c8f67088d579` and its receipts remain in Git / `.evidence/`; nothing was rewritten or overwritten.
  Both closure reviews were source-only; all runtime receipts here are author-executed evidence.
- **Standards P3, trigger lookup:** replaced `button[aria-haspopup=dialog]` with the browser accessibility snapshot's unique role/name lookup for button **Open settings**.
  Its `aria-haspopup=dialog` and `aria-expanded=true` attributes are then read through that accessible ref. No production control or shared test helper changed.
- **Spec P2, description:** `SheetContent` now forwards a required `aria_describedby` prop. The fixture connects it to `dock-settings-description` on its explanatory paragraph;
  the trusted browser regression resolves that reference and checks the description text for every Sheet edge. The fixture is the only current caller.
- **Nested free-form Popover objection:** added the actual public `Popover` / `PopoverTrigger` / `PopoverContent` inside Sheet, with `is_modal: false` and a real content button.
  Before changing Sheet production code, trusted input reproduced one Escape dismissing both dialogs and restoring **Open settings**.
  Expected RED: `.evidence/009-popover-escape-behavior-red.log`; reproducible test-only checkpoint `0de80f9e7df84d0ea4171d3701f2f999a2f2a8eb` retains the reviewed Sheet implementation.
  The passing nested Select case was not used as evidence for this objection.
- The narrow correction moves the owned Sheet listener from document to window bubbling, after primitives' document handlers can prevent their own Escape. Registration and removal use the same window target.
  The regression now asserts that the first Escape closes only the actual Popover, waits for its normal exit/unmount, then verifies a second Escape closes Sheet and restores its opener.
  The Popover source, dismissal lifecycle and classes are unchanged. Modal Popover focus-trap behavior is not exercised by this non-modal composition test.
- Code/test correction: `65899d6a86e6306e92c99c58f3bda20bf2d042c7`. New evidence uses `009-correction-*` filenames, preserving original logs, receipt and screenshots.
  Checks on that exact code passed: native **15/15** (`009-correction-native.log`); the full trusted fixture including role/name popup attributes, description references and actual free-form Popover first/second Escape
  (`009-correction-browser-final.log`); shared WASM **49/49**, no failed/ignored/incomplete results (`009-correction-shared-browser-receipt.json` and summary, `009-correction-shared-browser.log`);
  WASM clippy with `-D warnings` (`009-correction-clippy-wasm.log`); scoped rustfmt, Python compile, diff whitespace and 565-file Markdown link checks (`009-correction-doc-links.log`).
  These use the same commands and partial scope as the initial verification above. The only subsequent change is this ticket's correction record.
  The browser suite receipt may include that in-progress ticket edit; its Rust/test source is exactly `65899d6a`. No full browser/product gate or reviewer runtime execution is claimed.
