# 012: Command palette

Status: resolved
Type: build
Blocked by: [004: Vendored Lucide icons](004-lucide-icons.md), [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [010: Header card](010-header-card.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [command palette](../../rust-ui-migration/command-palette.md), [palette decision](../../rust-ui-migration/issues/012-command-palette-contents.md), [shared contract](../map.md#shared-contract)

## What to build

Ctrl/Cmd+K opens a searchable palette of the workbench's existing actions and objects, as specified.

## Acceptance criteria

- [x] The palette follows the command palette spec: the groups, object search by name, hidden commands for other Workspaces, disabled-with-reason for unavailable ones, and no destructive or numeric edits
- [x] It opens from the header button and from Ctrl/Cmd+K anywhere, including inside a field, which blurs as on a click-away; Escape closes it first
- [x] Undo and redo show their existing hints; the Alt, Shift and Ctrl pointer modifiers are documented as `Kbd` hints in tooltips on the controls they affect
- [ ] Built on Dialog and Combobox behaviour with no script; it passes the floor; every command dispatches the same event or UI change as the control it mirrors
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

**2026-10-22 continuation: candidate verification; claimed and landing HOLD.**
This is not closure approval. The approved implementation and original receipts
are preserved in [the continuation record](../012-lane-continuation.md).

### Binding and source

- Authoritative continuation checkout:
  `/home/chris/.t3/worktrees/ts-boardstudio2/t3-7bb7f80c`, branch
  `fix/command-palette-space-callback`. T3 binding and CWD matched; initial
  tracked/untracked inventory was clean at `b237903705244a266b07a64e5f23b8a3b5219c03`.
- Fast-forwarded committed handoff `331fd70fcf92e3b66ec9274e970dc57a8411f869`.
  Local feature `2de58e53aa5ca8a10d0223c2f32278ec42e3c623` was already its ancestor;
  no conflicts, implementation replay, source copies or coordinator WIP imports.
- Isolated target: `/home/chris/.cache/boardstudio-target/012-command-palette-lineage`.
  Copied the absent protocol settings and ignored CAD caches before building.
  The original lane, cache and PID 2849824 / port 4212 server remain untouched.
- Source/harness candidate: `1c700e2125a52c68e633439d309b00db98937aa6`.
  Regression checkpoint `d3b015c6ff1b24b6ac04ba0965a103c3369ba698`;
  callback fix `05ca48873c4a4b29ece9696aa11bfd2f31693c27`;
  Library tests `d2e5816f44977eb2c71b692917e50797db8b5412`;
  private storage alias `e551568b18d16cc6d2860551950f7c98318b7505`.
- Review preview retained at `http://127.0.0.1:4222/`, PID 3073160;
  `server.log` records serving. All owned browser/check/build commands completed.

### Changes and behavior

The palette keeps the approved owner-local callbacks, command groups, unavailable
reasons, shortcut hints, Dialog/Combobox behavior and scope/token/snapshot registry
guard. Registration refresh, unavailable/reordered snapshots, unregister and
remount rejection remain covered. Library requests carry the project identity,
consume once and retain the existing draft, blur, cancel and focus ownership.
Export commands reuse the row callback and its live owner/provider blockers.
013's NotificationProvider/WorkbenchApp, runtime queue and test adapters are intact.

The controlled disclosure now forwards its caller's `onkeydown` exactly once for
Space before the existing Dioxus/raw propagation stop. Enter still bubbles to
the wrapper listener once; Space remains unprevented for native keyup activation.
The uncontrolled Accordion branch and canvas listener are unchanged. The registry
storage received a private type alias to satisfy Clippy, with no contract or
visibility change, new dependency or lint suppression.

The production harness now uses trusted held-Space input on its own agent-browser
session through Node's built-in CDP WebSocket, verifies real matrix/key/part/outline
Inspector outcomes, and checks Add/Align/Transform tool opening, both shortcuts,
four Theme/Appearance combinations and narrow bounds. It creates its matrix
through the existing UI; built-in demos contain standalone parts, not matrices.

### Executed verification

Logs below are in this checkout's `.evidence/012-lineage/` unless stated otherwise.
No Cargo/wasm-pack builds overlapped against the target; the supported scheduler
used `BOARDSTUDIO_CHECK_JOBS=1`, and direct browser runs used the documented
`CARGO_PROFILE_DEV_DEBUG=false`.

| Command/check | Result and limits |
| --- | --- |
| Focused `wasm-pack test --headless --chrome web/crates/ui-shared --locked --lib -- ui::command::tests::controlled_space_preserves_caller_once_without_consuming_native_activation` before the fix | Actual RED, exit1: controlled Space expected `["controlled: "]`, received `[]`; `space-caller-red-2.log`. The earlier missing fixture type annotation was a compile diagnostic, not behavioral RED. |
| Same package, filter `ui::command::tests::` after the fix | PASS 3/3, exit0; `command-green.log`. Direct callback/ancestor counts for controlled and uncontrolled Space/Enter, no controlled default prevention; registration lifecycle and disclosure owner tests. Trusted native activation is a separate production receipt. |
| Focused Library project-tool test | PASS 1/1, exit0; `library-tools-2.log`: queued rename after a project switch is rejected; current project focus and consume/no-replay for rename and demo survive remount. Initial fixture callback feedback caused a 119.8s WebDriver timeout, not a behavioral RED; stable fixture callbacks corrected it without changing assertions. |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py browser --only ui-shared library layout case` | PASS 322 total (67+2+18+82+152+1), exit0; `affected-browser.log`. Includes separate panel/setup batches and the supported Layout CSS build. Partial subset, not the full browser gate. |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py lint typecheck` | PASS, exit0; `lint-typecheck-2.log`. Initial run stopped at the registry storage type's Clippy complexity diagnostic; private alias fixed it. |
| `cargo test --locked -p boardstudio-web-ui-shared -p boardstudio-web-ui-model -p boardstudio-web-library -p boardstudio-web-layout -p boardstudio-web-case`, then `check_native_tests.verify(..., full=False)` | PASS 146 native tests and partial name guard, exit0; `native-affected.log`. One pre-existing Layout archive-fixture test and two doctests remain ignored. Four new native names were reported from the incoming feature; no native tests were renamed/removed here and the baseline was left unchanged. The guard's printed 2104 figure is the global baseline size, not this subset's executed count. |
| `python3 scripts/run-wasm-tests.py --files web/src/presentation/export_workspace.rs web/src/presentation/zmk_firmware_export.rs --result-json web/target/test-results/palette-export-final.json` | PASS 6/6, explicit `PAGE_RUN_EXIT=0`; `page-export-final.log` and structured complete report. Includes the new direct row/palette callback counter and all four 013 export tests. Initial run recorded six passed/complete but its terminal session returned143; preserved as `page-export.log`/`palette-export.json`, not reused as a clean command exit. |
| `python3 scripts/build-web.py --fast --routes subpath,root` | PASS, exit0; `build-both-routes.log`. Worker/offline/renderer stages ran; supported CAD cache validation reported its package current and skipped recompilation. Both routes packaged. |
| `python3 scripts/build-web.py --fast --skip-providers --routes subpath,root` at committed source/harness `1c700e2125a52c68e633439d309b00db98937aa6` | PASS, exit0; `final-build-both-routes.log`. Rebuilt both page routes at the exact source pin, reusing the provider stage above; no provider source changed. |
| `python3 scripts/test-command-palette.py http://127.0.0.1:4222/ --evidence .evidence/012-lineage/production-root-final` | PASS, exit0; `production-root-final.log`. The complete expanded production harness against the root package from the source-pin build. |
| Same production harness at `http://127.0.0.1:4222/boardstudio/`, evidence `production-subpath-final` | PASS, exit0; `production-subpath-final.log`. Complete expanded harness against the subpath package from the same source-pin build. |
| `python3 scripts/test-command-palette.py http://127.0.0.1:4222/boardstudio/ --evidence .evidence/012-lineage/production-subpath-4` | PASS, exit0; `production-subpath-4.log`. Trusted Snap Enter/Space each toggle once with focus retained; trusted held-Space pan changes the canvas camera while preserving Snap; Space types in the rename field. Covers real matrix/key/part/outline selection, native tools, unavailable reason/no-dispatch, project rename/demo, ZMK blocker, modal focus/ARIA, Ctrl/Cmd+K, four Theme/Appearance combinations and 375px bounds. |
| Production driver corrections | `production-subpath-1.log`: CLI box output was text, not JSON. `-2.log`: demo lacked matrices. `-3.log`: generic no-dialog wait also matched the correctly opened native Add dialog. Driver/fixture diagnostics were corrected using semantic locators and actual palette unmount; no behavior assertion was removed or sleeps/retries added to force a pass. |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py repo`; formatting, Python compilation, diff read/check | PASS. Repository logs `repo.log`, `repo-final.log` and `repo-outcome.log`. |

Screenshots from `production-subpath-4/`: `palette-open.png`,
`palette-original-light.png`, `palette-original-dark.png`,
`palette-shadcn-light.png`, `palette-shadcn-dark.png`, `palette-narrow.png`.
Desktop, shadcn-dark and narrow screenshots were inspected; this is not human026
or screen-reader clearance. Original lane `green-4` and earlier receipts were
read directly as historical evidence, not relabeled fresh final acceptance.

### Blocked integration and remaining gates

- **Header RED preserved:** unchanged `scripts/test-header.py` fails at line 220
  selecting Original because `div#toast-dxc-275-label` covers the menu item.
  At 1440×960, thirteen stacked 013 notifications overlap Settings. Exact receipt:
  `.evidence/012-lineage/header.log`; screenshot
  `.evidence/012-lineage/header/failure.png`; DOM evidence
  `.evidence/012-lineage/header/failure-dom.txt`. No dismissal, retry, assertion
  weakening, notification fix or header workaround was applied. Coordinator
  assigned `rust-ui013-notification-stack-interoperability-sol-r1`; header
  integration remains blocked until that fix lands.
- **011 not yet landed at this verification pin.** Approved Objects/Inspector
  commands and Search popup ARIA remain pending its feature landing; do not
  compete in dock/Button/Sheet ownership. Required post-landing 979/980, current
  viewport dispatch, opposite compact side and desktop width/mode checks remain.
- Source-pin rebuild and both route production receipts passed. Final local
  feature freshness check still returned
  `2de58e53aa5ca8a10d0223c2f32278ec42e3c623`. Before closure, merge the latest
  landed feature changes, run affected checks and update this record with their
  exact pins/results.
- No agents, ordinary threads, watchers, closure reviews, local feature landing
  or remote pushes were started by this continuation. Request **one** combined
  Standards+Spec closure review through the coordinator at the clean completed
  pin. Keep the ticket claimed and landing HOLD until substantive approval.
- Global full/quick, CAD `worker.onerror`, first-paint, screen-reader, human026
  and final-merge gates remain open.

### Landed 013 stack integration and compact history correction

**2026-10-22 follow-up: Header integration GREEN; ticket still claimed and landing HOLD.**
This section supersedes the notification integration blocker above, preserving
both historical RED receipts. T3 binding, CWD, branch and clean inventory matched
`ca00747d1dfb95a8583e62b3738f7e8199ef355d` before integration; the isolated target
and retained port 4222 preview were unchanged.

Merged local feature `1a692e0eece287f094eafaf7e395920c18585f73` at
`c0f7fd11efef7108bfdc2eba6c73b789040b50a9`, without conflicts. Its four incoming
013 files (notification provider, scoped CSS, toast production driver and ticket)
had no overlap with 012 changes and remain identical to the landed feature.
No notification/runtime/test fix was made here. The coordinator reported both
review axes approved for that incoming slice; this is not a review of 012.

The first unchanged Header run against that merge cleared the original toast
obstruction, then failed with **`Control overflow: Edit history`** at 1024x960,
shadcn light, with the compact menu open. The source diff attributes the increased
row width to 012's Undo/Redo `Kbd` hints. The history flex row did not wrap.
Actual new RED: `.evidence/012-lineage/post-toast/header.log`, exit1; screenshot
`post-toast/header/failure.png` and `failure-dom.txt`. Original
`.evidence/012-lineage/header.log` and `header/failure.png` remain untouched.

Source pin `2679e5d1e872cc3c8af6d4629a084de22d545434` adds one scoped rule in
`web/assets/header.css`: `.header-mobile-menu .header-history { flex-wrap: wrap; }`.
It preserves both hints, callbacks and desktop layout. No Header or palette
driver changes, notification dismissals, retries without a source correction,
or assertion workarounds were used.

Fresh logs below are under `.evidence/012-lineage/post-toast/`:

| Command/check | Result and limits |
| --- | --- |
| `python3 scripts/build-web.py --fast --skip-providers --routes subpath,root` at merge `c0f7fd11efef7108bfdc2eba6c73b789040b50a9` | PASS, exit0; `build.log`. Reuses the earlier validated providers; both routes rebuilt. |
| Unchanged `scripts/test-command-palette.py` at root and subpath against that build | PASS, both exit0; `palette-root.log`, `palette-subpath.log`. Complete expanded harness; superseded by the final source-pin runs below. |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py lint repo` at the merge | PASS, exit0; `lint-repo.log`. |
| `python3 scripts/run-wasm-tests.py --files web/src/presentation/export_workspace.rs web/src/presentation/zmk_firmware_export.rs --result-json web/target/test-results/palette-export-post-toast.json` | BLOCKED, explicit `PAGE_RUN_EXIT=1`; `export.log`, structured report `complete: false`, zero executed. `/usr/bin/chromedriver` cannot load `libsimdutf.so.36`; its startup failure is not behavioral evidence. No host packages or driver configuration changed. |
| Same both-route build command at source pin `2679e5d1e872cc3c8af6d4629a084de22d545434` | PASS, exit0; `history-build.log`. Both page routes rebuilt, providers reused unchanged. |
| `python3 scripts/test-header.py http://127.0.0.1:4222/boardstudio/ --evidence .evidence/012-lineage/post-toast/history-header-green` | PASS, exit0; `history-header-green.log`. Unchanged Header driver completes real shell actions, Settings interaction, compact focus/lifecycle and all sixteen Theme/Appearance/width geometry cases. Its inherited summary still says "inert Search"; active palette behavior is established by the separate 012 harness. |
| `python3 scripts/test-command-palette.py http://127.0.0.1:4222/boardstudio/ --evidence .evidence/012-lineage/post-toast/history-palette-subpath` | PASS, exit0; `history-palette-subpath.log`. Complete expanded harness at the final source pin. |
| Same palette driver at `http://127.0.0.1:4222/`, evidence `post-toast/history-palette-root` | PASS, exit0; `history-palette-root.log`. Complete expanded harness at the final source pin. |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py lint repo` at the final source pin | PASS, exit0; `history-lint-repo.log`. Old-CSS count remains informational. |

Inspected `history-header-green/shadcn-light-1024.png`: both history buttons and
their hints fit on separate rows. Existing 013 stack behavior and its sixteen
burst-case receipts are reused from the landed 013 Outcome; its files are
unchanged here. Export source files also match `ca00747` exactly: reuse the
earlier six-test `page-export-final.log` / `palette-export-final.json` receipt
with explicit exit0, while retaining the fresh driver failure separately.
No fresh mounted Export pass is claimed. The earlier native/affected-browser
receipts retain their stated source and subset limits; no global gate is inferred.

Latest local feature remained `1a692e0eece287f094eafaf7e395920c18585f73` after
these checks. **011 landing and its approved Objects/Inspector commands plus
Header Search ARIA remain pending**, with no competing edits. One combined
Standards+Spec closure review must still run through the coordinator at a clean
completed pin after that integration. No review was self-delegated; no agents,
threads, watchers, feature landing, origin/dev push, visibility widening or new
dependency. Global full/quick, CAD `worker.onerror`, first-paint, screen-reader,
human026 and final-merge gates remain open.

### Landed 011 integration and final owner handoffs

**2026-10-22: continuation candidate; claimed, closure review and landing HOLD.**
This section supersedes the pending 011 and driver blockers above. The original
RED images/logs remain unchanged. T3 binding, CWD, branch and clean inventory
matched `c617b4e1a53f02fd8a6e2e4cc8b91f61972f559f`; the isolated lineage target
and retained port 4222 server are unchanged. Merged local feature
`a1828bb3aa15f46a45b355880ba5e31ab0aa9262` at
`add2427f17b1f6763993e833c92b51c071e4714d`, with no conflicts. The incoming
011 persistent dock subtrees, Sheet props, Button ARIA, summary Space handler
and test-role helper, and 013 notification/CSS/runtime ownership are preserved.
The original lane and its server were not edited or restarted.

Approved integration changes:

- Editor appends owner-local View commands using the current compact signals
  and desktop `PanelSettings`. At dispatch, width <980 toggles the requested
  side and closes the opposite side; desktop changes only mode, retaining width.
  Stable `use_callback` handles preserve the selected View command across the
  979/980 resize without relaxing registry admission. The approved Add adapter
  reveals Objects before the unchanged `add_open.set(true)`, using the same
  current-viewport state and preserving the desktop width.
- Header Search uses 011's Button ARIA props and a `ReadSignal<bool>` prop.
  Header owns the subscription; App does not read palette-open state to refresh
  its navigation commands. Native Header Undo/Redo close the compact menu before
  invoking their unchanged owner callback. Hints, desktop behavior and history
  ownership remain intact.
- `scripts/test-header.py` resolves Undo/Redo from the Header-scoped semantic
  snapshot, asserts one matching button and validates its ref metadata/name.
  The global snapshot contains duplicate footer controls, including an inert,
  covered footer Undo. No global-first/nth locator or UI dismissal is used.
  All original native history outcomes remain asserted.
- Snap settings registration moved from `LayoutSnapDock` into the existing
  `LayoutCommandPill` command block, using its existing `open_menu` owner.
  Both presentations share identical private settings; Dock adds disclosure
  and the current increment. The two approved caller hunks are included here:
  `layout_workspace.rs` sets `snap_in_dock: false`; `presentation.rs` removes
  only the redundant Objects `LayoutSnapDock` mount. Component code stays for
  later cleanup. There is one visible canvas trigger, popup and command owner.
- Only the Snap command calls the existing synchronous
  `restore_layout_command_focus(menu)` after setting `open_menu`. The persistent
  native trigger is the canonical command target; no DOM activation or delayed
  focus task was added. The Pill root stops virtual/raw propagation only for
  Space whose raw target has that trigger's canonical ID. It does not prevent
  default, overwrite another caller handler, or change canvas handling.
- `PaletteState.show` captures a public Popover relationship before focusout can
  unmount content. For an active element inside the nearest actual Popover
  content, it resolves that content ID to one connected, visible, focusable
  `aria-controls` trigger, excluding disabled/hidden/inert candidates. Ambiguous
  or invalid relationships fall back to the original active element. The
  original element still owns field blur/commit; `finish_close` guards remain
  unchanged. No Layout-specific IDs, state, API, timer or Popover change.
  The browser DOM fixture proves nearest nested relationship selection and
  ambiguous/invalid rejection. This is capture-time normalization: if that
  trigger later unmounts too, the existing connected-element guard and fallback
  still apply; no recursive popup-owner walk or new focus policy is introduced.
- Rename's existing ProjectHeaderMenu callback is memoized only by captured
  project ID; the original latest-runtime ID check, request and browser body
  remain. Unrelated renders retain identity, while a project change replaces
  it and cannot retarget an old snapshot. Open Demo uses an unconditional stable
  callback. Both hooks run in the same order for project None/Some; no registry
  guard, domain state or visibility change. The mounted test retains consume
  once, current-project checks and no replay on Library remount.

Product checkpoints: View/Search/Snap ownership `5f06c7a4829a8309d13bd8ba2b092e8e5ce6f816`;
Header callback correction `840e91201014dd1ace1e4253f70dfe4cd5dcb271`;
View identity/Add reveal `6f2629515de2633f3f23cfa454614e498557dd8f`;
Snap handoff `db14ba0ad109ac1e37323a7223a4e8522e79e28d`;
native Space guard `3be06eee4782c2a6d1422eb2232b4cb54696a7ac`;
Popover opener `fc118b020f31a38b5e3d0d7bc6c3eb11387c00e3`;
Library callback identity `11e025a0e681fd9f06707ac617a1a2c5537d926c`.
The document-teardown/remount harness addition is `a0b79b1834c37213041759fba01917e56fcd4fa8`.

#### Regression attribution

All logs below are under `.evidence/012-lineage/post-dock/`.

| Receipt | Actual finding or diagnostic limit |
| --- | --- |
| `view-red.log`, `search-aria-red.log` | Missing View commands and missing Search popup ARIA before integration. |
| `candidate-palette.log` | Navigation failed when App subscribed to palette-open; Header-local ReadSignal fixes that ownership. |
| `candidate-header.log`, `header-diagnostic/` | Original global locator selected the covered footer Undo. Header-scoped snapshot contains one visible intended pair. The first scoped-ref implementation incorrectly used accumulated global refs; corrected to parse the scoped snapshot text. |
| `header-3.log`, `header-3/failure.png` | Correct native Header Undo changed the project name but left its mobile menu open. The two approved native callbacks now close it. |
| `palette-3.log`, `view-diagnostic-2.log` | Actual first View activation at979 closed the palette without opening Objects. Synchronous marker: same owner, changed callback (`identity=true command=false`). Stable View handles fix this. The earlier asynchronous diagnostic changed timing and is not GREEN. |
| `view-diagnostic.log` | Add opened in hidden Objects when desktop mode was Collapsed. The approved reveal adapter preserves width and authoritative compact ownership. |
| `snap-focus-diagnostic.log` and image | Actual command-open Snap was visible while focus remained BODY. Canonical synchronous handoff fixes this bounded command-target requirement. |
| `snap-space-red.log` | Trusted Enter produced click1; Space keydown/keyup produced no click2 because Layout pan prevented default. The scoped root guard preserves native keyup activation. |
| `snap-space-green-subpath.log`, `complete-subpath-2.log`, `snap-palette-escape-diagnostic.log` | Harness assumptions, not product fixes: a later native reopen may focus inside the actual popup; CtrlK focusout dismisses native Snap before query/Enter/Escape. Old Accordion persistence expectations were invalid. Preserve the image/trace; remove only Escape on an already closed Snap. Actual compact Inspector Sheet now provides the stronger Escape-first/same-owner test. |
| `complete-subpath-3.log`, `snap-detached-opener-diagnostic.log` | Actual CtrlK from Close Snap captured a later-detached child; Escape returned BODY. Public-ARIA capture-time normalization fixes that opener lifetime. First command handoff and Snap Escape remain strict canonical-target assertions. |
| `opener-root.log`, `opener-subpath.log`, `rename-diagnostic.log:1491` | Both routes passed the new Snap focus checks and canvas pan, then Rename closed without opening Library. Synchronous diagnostic `identity=true command=false` confirmed callback refresh under the same owner. Neither partial run is a complete production pass. |
| `rename-refresh-red-2.log` | Mounted captured Rename after an unrelated same-project refresh failed the expected pending-request condition in5000ms. Initial attempt used the page-test runner for a child crate; its selection error is not behavioral RED. |
| Earlier build/driver diagnostics | The mutable binding error after011 and optional owned String prop error were compile failures, not behavioral proof. Old WebDriver missing-loader failure is resolved by the coordinator's matching155 driver repair; its receipt is retained at `driver-repair/run-receipt.json`. |

All temporary Rust console/Reflect markers and diagnostic-only driver reads were
removed. Test-only trusted-key event records remain to assert target, prevention
and exact activation counts; they do not alter behavior.

#### Verification and limits

The first final replay at `fc118b02` rebuilt both routes (`opener-build.log`,
exit0), passed Header (`closure-header.log`, exit0) and seven page browser tests
(`closure-page.log`, `closure-page-results.json`: complete,7passed,0failed,
exit0). Those page tests cover public relationship resolution plus all six
Export tests; no missing-driver blocker remains. The complete production
replays stopped at the newly discovered Rename failure, as recorded above.
Final nondiagnostic receipts after the Library fix follow.

Final product source is `11e025a0e681fd9f06707ac617a1a2c5537d926c`;
final production harness is `5b2edfc07e55ae7402e22d086ade4cf758cb9e30`.
Later Outcome commits change documentation only. No diagnostic instrumentation
is present in this source or these replays.

| Final command/check | Result and receipt |
| --- | --- |
| `CARGO_PROFILE_DEV_DEBUG=false wasm-pack test --headless --chrome web/crates/library --locked --lib -- library::mounted_tests::palette_project_tools_consume_once_and_reject_rename_after_project_switch` | PASS1/1, exit0; `rename-refresh-green.log`. One captured Rename invocation queues its request after same-project refresh; the old snapshot queues nothing after project switch. Current-project focus, consumption/remount and captured Demo after refresh pass. |
| `python3 scripts/build-web.py --fast --skip-providers --routes subpath,root` | PASS, exit0; `closure-build.log`. Both routes rebuilt at the final product source; unchanged validated providers reused. |
| `python3 scripts/test-command-palette.py http://127.0.0.1:4222/ --evidence .evidence/012-lineage/post-dock/closure-root` | PASS, exit0; `closure-root.log`. Complete nondiagnostic production driver, including native trusted actions and document teardown/reopen. |
| Same driver at `http://127.0.0.1:4222/boardstudio/`, evidence `post-dock/closure-subpath` | PASS, exit0; `closure-subpath.log`. Same complete assertions on the subpath package. |
| `python3 scripts/test-header.py http://127.0.0.1:4222/boardstudio/ --evidence .evidence/012-lineage/post-dock/owner-header` | PASS, exit0; `owner-header.log`. Actual Header Undo/Redo, project history, compact menu close/focus, retained inert content, desktop behavior and all16 geometry cases. |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py browser --only library` | PASS18/18, exit0; `closure-library-browser.log`. Scoped Library browser suite, not a global browser gate. |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py lint typecheck repo` | PASS, exit0; `owner-lint-typecheck-repo.log`. Native/wasm Clippy, page typecheck and repository checks. Old CSS count remains informational. |
| `CARGO_PROFILE_DEV_DEBUG=false python3 scripts/run-wasm-tests.py --files web/src/presentation/command_palette.rs web/src/presentation/export_workspace.rs web/src/presentation/zmk_firmware_export.rs --result-json .evidence/012-lineage/post-dock/owner-page-results.json` | PASS7/7, exit0; `owner-page.log`, complete structured report `owner-page-results.json`,0failed/0incomplete. Public Popover relationship test and all six Export tests at the final product source. |

The full route runs verify canonical Snap handoff; Tab into actual settings;
increment, Geometry and gap callbacks; Escape/reopen; one command/trigger/popup;
native Enter and three held Space phases (no activation while held, default not
prevented, exactly one trusted keyup click per phase); canvas pan outside and
field Space typing. They also exercise selected View dispatch across979/980,
correct compact sides/opposite close, desktop toggles and persisted widths,
Add reveal/cancel/focus, the SAME Inspector Sheet across palette open/Escape,
strict Snap-content Escape focus, outside Project focus, normal field blur
commit/Escape cancel, Rename/Demo, object selection and existing tools/Export
blockers. Document teardown is a full navigation away with a palette open,
then return to the same packaged route: closed state, one reopened palette and
normal Escape. Component-scope unregister/stale-owner behavior remains the
separate registry browser test; document teardown does not stand in for it.
Inspected final `closure-subpath/palette-open.png` and `palette-narrow.png`;
both route runs retain four Theme/Appearance screenshots. This is bounded
visual inspection, not human026 or screen-reader acceptance.

The final freshness check still returned local feature
`a1828bb3aa15f46a45b355880ba5e31ab0aa9262`. Exact diffs against it remain empty
for incoming `panels.rs`, `ui/button.rs`, `ui/sheet.rs`, `test_role.rs`,
`web/tailwind.css` and `presentation/notifications.rs`. Formatting, Python
compilation and diff checks passed. The Outcome-only repository check is logged
as `closure-outcome-repo.log`; no Cargo/wasm-pack runs overlapped on this target.

Reused scoped evidence is explicit: `affected-browser.log` ran223 ui-shared/Layout
browser tests after011 integration (68+2+152+1), including registry refresh,
unavailable/reordered snapshots, unregister/remount and controlled/uncontrolled
Space behavior. `dock-production.log` passed the landed011 production driver,
including six workspaces, persistent drafts, widths979/980 and canvas pan.
Earlier146 native and322 affected-browser receipts retain their original pins
and subset limits. Later Snap/opening behavior is checked by the final production
harness rather than relabeling these older receipts as fresh. No provider source
changed; page rebuilds reuse the validated provider stage recorded above.

The [accessibility floor](../../rust-ui-migration/accessibility-floor.md) records
Popover focus moving inside and Tab remaining inside (§2), plus overlay Escape,
focus return and ARIA requirements (§3); action menus require entry focus.
Legacy `BoardPopover` is non-modal and its initial-focus timing does not satisfy
that full entry-focus claim. Canonical command-target focus plus Tab to Close
Snap is **bounded 012 evidence, not full Popover-floor acceptance**. The approved
native-reopen trigger-or-popup expectation explicitly rejects BODY/outside focus;
it does not weaken the first command handoff or canonical Escape assertions.
015 owns the new popup-entry adapter and broader native-button Space guard.
Its unlanded markup was not copied. It must preserve 012 callbacks, registration,
canonical ID and the Snap predicate in one composed root handler; final merged
validation must cover command and toolbar keyboard/actions, popup-entry focus
and the applicable floor. This gap remains explicit for Spec review.

No agents, threads, watchers, local feature landing or remote push were created.
One complete Standards+Spec review must run through the coordinator at a clean
completed pin. Ticket stays claimed and landing HOLD until substantive approval.
Global full/quick, CAD `worker.onerror`, first-paint, screen-reader, human026 and
final-merge gates remain open; no scoped receipt clears them.

### Spec correction: palette above native modal confirmations

**Correction candidate; claimed and landing HOLD.** The coordinator reported
Standards APPROVE with0 findings across all22 paths at
`e974b961a74436540255fb60e5d1822c49863925`, and Spec CHANGES with one P2:
the div-based palette could render behind Library's native Delete confirmation.
That finding was source-inferred; the correction first reproduced it with the
actual trusted browser path. Original pin and first review receipts are retained;
their approval does not cover this new host. A completed correction slice needs
targeted Spec and explicit Standards coverage through the coordinator.

The correction changes only palette presentation, owned CSS and its harnesses:

- A palette-local native `<dialog>` uses public `show_modal` / `close` to enter
  the top layer above existing native modal confirmations. It keeps the existing
  `command-palette` ID, so Header's public ARIA target remains the actual popup.
  The native host is the sole named semantic dialog, labelled/described by the
  existing Title/Description components through their public ID props. The inner
  copied Dialog uses public `is_modal:false`, `role:presentation` and
  `aria_modal:None`; its structural content is unfocusable. Actual product AX
  output is checked for exactly one exposed dialog named Command palette.
- The host DOM handle is a local lifecycle resource, not new modal/domain state.
  Existing palette open/opener/pending signals, query, registry admission and
  command owners remain unchanged. The normal effect and public content mount
  event synchronize the same connected host; showing waits for its actual
  mounted input. Content-node identity is checked before the mount callback
  acts. No timer, retry or shared primitive/API change. Current-open sync focuses
  the actual input for both a newly shown host and reentry into a retained host.
  It runs from the open effect/current mount event, without a query subscription
  or refresh loop. The mounted test also covers rapid close/show of the same host.
- The native host remains modal through the copied Dialog's retained exit.
  Existing `PaletteLifetime` closes the current host synchronously before the
  unchanged `finish_close` handoff. Current open state prevents an old exit from
  closing a reopened palette. Unmount retires the owned DOM handle and closes
  its connected open host. A mounted browser test removes an open owner, proves
  modal/inert release and no late focus steal, then mounts a distinct new host
  and closes/cleans it normally.
- Native cancel prevents default/propagation and closes only the palette, with
  exact event-target/current-host, connected/open and current palette guards.
  The native Delete confirmation remains the same owner and DOM node throughout
  palette open/Escape, with no native cancel/close event delivered to it.
- Chromium's native Shift+Tab can escape the first control to BODY. The bounded
  host handler wraps only first/last Tab boundaries, including single/empty
  content. It uses current enabled, visible, non-inert tabbables in the current
  content, checks raw target/active element and mounted modal identity, and
  preserves already-prevented or modified Tab and every other key. Native
  movement handles ordinary Tab. The copied primitive trap stays disabled;
  no second trap or global keyboard handler was added. The existing upstream
  focus helper is private `pub(crate)` JavaScript, so it was not exposed/reused.
  Current palette controls use ordinary zero/negative tab indices.
- Owned tests now assert native host `[open]` state; geometry still measures the
  actual copied panel. Header's one palette-close wait follows that same native
  state. Role/name actions and all original behavior outcomes remain. The actual
  destructive confirmation click is resolved uniquely from its native dialog's
  labelled ancestry snapshot, with button-role/name/ref assertions; it cannot
  select the identically named background Library button. Only the test-created
  keyboard in the isolated browser is deleted, after explicit native Confirm.

No Library, BoardDialog, shared primitive, Header production, registry, domain
callback, dependency or visibility changes. 011/013 and the approved Snap owner,
callbacks and Space predicate remain intact. Native Snap entry-focus/floor and
015's broader composed toolbar Space guard remain the previously stated combined
acceptance gap, unchanged by this correction.

| Preserved regression/probe | Result and attribution |
| --- | --- |
| `native-modal-red.log` | Fixture setup error: no Library card was selected. Not behavioral RED. |
| `native-modal-red-2.log`, `native-modal-red-2/native-confirmation-palette.png`, checkpoint `bb4d4fb8` | Actual trusted CtrlK from existing Delete: native modal stays open, active element remains its Cancel button, palette appears behind it; Search-focus assertion fails. Confirms the P2. |
| `native-host-platform-probe.log` | Isolated blank-document platform probe, not product GREEN: a native host with presentation role still exposes an outer dialog; native ShiftTab can reach BODY. Both conflicts informed the corrected local host. |
| `native-host-tab-red.log`, `native-host-initial-focus-diagnostic.log` | Actual first host candidate focused an empty native host before animated content mounted. Public mounted-content synchronization corrects the ordering. The intervening E0599 for a direct `onmounted` builder was compile-only; the existing public attribute vector is used instead. |
| `native-host-tab-red-2.log`, checkpoint `882a90db` | Initial focus worked, then Browse projects closed without showing Library. Keeping the native host through the existing content-exit/dispatch boundary restored this outcome. No claim about a registry marker was inferred, and the guard was unchanged. |
| `native-host-tab-red-3.log`, `native-host-tab-red-3/native-confirmation-palette-ax.txt`, `native-confirmation-shift-tab.png`, checkpoint `2c1cfad0` | Actual product Search focus and one-dialog AX passed above Delete; trusted ShiftTab then left to BODY and failed containment. No boundary guard existed for this RED. |
| `native-host-lifecycle.log` / `native-host-lifecycle-results.json` | PASS2/2, exit0 at the pre-boundary checkpoint: existing public relationship test plus native owner unmount/remount/inert/focus lifecycle. Later final page results include the additional boundary cases. |
| `native-host-final-root.log`, `native-host-final-subpath.log` | Harness misexpectation after the correct native Confirm: deleting the last project creates a distinct durable replacement with the same Untitled keyboard name. Existing `runtime.rs`/Library mounted tests establish that behavior. The assertion now requires original project-ID removal, a different replacement ID and one explicit trusted confirmation click; cancellation must retain the original ID. No product deletion fix. |
| `native-host-complete-root.log`, `native-host-complete-subpath.log` | Both routes passed the native sequence and old behavior through themes/narrow bounds, then rapid close/reopen before teardown failed initial Search focus. A retained native host does not rerun `show_modal` entry focus. The local current-open sync now restores its real input; opener/pending/field blur and stale admission remain unchanged. This failure is retained as a product host regression, not relabelled a complete pass. |

Evidence paths in this section are relative to `.evidence/012-lineage/`.
Final product source is `9af2a38a622745314ff9a953aa628d9ce32acd22`;
final production harness is `4028631b7bf72cdac9544c5dcaa81b5c77b0f7ee`.
`d36069db` is the earlier boundary-guard checkpoint. Later Outcome commits are
documentation only. No temporary production markers or diagnostic focus forcing
are present; all browser actions use the actual mounted UI.

| Final correction command/check | Result and limits |
| --- | --- |
| `python3 scripts/build-web.py --fast --skip-providers --routes subpath,root` | PASS, exit0; `native-host-reentry-build.log`. Both page routes at the final product source; unchanged validated provider assets reused. |
| `python3 scripts/test-command-palette.py http://127.0.0.1:4222/ --evidence .evidence/012-lineage/native-host-reentry-root` | PASS, exit0; `native-host-reentry-root.log`. Complete nondiagnostic production driver, including the real native Delete sequence and every prior palette outcome. |
| Same driver at `http://127.0.0.1:4222/boardstudio/`, evidence `native-host-reentry-subpath` | PASS, exit0; `native-host-reentry-subpath.log`. Same complete assertions on the subpath package. |
| `python3 scripts/test-header.py http://127.0.0.1:4222/boardstudio/ --evidence .evidence/012-lineage/native-host-reentry-header` | PASS, exit0; `native-host-reentry-header.log`. Existing Header navigation/history/domain outcomes, compact lifecycle and all16 geometry cases at the new host source. |
| `CARGO_PROFILE_DEV_DEBUG=false python3 scripts/run-wasm-tests.py --files web/src/presentation/command_palette.rs web/src/presentation/export_workspace.rs web/src/presentation/zmk_firmware_export.rs --result-json .evidence/012-lineage/native-host-reentry-page-results.json` | PASS9/9, exit0; `native-host-reentry-page.log`, complete structured report with0failed/0incomplete. Three palette tests cover public opener relationships, native host rapid reentry/unmount/remount and first/last/single/empty boundaries with disabled/hidden/inert/current-content guards; all six Export tests also pass. |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py lint typecheck repo` | PASS, exit0; `native-host-reentry-lint-typecheck-repo.log`. Source formatting, native/wasm Clippy, page typecheck and repository checks. |

Both full route runs prove CtrlK and CmdK above the real existing native Delete
confirmation, Search focus/typing/pointer/list navigation, one named exposed AX
dialog and unfocusable structural inner content, trusted first/last Tab wrapping
with prevention only at boundaries (`true,true,false,false` for four keys),
Escape-first and rapid reopen with the SAME underlying confirmation and original
Cancel focus. Underlying cancel/close counts stay zero across palette use;
subsequent native Cancel/Escape retain the original project ID, and only one
explicit trusted Confirm click removes that ID and retains the distinct durable
replacement. No underlying confirmation was closed to make the palette usable.
The same runs preserve field commit/cancel, Inspector Sheet identity/hidden-focus
checks, Snap settings/canonical focus/native Space counts, outside focus, canvas
pan/typing, command owner leases, real tools/objects, themes/narrow bounds and
document teardown/reopen. The native mounted fixture separately verifies
component-scope unmount/inert release and fresh host identity.

Inspected `native-host-reentry-subpath/native-confirmation-palette.png` and its
`native-confirmation-palette-ax.txt`: palette is above the retained confirmation,
with one exposed named dialog. These are Chromium visual/AX and keyboard receipts;
screen-reader and human026 clearance remain open.

Final feature freshness remains `a1828bb3aa15f46a45b355880ba5e31ab0aa9262`.
Exact diffs against `e974b961` remain empty for Library, BoardDialog/controls,
copied Dialog, registry, Header production and Layout adapters; incoming011/013
preservation checks also remain empty. Earlier registry223/Layout/dock/Library18
receipts retain their stated scope and pins. The new host makes no shared source
change that would turn them into fresh global acceptance. Final Outcome-only
repository validation is recorded in `native-host-outcome-repo.log`.
Targeted Spec and explicit Standards review through the coordinator is still
required at the completed clean correction pin. Ticket remains claimed and
landing HOLD; 015 combined acceptance and every previously named global gate
remain open. No new agents, threads, watchers or remote/local feature pushes.

### Final coordinator-approved closure and local landing — 2026-10-22

**Resolved within the coordinator's explicit bounded 012 acceptance.** This
supersedes the earlier claimed/review-pending/landing-HOLD entries above, which
remain as historical evidence. The two unchecked floor/shared-contract criteria
are intentional: 015's combined popup-entry-focus/root-Space-guard acceptance
and the global gates below are still open; resolution does not waive them.

Review results are coordinator-reported, not additional reviews executed by this
implementation child:

- Standards R1: APPROVE, 0 findings across the full 22-path closure at
  `e974b961a74436540255fb60e5d1822c49863925`.
- Spec R1: the native Delete top-layer P2 was the sole correction request; its
  actual RED and completed correction remain documented above.
- Standards R2 and Spec R2: both APPROVE, 0 findings at
  `a0d83bc9387c1426a7db49e25e3efee2f63db325`. Standards explicitly covered the
  meaningful new native host; Spec resolved only that P2 and connected host
  regressions, alongside the original review scope.
- R2 receipts:
  `node:delegated-task:command%3Amcp%3A2a86db4d-5ca5-4383-b139-deebb4fe7ee7%3Adelegate-task%3Arust-ui012-standards-review-r2`
  and
  `node:delegated-task:command%3Amcp%3A2a86db4d-5ca5-4383-b139-deebb4fe7ee7%3Adelegate-task%3Arust-ui012-spec-review-r2`.

Fresh T3 binding/CWD/branch and clean tracked/untracked inventory matched the
isolated continuation at the approved pin. Local feature remained
`a1828bb3aa15f46a45b355880ba5e31ab0aa9262`, an ancestor of this branch and not
checked out in another worktree. Source and tests are byte-identical to the
approved pin; changes since the final harness `4028631b` are this ticket only.
The final root/subpath, Header, nine palette/Export browser tests and
lint/typecheck/repo receipts above are reused with their exact source pins and
limits. No runtime rerun or new global acceptance is claimed for this doc-only
closure. Fresh closure `git diff --check` and repository validation are recorded
in `.evidence/012-lineage/landing-repo.log`; final local-FF refs and clean inventory
are recorded in `.evidence/012-lineage/landing-receipt.log` after commit.
The coordinator explicitly authorized the lane-protocol local fast-forward to
`feat/rust-ui-migration`; no origin/dev push or original-worktree edit is involved.

015 integration handoff (no unlanded 015 source copied): preserve the additive
`LayoutCommandPill` command registration/callback block in
`web/crates/layout/src/objects/layout_toolbar.rs:645`, the synchronous Snap-only
canonical handoff at line 658, and the canonical `m1-layout-snap-trigger` predicate
in the single root key handler at line 771. Compose 015's approved broader native
button Space guard there without preventing default or duplicating callbacks.
The 012 caller decisions remain `snap_in_dock: false` in
`web/src/presentation/layout_workspace.rs:420` and removal of only the redundant
Objects `LayoutSnapDock` mount in `web/src/presentation.rs`. Keep the existing
menu/settings owner and the transform-toolbar command adapters. Combined
validation must use `scripts/test-command-palette.py:396` onward (canonical
handoff, settings, trusted Enter/held Space counts, Escape, outside focus,
canvas pan and typing) plus 015's toolbar keyboard/actions checks. Its new popup
entry-focus adapter will require reviewing/updating the current legacy
canonical-trigger-on-open expectation to test the actual inside-popup floor.
This remains ticket 015/final-integration work, not 012 full-floor acceptance.

Global full/quick, CAD `worker.onerror`, first paint, screen-reader, human026 and
final merge gates remain open. Existing 013 notification and 011 persistent dock,
Sheet, Button ARIA and summary Space changes remain preserved.
