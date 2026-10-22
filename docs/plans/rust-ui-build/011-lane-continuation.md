# 011 safe handoff — 2026-10-22

Resume the completed implementation and verification from this checkpoint. The user stopped this lane for coordinator-owned T3 lineage migration. No closure review has been requested and nothing has landed. Ticket 011 remains `Status: claimed`, with acceptance boxes and Outcome still pending closure documentation/review.

## Resume sequence

1. Coordinator creates an app-owned `delegate_task` child from the coordinator thread in a new isolated bound checkout. Merge the checkpoint containing this file and latest **local** `feat/rust-ui-migration`; preserve the existing implementation rather than replaying it. Verify T3 binding, CWD, branch, base, inventory, and isolated target before any build. Preserve this original checkout, caches, and gitignored evidence.
2. Read the actual ticket `issues/011-docks-and-narrow-sheet.md`, `map.md` shared contract/lane protocol, linked migration spec, structure decision 011, ADR-0035, accessibility/theme rules, and applicable AGENTS instructions. Historical workflow documents are not operating instructions. Gortex is uninstalled: no calls, restoration, or requirements. Skip CodeGraph unless `.codegraph` exists (absent here).
3. Reuse the receipts below. Determine changes since the verified source pin before running affected checks; do not automatically repeat long builds. The last production source is `8978cd24d8b2758894d720342bac3bf749764f49`; through source/harness HEAD `18a43362b4b2027d8a082b2f3c3325d6d15efff1`, only `scripts/test-dock-layout.py` changed afterward.
4. Inspect the final settled production screenshots (latest captures have not yet had a fresh visual read), finish the ticket Outcome with literal evidence and limits, and create a clean final review pin. Reverify/merge latest local feature before final affected checks. Request **one real Standards + Spec closure review through coordinator** at that clean pin. No lane-created agents, threads, or watchers. Hold landing until substantive approval and coordinator authorization.

## Binding, commits, and inventory at handoff

- Original CWD/worktree: `/home/chris/.t3/worktrees/ts-boardstudio2/lane-011-docks-and-narrow-sheet`.
- T3 binding freshly verified attached to branch `lane/011-docks-and-narrow-sheet`; project root `/home/chris/01_Projects/ts-boardstudio2`.
- Lane thread: `mcp:12c4a7b3-edbf-47b9-9f2f-0907aa3279c4`; coordinator: `9b186519-9e71-4e26-a24b-6587e94c69e4`; 012: `mcp:287ab835-3b1a-497d-96e7-52c21bc33c9a`.
- Initial clean base `1620af409354adbec350b2fb8b8e03f70ca96585`; actual lane claim `ee81b5c01`.
- Behavioral RED checkpoint `52d4b6f28`; implementation `01f62eea3`; harness `945a5b4e9`.
- Local 013 feature `2de58e53aa5ca8a10d0223c2f32278ec42e3c623` merged without conflict at `3bc257e8271c8a7b23ae18131f5e8746370c308c`.
- Approved cleanup/focus harness `28443018bb46ede284956ec75c0c6f7a561708f8`; production final `8978cd24d8b2758894d720342bac3bf749764f49`; settled capture `4d7f2c283`; final Layout Add harness/source HEAD `18a43362b4b2027d8a082b2f3c3325d6d15efff1`.
- At capture, tracked dirty inventory **empty** and nonignored untracked inventory **empty**. This handoff file is the sole subsequent change. The handoff commit is discoverable with `git log -1 --format=%H -- docs/plans/rust-ui-build/011-lane-continuation.md` and is reported in the sending turn.
- Latest local feature freshly checked remains `2de58e53aa5ca8a10d0223c2f32278ec42e3c623`. No origin/dev push, reset, discard, history rewrite, or private visibility widening.
- Ignored inventory retained: `.claude/`, `.evidence/`, `.memsearch/`, `cad/.cache/`, `cad/scripts/__pycache__/`, `cad/wasm/pkg/`, `renderer/pkg/`, `scripts/__pycache__/`, `target`, generated `web/assets/{cad-worker,cad,core-worker,ergogen-models,renderer}/`, `web/assets/imported-modules.json`, `web/assets/tailwind.css`, `web/target/`.

