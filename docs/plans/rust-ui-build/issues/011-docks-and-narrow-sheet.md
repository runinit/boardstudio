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

Implementation complete; the coordinator-owned Standards + Spec closure review is pending. Status stays claimed and landing is held until that review is approved.

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
