# 012: Command palette

Status: claimed
Type: build
Blocked by: [004: Vendored Lucide icons](004-lucide-icons.md), [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [010: Header card](010-header-card.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [command palette](../../rust-ui-migration/command-palette.md), [palette decision](../../rust-ui-migration/issues/012-command-palette-contents.md), [shared contract](../map.md#shared-contract)

## What to build

Ctrl/Cmd+K opens a searchable palette of the workbench's existing actions and objects, as specified.

## Acceptance criteria

- [ ] The palette follows the command palette spec: the groups, object search by name, hidden commands for other Workspaces, disabled-with-reason for unavailable ones, and no destructive or numeric edits
- [ ] It opens from the header button and from Ctrl/Cmd+K anywhere, including inside a field, which blurs as on a click-away; Escape closes it first
- [ ] Undo and redo show their existing hints; the Alt, Shift and Ctrl pointer modifiers are documented as `Kbd` hints in tooltips on the controls they affect
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
