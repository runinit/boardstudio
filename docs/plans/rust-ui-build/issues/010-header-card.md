# 010: Header card

Status: claimed
Type: build
Blocked by: [002: Complete Theme system](002-complete-theme-system.md), [004: Vendored Lucide icons](004-lucide-icons.md), [007: Accessible action menus](007-accessible-action-menus.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [ADR-0035](../../../adr/0035-rebuild-the-workbench-shell-on-rust-ui-patterns.md), [structure decision](../../rust-ui-migration/issues/011-target-workbench-structure.md), [shared contract](../map.md#shared-contract)

## What to build

The header becomes a 56px borderless card with the project identity and Project menu, the Workspace links, the Command search button, save status, undo and redo, one theme control and Export. A user can switch Theme and Appearance from the header.

## Acceptance criteria

- [x] Project and Settings are Dropdown Menus passing the floor; the Appearance and Theme choices are two radio groups in one theme control, replacing the temporary placement from ticket 001
- [x] Workspace links are ghost buttons keeping `aria-current` on the active one; Export is the primary button; undo and redo and save status keep their behaviour
- [x] The Command search button is present and inert until ticket 012 wires it
- [x] The header holds up at desktop, mid and narrow widths with the mobile menu; existing header tests pass after the rewrite rule
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

Implemented the 56px borderless header card in `presentation/header.rs` and `assets/header.css`.
The header consumes workspace/save signals, history/Export callbacks and the existing panel-control
element; App retains its Runtime, subscriptions, provider ancestry, workspace/Export-return signals
and mounted Editor lifetime. The parked extraction work was not integrated. The coordinator's live
overlap audit found no matching active writer before shared shell edits began.

Project is the reviewed Dropdown Menu, with Library-owned new/open/copy/browser/guide operations.
Creation uses the same private helper as Library, including its pending/error handling. Rename,
saved-project selection, deletion, demos and recovery remain in the existing project browser reached
by Browse projects; Return to editor preserves the mounted Editor. The native file input stays
mounted outside the menu so closing a command does not drop an in-progress file selection. History
and Export invoke the original callbacks. No persistence, admission or document API changed.

Settings is one Dropdown Menu with Appearance and Theme radio groups (`menuitemradio` /
`aria-checked`), replacing the temporary native selects. The existing preference functions, keys,
legacy Appearance fallback, values and system-appearance owner are unchanged. Workspace ghost
buttons retain their names and `aria-current`; primary Export retains its pressed state. Button
has two optional caller-state passthrough props only, `aria_current` and `aria_pressed`, on both
render branches; defaults, sizes, class/provenance rules and Themes are preserved.

At 1250px and below, a non-modal compact header popup holds the same command presentation. Its
owned open state supplies trigger ARIA and local focus entry/return through the primitive's retained
exit; it preserves focus on an outside control. Nested Settings expands within the bounded popup
instead of clipping its choices. Header and save status remain visible. Existing panel controls are
consumed without migrating their behaviour; no dock/Sheet, workspace-control, palette, Toast or
remaining-shell migration was pulled forward. Search is deliberately inert under the explicit
010/012 contract. Vendored ChevronDown is reused and the existing BoardStudio logo is retained.

### Source and verification

Initial T3 binding, CWD and branch matched `lane/010-header-card`, with a clean tree at the required
`7b1bb75386f0b43dcc5d7a85530bb5789cdb7156`; 002–007 were resolved there. Claim `9ecf38b7` preceded
changes, and `8562483f` saved the mounted RED. Implementation `25e77947` and edge corrections
`b5f4e0d6` were committed separately. Local feature 009 was merged as `20a4dd5a`; latest local
feature `95ef094d2152165e8da771c652b62bd677b06c9c` (including 008) was then merged cleanly as
`951006d4be609fb928dd388e1f01eb9b3bbdf4cd`. The 006/007/008/009 source and registrations were
compared with feature and are unchanged. The final checks below ran on that clean merged source.

All builds used the bound checkout's `target` link to
`/home/chris/.cache/boardstudio-target/010-header-card`; protocol settings/CAD cache/pkg copies were
established before the first build. Cargo/WASM runs were serial through that target. Long logs and
captures are under this lane's ignored `.evidence/010/`, with no source copies or custom manifests.

- **RED:** `header-red.log` records the actual incumbent header at **64px instead of 56px**.
  Further retained failures were the off-screen compact popup (`header-green-1.log`), nested Settings
  horizontal overflow (`header-edges-1.log`), missing Popover trigger state (`mobile-aria-red.log`),
  focus left on Layout in retained closing content (`header-focus-diagnostic.log` and its DOM/capture),
  and clipped Original choice (`header-clip-red.log`). Each has a corresponding assertion in the final
  production test. Intermediate compile/animation-diagnostic failures are retained and are not GREEN.
- **Native:** `cargo test --locked -p boardstudio-web-ui-shared -p boardstudio-web-library --lib`:
  **15 + 5 passed**, `native-post008.log`. No native test was renamed or baseline weakened.
- **Mounted shared/Library:** `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules
  BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py browser --only ui-shared library --receipt
  .evidence/010/browser-post008.json`: **64 + 2 + 17 passed**, zero failed/ignored/incomplete.
  `browser-post008.json` and `.json.md` bind the receipt to clean `951006d4be60`; log
  `browser-post008.log`. This includes the new role-and-name Button state/radio owner test, 007's
  action-menu tests, 008's 16 mounted cases, the base primitive/icon tests and Library's failure,
  pending draft, ownership, retry and deletion regressions.
- **Clippy:** `cargo clippy --locked -p boardstudio-web -p boardstudio-web-ui-shared
  -p boardstudio-web-library --no-default-features --features boardstudio-web/page
  --target wasm32-unknown-unknown --tests -- -D warnings`: **passed**, `clippy-post008.log`.
- **Packaged page:** `python3 scripts/build-web.py --fast --skip-providers`: **passed**,
  `build-post008.log`. It reuses the lane's runtime providers from the initial successful
  `python3 scripts/build-web.py --fast` (`base-build.log`); provider source is unaffected by this slice.
- **Production interaction/geometry:** with `python3 scripts/serve-web.py web/target/site 4180`,
  `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules python3 scripts/test-header.py
  http://127.0.0.1:4180/boardstudio/ --evidence .evidence/010/header-post008`: **passed**,
  `header-post008.log`. Real role/name actions and key presses cover creation, all six workspaces,
  menu arrows/Home/End/typeahead/activation, save details, rename, button and keyboard Undo/Redo,
  Export return/pressed state, real copy download, import restoring the copied project after opening
  a different one, inert Search, compact navigation/close focus, nested Settings focus and unclipped
  choices, Project-to-Settings routes at both widths, all four Theme/Appearance pairs across reload,
  System changes and legacy/new-key precedence. Header geometry is 56px and borderless at
  **1440/1024/768/390px** in every pair; captures and `geometry.txt` are in `header-post008/`.
- **Retained Theme test:** `scripts/test-theme.py` uses the new radio roles without changing its
  token/font/geometry assertions. The same URL/evidence invocation with
  `--evidence .evidence/010/theme-post008` **passed**, `theme-post008.log`.
- `cargo fmt --all -- --check`, `git diff --check`, `python3 scripts/check-doc-links.py`,
  `python3 scripts/check-wasm-tests.py`, and Python compilation of both browser scripts: **passed**.
  Logs: `fmt-post008.log`, `doc-links-post008.log`, `wasm-declarations-post008.log`.
  The Impeccable detector's one advisory (`design-detector.json`) parses the `+ 4px` part of
  `calc(var(--radius) + 4px)` as a legacy DESIGN radius mismatch; that is the pinned migration's
  rounded-xl card metric. DESIGN's rewrite remains ticket 025. Later focus/position fixes do not
  change that radius. Captures were inspected for layout; interaction proof comes from the tests.

These are scoped Chromium/native checks, not the global full-product/build/browser/quick gate.
CAD `worker.onerror`, first paint, screen readers, complete human visual review/026 and the final
merge remain open. No origin push, dev merge, new agents/threads/watchers, Gortex operation,
parked extraction integration or coordinator cleanup edit was performed.

### Closure review

Pending one real Standards + Spec closure review of this exact ticket slice and coordinator approval
before local landing. Status remains claimed while that gate is open.