## Scope and approved seams

011-only diff against merged feature (before this handoff):

- `web/crates/ui-shared/src/panels.rs`, `panels/policy.rs`, `panels_resize_tests.rs`, `panels_scroll_tests.rs`.
- `web/assets/docks.css`, `web/tailwind.css`, `web/crates/ui-shared/examples/dock_layout.rs`.
- `web/src/presentation.rs` reserved dock integration hunks only.
- Approved connected shared changes: `ui/button.rs`, `ui/sheet.rs`, `test_role.rs`, `ui/primitives_mount_tests.rs` under ui-shared; test-only `web/crates/layout/src/setup_guide/tests.rs`.
- `scripts/test-dock-layout.py`, `scripts/test-dock-primitives.py`, ticket 011 claim.

Implementation and ownership:

- Desktop Objects/Inspector are Sidenav cards with persisted per-side widths and pinned/collapsed mode. Legacy saved autohide migrates to pinned preserving widths; hover/timer/scrim behavior removed. Collapsed child subtree remains mounted under hidden/inert, rejects keyboard focus, and preserves local state on reopen. Legacy `m1-objects-panel`, `m1-inspector-panel`, and content IDs remain for existing focus callbacks.
- One nonmodal bottom Sheet below 980px across all six Workspaces, with named Objects/Inspector Tabs, bounded scrolling, visible interactive canvas, and one unique `workspace-panel-sheet` ID per document. Canvas child is stable across breakpoint changes; existing domain callbacks/subtrees retain ownership.
- App `CompactPanelState` / `CompactPanelControls` remain authoritative; Sheet open is a presentation adapter. The adapter effect reads `sheet_open` unconditionally to avoid the Escape/resize stale-subscription bug. No `workspace_composition` edits.
- Reserved presentation hunks: panels import; remove Editor track/style calculations; remove old CompactPanelScrim/LayoutPanelSheet; wrap existing Objects/Inspector in DockLayout while preserving canvas/domain callbacks; approved Case compact onfocusin close-handler removal; `is_compact_viewport` query `(width < 980px)` for existing browseParts/pinInspector callbacks; delete exactly three redundant `let objects_open = objects_open;` Copy captures. Callbacks otherwise unchanged and no lint weakening.
- Button approved optional `aria_expanded: Option<bool>`, `aria_controls: Option<String>`, `aria_haspopup: Option<String>` forwarded to both button/link branches; absent attributes remain absent. Shared mounted assertions cover both branches.
- Sheet approved optional mount-time `id: Option<String>` uses supplied ID or unchanged generated counter format. Existing focus/Escape/description/lifecycle behavior preserved, including 009 default callers.
- Test-only role helper correction skips aria-hidden=true descendants during fallback content text, preserving text-node whitespace/inline concatenation and explicit name precedence. Focused hidden-chevron/nested-text/simple/explicit-label cases pass. Layout setup-guide locator migration uses existing role/name helper and min980; production guide untouched, test name/assertions retained.
- Mobile return-focus candidates are actual opener plus controls whose real aria-controls relationship targets its containing dialog content ID. Candidate must be connected, visible, and outside hidden/inert/aria-hidden ancestors. Deliberate outside focus on external close is preserved. No guessed Header IDs, raw DOM mutation, Header/Popover edits, or bypass of nested preventDefault.

012 owns command palette module, WorkbenchApp palette state/shortcut, Header search/keyboard hints, and sibling palette render. Approved `ui::command` is being added by 012; 011 intentionally has **no dependency on that unlanded API**. Coordinator agreed 012 registers owner-local dock commands after 011 lands, exact hunk reviewed first. Authoritative private existing helpers: `toggle_compact_panel(state: CompactPanelState, side: PanelSide)` in panels.rs and `set_panel_mode(settings: Signal<PanelSettings>, mode: PanelMode)`. Desktop settings stay Editor-local, compact state stays App-owned; DockLayout accepts Objects/Inspector elements and children. Preserve widths, no second state or visibility widening; registration may need Editor-local callbacks rather than lifting settings into DockLayout.

