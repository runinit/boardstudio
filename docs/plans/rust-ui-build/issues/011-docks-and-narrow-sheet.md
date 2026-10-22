# 011: Docks and narrow-screen sheet

Status: claimed
Type: build
Blocked by: [009: Dock primitives](009-dock-primitives.md), [010: Header card](010-header-card.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [structure decision](../../rust-ui-migration/issues/011-target-workbench-structure.md), [ADR-0035](../../../adr/0035-rebuild-the-workbench-shell-on-rust-ui-patterns.md), [shared contract](../map.md#shared-contract)

## What to build

Objects and Inspector are cards on the new docks, with pinned and collapsed modes, and below 980px every Workspace shows them as one Sheet with Objects and Inspector tabs, leaving the canvas visible and interactive.

## Acceptance criteria

- [x] Objects (left) and Inspector (right) use the dock primitives as cards on the desk; widths and mode persist per side
- [x] Auto-hide is removed; a saved auto-hide preference becomes pinned without losing the user's widths
- [x] Below 980px every Workspace uses the same non-modal Sheet with Objects and Inspector tabs, replacing the separate drawer and scrim behaviour of the other Workspaces
- [x] Layout's Add action still opens its create view inside the Objects dock
- [x] Existing dock and panel tests are rewritten per the test rule and pass; the removed auto-hide tests are deleted explicitly
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

Closure review at `10f6008bcc6b7323b58fbcbcdd3ca32714518077` returned Standards APPROVE (0 findings) and Spec CHANGES REQUIRED (3 findings). The correction slice below awaits coordinator-owned targeted Spec re-review. Standards approval carries only for unchanged, unrelated scope. Status stays claimed and landing is held.

### Behavior and ownership

- Objects and Inspector use in-flow Sidenav cards, retaining per-side saved widths and pinned/collapsed modes. Saved `autohide` decodes to pinned without losing a valid width. Old hover/timer/scrim behavior is removed; the standalone primitive harness's obsolete autohide expectation now expects pinned. No native test names were added, removed, or renamed and the native name baseline is unchanged.
- Collapsed dock children remain mounted under hidden/inert to preserve local drafts while removing their subtree from keyboard and accessibility interaction. Legacy `m1-objects-panel`, `m1-inspector-panel`, content and Add-trigger IDs remain for existing focus paths.
- Below 980px all six editor Workspaces share one nonmodal lower Sheet with Objects/Inspector Tabs. The canvas remains visible and interactive; canvas/domain code and callback ownership are preserved. The App's existing `CompactPanelState` and Editor-local desktop `PanelSettings` remain authoritative. The Sheet adapter reads its open signal unconditionally, covering Escape followed by resize to 390px and the 979/980 boundary.
- Layout Add still opens its existing create view inside Objects, with nested Escape returning to Add while preserving the outer Sheet. Field/draft/commit ownership remains with the existing domain subtrees.
- Approved shared seams are optional Button expanded/controls/haspopup attributes on both render branches and an optional stable Sheet ID; defaults remain unchanged. The role helper's fallback name skips aria-hidden descendants while preserving explicit labels and inline text. Setup-guide tests use role/name controls, with unchanged assertions and test names.
- The fresh screenshot read found a heading overflow at the minimum Objects width: the 200px card contained 333px of heading controls. A focused real-package assertion failed before adding `flex-wrap: wrap` to the owned dock heading CSS. The production harness now checks each heading control at both width limits.
- Latest local feature's 013 NotificationProvider/WorkbenchApp, typed queue, runtime/export/firmware tests and stack CSS are retained. No 012 branch or command API was merged. Ticket 012 owns later Editor-local command registration using existing signals/settings after 011 lands; no state change or visibility widening is part of 011.

### Source and reused evidence

The authoritative [continuation](../011-lane-continuation.md) records the original RED/GREEN sequence and commands. Original lane, cache and evidence were preserved. Reused logs below are under `/home/chris/.t3/worktrees/ts-boardstudio2/lane-011-docks-and-narrow-sheet/.evidence/`:

- Clean production source `8978cd24d8b2758894d720342bac3bf749764f49`; final original harness/source `18a43362b4b2027d8a082b2f3c3325d6d15efff1`. The intervening change is only the dock page harness. Handoff `acbb102fa475bcd7ea71803334df80a731c0e36d` fast-forwarded from the clean new base `b237903705244a266b07a64e5f23b8a3b5219c03`. Local feature `1a692e0eece287f094eafaf7e395920c18585f73` merged without conflicts at `680bf0f0dc65805f92f6a63800e1488163c51e36`.
- `011-merged-mounted-receipt.json` and `.json.md`: 220 mounted passes (shared 65 + panel 2 + layout 152 + setup 1), 5/5 commands, zero failed/ignored/incomplete. Partial selection: no page suite or provider/product-smoke acceptance.
- `011-merged-native.log`: 15/15 native shared-library tests. `011-native-name-guard.log`: 11 baseline matches, zero missing, four pre-existing tests new to baseline. Its generic 2104 total does not mean 2104 tests executed.
- `011-merged-build.log`: checked package build including providers passed. `011-merged-fixture.log`: production-component dock fixture passed. `011-production-final.log`: actual packaged six-Workspace flow passed, including Case stage, browseParts/pinInspector boundaries, Layout Add and mobile focus cases. Browser errors were empty.
- `011-primitives-final.log`: standalone 009 regression passed; later default Sheet behavior is unchanged and the new optional props are covered by the merged mounted receipt. `011-closure-clippy-{shared,page}.log`: both WASM Clippy invocations passed with `-D warnings`. Targeted rustfmt, Python compilation and diff checks passed.

### Fresh merged checks

New CWD `/home/chris/.t3/worktrees/ts-boardstudio2/t3-f884f4fd`, branch `fix/rust-ui-011-dock`, is an app-owned child of coordinator `9b186519-9e71-4e26-a24b-6587e94c69e4`. Its initial tracked/untracked inventory was clean. Target resolves to `/home/chris/.cache/boardstudio-target/011-docks-lineage`; protocol settings and CAD caches were copied before the first build. All fresh logs/captures below are in this new checkout's `.evidence/`. Source is merge `680bf0f0` plus the heading CSS fix and its harness regression; the final review commit records those changes.

- `python3 scripts/build-web.py --fast`: PASS, `011-lineage-build.log`, including fresh core/CAD/offline workers, renderer, CAD provider/cache path, Ergogen models and packaged page. No providers skipped.
- `python3 scripts/test-dock-layout.py --page-url http://127.0.0.1:4215/boardstudio/`: PASS, `011-lineage-production-first.log`. Includes the width-limit heading regression, all six Workspaces, Case stage, Layout Add, real browseParts/pinInspector callbacks at 979/980, normal/unmounted mobile Escape and retained-inert Close panels focus return. `011-production-browser-errors.json` is empty.
- `python3 scripts/test-toasts.py http://127.0.0.1:4215/boardstudio/ --evidence .evidence/011-lineage-toast-stack --stack-only`: PASS, `011-lineage-toast-stack.log`. Actual 14-result bursts preserve multiplicity, passive atomic announcements, Settings access, scrolling, keyboard dismissal and F6 across both Themes/Appearances at four viewport sizes.
- `wasm-pack test --headless --chrome web/crates/ui-shared --locked --lib -- panels::`, using `scripts/run-wasm-tests.py::runner_environment()`: PASS, 2/2 mounted panel tests, 65 filtered out, `011-lineage-panels-mounted.log`. No concurrent Cargo/wasm-pack commands shared the target.
- `python3 scripts/test-dock-layout.py`: PASS, `011-lineage-fixture-settled.log`, after adding a condition-only selected-Inspector wait for each theme capture. The first fresh run also passed (`011-lineage-fixture.log`), but its shadcn-light screenshot caught the previous Objects selection before rendering settled. The final four theme captures were inspected again after the wait; this is a capture correction, not a behavioral failure masked by retry.
- `python3 -m py_compile scripts/test-dock-layout.py scripts/test-dock-primitives.py`, `git diff --check` and `python3 scripts/check-doc-links.py`: PASS; documentation log `011-lineage-doc-links.log` checks 566 Markdown files.

The focused heading RED is `011-heading-focused-red.log` against the retained package, before the CSS edit. A separate full replay against that retained package stopped earlier when the Sheet covered Browse parts (`011-heading-overflow-red.log`); that is not counted as the heading RED or as a pass. The freshly built merged package passed the complete replay without weakening assertions or adding sleeps/retries.

### Visual inspection and limits

The two retained production screenshots and four `011-narrow-{original,shadcn}-{light,dark}.png` fixture captures were read in this continuation. The fresh merged `011-production-desktop.png`, `011-production-narrow.png` and `011-production-objects-{home,end}.png` were also inspected: heading controls fit within the card at both width limits, with a visible canvas and one lower Sheet on narrow screens. Minimum-width tree labels elide within their rows. The old desktop Toast numbering is superseded by the merged 013 stack CSS. Fixture captures are not product-wide theme/contrast acceptance.

The normal mobile Workbench -> Objects -> Inspector -> Escape sequence, with the menu unmounted, is distinct from the retained-menu candidate test. A retained connected+inert exiting Header Popover consumes Escape until it unmounts; 011 respects nested preventDefault. The approved retained-candidate test uses the real Close panels button and checks return to Workbench menu. This is a documented limitation, not an Escape pass. Header/Popover ownership is unchanged.

Global full/quick, CAD worker.onerror, first paint, screen reader, human026 and final merge remain open. Neither reused nor fresh scoped checks certify every Workspace draft/focus route. No origin/dev push, local feature landing or review delegation has been performed by this child.


### Closure corrections after `10f6008`

The three Spec findings and the original review receipts are preserved in `.evidence/011-r1-review-results.md`; reviewed screenshots are retained in `.evidence/011-reviewed-10f6008-captures/`. Review nodes are `delegated-task:command%3Amcp%3A2a86db4d-5ca5-4383-b139-deebb4fe7ee7%3Adelegate-task%3Arust-ui011-{standards,spec}-review-r1`.

- **P1, domain-local draft lifetime:** inspected the public pinned Tabs lifecycle before changing composition: inactive `TabsContent` unmounts its children, and the default Sheet unmounts closed content. The actual production RED (`011-r2-local-offset-red.log`) filled the existing Layout constraint Offset X with unsubmitted `123.456`; Inspector -> Objects -> Inspector disconnected that input and recreated it with `0`. DockLayout now keeps one stable keyed domain subtree through tab selection, Sheet visibility and desktop/compact transitions. Inactive panels and closed retained content use hidden/inert presentation, with unique active tab roles, labels and control IDs. No domain draft was lifted or fixture value substituted for acceptance. Workspace-specific domain selection/lifetime still belongs to the existing workspace composition; 011 does not retain unrelated Workspace domain components after their owner replaces them.
- **P2, clipped Options popup:** `011-r2-options-bounds-red.log` records the 250px nonportalled popup/control extending left of the 200px card. The owned popup is now anchored to the heading and bounded to its available width; both Tailwind's individual `translate` and `transform` are cleared. Real Home/End card-width replay checks control bounds and hit testing at both edges, changes Rows/Columns through the native select, and verifies the stored result.
- **P2, Options trigger semantics:** `011-r2-options-aria-red.log` records missing expanded/haspopup attributes while open. The existing public controlled Popover state now drives `aria-expanded`, with `aria-haspopup="dialog"`. Production assertions check false -> true -> false and focus return on Escape. No new primitive API was added for Options.

The coordinator explicitly approved two optional `SheetContent` props in `web/crates/ui-shared/src/ui/sheet.rs`: `keep_mounted=false` and `inline=false`. Defaults preserve existing callers. DockLayout opts into retained mounting and renders that same subtree inline on desktop, without dialog semantics, fixed positioning or focus-on-open. Its desktop adapter closes the Sheet signal and removes the owned Escape listener; the real field keeps focus and Escape remains unprevented at 980px. New props and cross-mode cleanup are part of targeted re-review. No Tabs API, callback/state ownership or private visibility was widened.

The coordinator also approved exactly two event hunks on the existing Layout constraint summary in `web/src/presentation/inspector/layout_component_inspector.rs` (around line 1313):

1. Call `event.prevent_default()` before the existing controlled `constraint_open.set(!constraint_open())` click toggle. A trusted click previously caused controlled opening followed by native closing; `011-r2-disclosure-native-controlled-red-final.log` correlates the click, two open-attribute mutations and final closed state. This is a real disclosure correction required to reach the local draft, not a test setup bypass.
2. On Space keydown only, stop propagation without preventing default. The existing Window `LayoutSpacePanWindowListener` prevented Space on SUMMARY, leaving no native activation click (`011-r2-local-key-trace.log` and `011-r2-space-prevent-default-trace.log`). Dioxus's virtual `stop_propagation()` alone does not stop that native Window listener (`011-r2-production-final-held-space.log`); the same guarded boundary therefore also calls `stop_propagation()` on `try_as_web_event()`. Other keys remain untouched, existing handlers run once, and no Canvas source changed. Draft, validation and commit callbacks are unchanged.

`011-r2-production-native-space-final.log` proves trusted click and Enter open/close once, and native Space activation on keyup produces exactly one click and one open-attribute mutation in each direction. The held keydown is trusted and unprevented, with no click/mutation before keyup. The same replay then types into the actual local Offset X draft and checks identical input/domain-owner nodes and value through both tab directions twice, Sheet close/reopen and 979/980 twice in both directions. Hidden draft content is absent from the interactive accessibility snapshot and refuses focus. Canvas Space+pointer drag outside the control still changes the viewBox, preserving selected-part identity and the local draft. No keyboard skip, programmatic details opening or draft submission is used.

### Correction validation and remaining gates

All correction evidence is in this checkout's `.evidence/`, on `10f6008` plus this coherent correction diff:

- `011-r2-build-native-space-final.log`: fresh checked page build, `python3 scripts/build-web.py --fast --skip-providers`, PASS. Unchanged provider artifacts and their full build acceptance are reused from `011-lineage-build.log`; this page rebuild does not claim fresh provider execution.
- `011-r2-production-native-space-final.log`: fresh complete real-package replay at `http://127.0.0.1:4219/boardstudio/`, PASS, including all six Workspaces, Layout Add, Case stage, width/breakpoint callbacks, normal mobile Escape versus retained-menu Close, all three finding regressions, native summary activation and actual local draft retention. `011-production-browser-errors.json` has zero errors.
- `011-r2-clippy-{shared,page}-final.log`: both final WASM Clippy invocations with `--all-targets --no-deps -- -D warnings` passed. `011-r2-fixture-keyboard-captures-final.log`: fresh production-component fixture passed, including its component-local draft, tabs, nested Escape/focus, canvas interaction, modes, widths, breakpoints and four theme captures.
- `011-r2-mounted-shared-receipt.json` and `.json.md`: 67 mounted passes (65 shared + 2 panel), 2/2 commands, zero failed/ignored/incomplete. `011-r2-setup-guide.log`: 1/1 setup test passed, 152 filtered. These were executed on the correction's final Sheet lifecycle logic before the final dock-only CSS selector narrowing and approved native summary boundary. The final fixture and page replay cover those later changes; the 67 are reused rather than represented as re-executed.
- `011-r2-primitives.log`: default 009 regression PASS on the new Sheet props/lifecycle. Reused after later dock CSS scoping and domain summary edits, which do not alter default Sheet source or the standalone primitive fixture. Earlier native15, layout152, merged 013 stack/provider receipts remain scoped reused evidence as listed above.
- Final targeted Rust formatting, Python compilation, diff and Markdown-link checks passed. Fresh desktop, 390px Sheet, minimum/maximum Objects and 979px local-constraint screenshots, plus all four final fixture theme captures, were inspected. Screenshots show composition/geometry; the draft's value and native lifecycle acceptance come from the real interactions and node/value assertions, not a screenshot or hoisted fixture.

Intermediate failed replays remain preserved and are not passes: popup translation still active, closed disclosure, pending Header exit, breakpoint-open precondition, agent-browser's held `Space` alias emitting an empty key, and virtual-only propagation. The final harness uses condition-based render waits and a literal held space, verified against native SUMMARY activation. The final fixture first stopped at a selected-Inspector theme-capture wait (`011-r2-fixture-final.log`). Its diagnostic trace (`011-r2-fixture-theme-trace.log`) shows the Inspector-requested pointer action actually dispatched a trusted click on Objects after the theme changed, and the Sheet correctly selected Objects. Capture selection now uses focus plus trusted Enter on the Inspector button, asserting focus identity first; existing pointer coverage remains. The diagnostic replay with extra snapshots (`011-r2-fixture-theme-diagnostic.log`) passed but is not substituted for a correction. No retries, sleeps, weakened assertions or domain setup bypasses were added.

Targeted Spec re-review must cover all three responses, the new optional Sheet props and cross-mode cleanup, and both approved summary hunks. Standards r1 approval covers only unchanged approved scope. Landing and all global gates listed above remain held/open; no new reviews were delegated by this child and no origin/dev push or feature landing occurred.
