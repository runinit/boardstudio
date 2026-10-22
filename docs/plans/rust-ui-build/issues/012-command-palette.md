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