013 landed code remains intact: tiny App -> NotificationProvider -> private WorkbenchApp wrapper preserves existing hooks/state/Editor lifetimes; notifications module, RuntimeNotifications/banner suppression, runtime typed queue/exact lease/export_delivery_errors, and export test adapters. No 011 diff on those owned paths. Coordinator's 12 dirty cleanup docs remain untouched.

## Evidence and results

All paths in this section are under the **original lane's** gitignored `.evidence/`; they are not copied into Git/new checkout. Long logs only; no target/source copies/manifests there. Source pin is stated where important.

Fresh final checks after merging 013 (commands serial, no shared-target overlap):

| Command | Result and receipt |
| --- | --- |
| `python3 scripts/build-web.py --fast` | PASS, `011-merged-build.log`. Fresh core worker, CAD worker, offline worker, renderer, CAD provider/cache path, Ergogen models and packaged Dioxus app. No skip-providers in this final build. |
| `BOARDSTUDIO_CHECK_JOBS=1 NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules python3 scripts/check.py browser --only ui-shared layout --receipt .evidence/011-merged-mounted-receipt.json` | PASS at clean production source `8978cd24`: 5/5 commands, **220 mounted tests** = shared65 + panel2 + layout152 + setup1, zero failed/ignored/incomplete. `011-merged-mounted.log`, `011-merged-mounted-receipt.json`, companion **`011-merged-mounted-receipt.json.md`**. Partial selection; page suite/provider smoke omitted. |
| `cargo test --locked -p boardstudio-web-ui-shared --lib` | PASS **15/15**, `011-merged-native.log`. |
| `python3 scripts/test-dock-layout.py` | PASS production-component fixture at source8978, `011-merged-fixture.log`. Six Workspaces, canvas input, ARIA/tab keyboard floor, migration/persistence/resize, hidden/inert retained local draft and focus, external-close focus preservation, nested Popover and mode/geometry boundaries. |
| `python3 scripts/test-dock-layout.py --page-url http://127.0.0.1:4211/boardstudio/` | PASS actual packaged app on final harness18a, `011-production-final.log` (earlier merged run `011-merged-production-browser.log`). Actual Case stage role/name; browseParts/pinInspector979/980 preserving saved widths; all Workspace Sheets and unique controls/content IDs; Layout Add create view and nested Escape desktop+narrow; mobile original Escape and retained-inert Close focus sequences. Browser errors `011-production-browser-errors.json`: empty. |
| `python3 scripts/test-dock-primitives.py` | PASS standalone 009 regression, `011-primitives-final.log` at source01f (later Sheet/default behavior unchanged; new shared props checked in merged mounted suite). Only obsolete saved-autohide expectation migrated to pinned. |
| `cargo clippy --locked -p boardstudio-web-ui-shared --lib --target wasm32-unknown-unknown -- -D warnings` | PASS, `011-closure-clippy-shared.log`. |
| `cargo clippy --locked -p boardstudio-web --bin boardstudio-web --target wasm32-unknown-unknown --no-default-features --features page -- -D warnings` | PASS, `011-closure-clippy-page.log`. |
| Targeted rustfmt, both Python scripts py_compile, git diff --check | PASS, `011-closure-rustfmt.log` (empty success), final diff checked before handoff. |
| Documentation links check | PASS 565 Markdown files **before adding this handoff**, `011-closure-doc-links.log`. |

Native name guard `011-native-name-guard.log` is scoped: actual executed15, matched11 baselined, missing0, four already-existing 001–009 tests new to baseline (three copy_rules and Sidenav resize). Baseline unchanged, no native test names added/removed/renamed by 011. The guard's generic “all2104 executed” output is misleading for scoped runs: **2104 tests were not run**.

Rendered captures: `011-production-desktop.png`, `011-production-narrow.png` (latest settled captures still need visual read); four fixture theme captures `011-narrow-{original,shadcn}-{light,dark}.png`. `011-design-detector-final.json` has no detector hits on panels/Button/Sheet; this is not visual, human, or screen-reader acceptance. Design context/guidance logs retained.

Meaningful REDs retained, followed by GREEN above:

- `011-browser-red-behavior.log`: old actual900px PCB Objects opener produced dialogs=[]; checkpoint52d4b6f2 before behavior fix.
- `011-boundary-diagnostic.log`: Escape then resize390 stale opener/Sheet state; fixed unconditional signal read, exact fixture sequence GREEN.
- `011-layout-mounted-initial.log` and receipt: ordinary layout152 PASS, setup-guide helper unwrap failed at line68 hidden chevron. `011-shared-mounted-initial.log` and receipt: new helper test first panic missing “4 Case (optional)”; later panic/timeouts were cascades. Minimal helper fix then merged220 GREEN.
- `011-mobile-opener-red.log`: original actual Workbench -> Objects -> Inspector -> Escape returned BODY; `011-mobile-control-relationship.log` proves real control relationship. Same normal/unmounted sequence now GREEN and unchanged in harness.
- `011-production-focus-red.log`: retained connected+inert exiting Header Popover consumes Escape until unmount. **Documented limitation**, not an Escape pass. User explicitly approved real Close panels click for this retained candidate test; returns focus to Workbench menu while opener is retained. Normal/unmounted Escape test remains separate and GREEN. Header/Popover/event ownership unchanged.
- `011-clippy-page-final.log`: exactly three redundant Copy capture bindings failed; approved exact deletions, final clippy GREEN without suppression.
- Early harness precondition/locator attempts retained as `011-production-regression-red*.log`; these are not counted as product behavioral REDs.

## Processes, target, and stop state

- `target` resolves to `/home/chris/.cache/boardstudio-target/011-docks-and-narrow-sheet`. Existing isolated target/settings and ignored protocol caches are preserved. No second worktree was made in this lane.
- No Cargo, wasm-pack, clippy, or test-dock command was running at final inventory. In-flight exec sessions **80313** (clippy) and **97305** (actual browser) were awaited to completion: both exit0. Earlier serial check session4222 had completed exit0. No command interrupted and no new build started for handoff.
- Own persistent static server intentionally retained: exec session **88365**, shell PID **2909738**, Python PID **2909753**, original lane CWD verified. Command `python3 scripts/serve-web.py web/target/site 4211 > .evidence/011-page-server.log 2>&1`; URL above. It serves the current generated artifact but is not a build/watch process.
- Own browser session intentionally retained: **`dock-011-page`**, agent-browser daemon PID **2910006**, browser PID **2910024**, original lane CWD verified. Its loaded page may predate final packaged rebuild; do not infer freshness from that tab. Automated test sessions close themselves. No other server/browser/process was touched.
- Do not run concurrent Cargo/wasm-pack on this target. Respect project jobs/settings and `BOARDSTUDIO_CHECK_JOBS=1`; no CARGO_BUILD_JOBS=2/CARGO_INCREMENTAL=0/debug overrides. New child must use its newly bound isolated target and protocol caches, not build concurrently here.

## Remaining work and gates

No unresolved check failure remains within the executed scope. The first actual failures and their fixes are listed above; there is no blanket full acceptance claim. Finish fresh screenshot inspection, ticket Outcome, latest-feature merge reconciliation and any affected checks, then the single coordinator-owned substantive Standards+Spec review. Landing remains held.

Global full/quick, CAD worker.onerror, first paint, screen reader, human026, and final merge gates remain **open**. No whole-app page suite/provider product-smoke clearance is implied by the mounted receipt or successful package build. Existing domain draft ownership/legacy focus IDs are preserved, but universal Workspace/domain draft and every focus route were not separately proven. Retained exiting Header Popover Escape consumption remains the documented limit above.

At user request, stop after committing this handoff. Do not start review, landing, agents, threads, watchers, or another long build from this original lane. Coordinator resumes through an app-owned child and merges these commits; leave original checkout/cache/evidence intact.
