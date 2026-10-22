# 013: Toasts

Status: resolved
Type: build
Blocked by: [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [010: Header card](010-header-card.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [mapping](../../rust-ui-migration/mapping.md), [spec](../../rust-ui-migration/spec.md), [shared contract](../map.md#shared-contract)

## What to build

Transient results of one-shot actions appear as Toasts instead of inline text.

## Acceptance criteria

- [x] A Toast provider is mounted in the shell; saving, copying, export started or finished and one-shot failures use it
- [x] Blocking and persistent states stay inline (Export blockers, Case readiness, field errors); the classification rule in the mapping is applied to the shell's messages
- [x] Toasts are announced to assistive technology and dismissible by keyboard; tests cover both
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

Resolved after scoped verification and substantive Standards+Spec approval on `lane/013-toasts`; local landing follows the authorized fast-forward protocol.

### Ownership and contract

T3 binding, CWD, branch and clean initial inventory matched the explicitly bound lane at `1620af409354adbec350b2fb8b8e03f70ca96585`, also the local feature tip. Claim commit `ce232caa3` preceded implementation. The existing checkout uses `target -> /home/chris/.cache/boardstudio-target/013-toasts`; ignored settings, `cad/.cache` and `cad/wasm/pkg` were copied using the lane protocol, without creating another worktree. Builds are serial per target with project job settings. No Gortex or CodeGraph calls, agents, extra threads, watchers, origin pushes, dev changes or coordinator cleanup-document edits.

The coordinator approved exact shared shell hunks before production edits: a private notification module, the stable App/provider wrapper around the renamed private WorkbenchApp, one shell consumer replacing App's report banner, and suppression of explicitly transient reports in the shared banner. All existing App hooks, state and Editor lifetime remain inside WorkbenchApp. Lanes 011/012 were notified of the rename. Their dock/palette/header/shortcut ownership is preserved. The firmware panel adaptation is test-only; the production export callbacks and command registration remain with 012.

The runtime exposes only the new `RuntimeNotification { message, is_error }`, `take_notifications()` and `status_is_transient()` contract. Existing private visibility is unchanged. A single shell consumer drains an ordered queue using the existing version subscription; identical events between renders remain separate, draining has no notification side effect, and workspace changes do not remount the consumer or provider. Toast behavior, timing, atomic polite status announcement and the reachable close button come from ticket 008's copied Toast/primitives. The shell sets the existing `max_toasts` prop to `usize::MAX`, so there is no simultaneous-count cap and batches larger than ten are not silently truncated. The existing five-second expiry and explicit dismissal remain primitive-owned; no timer or queue option is added. Mounted coverage retains identical event multiplicity and persistent inline notices, waits for all twelve entries to expire, and normally unmounts/remounts a provider with active Toasts. The pinned primitive uses scoped `use_timeout`/Dioxus tasks; its receiver and sleep tasks are owned by the Toast component and are dropped with that scope. No separate timer instrumentation or large-queue responsiveness benchmark is claimed. Escape remains with the workspace. This verifies DOM semantics and keyboard interaction, not measured screen-reader speech.

### Classification and delivery order

Classification is explicit at emitters, never inferred from message text. Settled saves, actual export starts, current export completions/failures, and known import/download action failures produce Toasts. Project-copy download is the shell's existing copying action; no new clipboard action is added. Generic rejected edits, field errors, persistent startup/storage notices, persistence/recovery and non-export executor failures, export preflight/readiness blockers, Case readiness and new-keyboard dialog errors remain inline. The existing runtime status/severity interface is retained; transient error presentation is recorded separately so legacy status messages can still produce error Toasts.

`Effect::Settled` removes the lease for its exact `operation_id` and classifies with that returned capture. A matching current export's `ExecutorFailed` is an action result (Session uses that outcome for recoverable export generation failures); an unrelated operation's executor failure stays inline even while another export lease is active. Existing source/epoch/worker and latest-firmware gates remain authoritative. No domain outcome, edit, selection or callback changes.

Browser-refused project copies previously reported an error during `DeliverExport`, then `Completed` overwrote it with success. The existing private firmware delivery-error map is generalized to `export_delivery_errors`, still keyed by exact OperationId; settlement consumes the matching error before current-owner reporting, and cancellation removes it. A refused delivery produces one failure and no success notification. Stale delivery checks precede insertion. Board-outline success/error still reports at actual delivery, and its subsequent Completed remains silent.

### RED and intermediate verification

- `python3 scripts/check.py build --receipt .evidence/013/baseline-build.json`: passed, 443.3s, freshly packaged incumbent production source before edits.
- `python3 scripts/test-toasts.py http://127.0.0.1:4313/ --evidence .evidence/013/red-save`: expected RED, settled save had no named status Toast.
- Same command with `--delivery-only --evidence .evidence/013/red-delivery`: independent expected RED, a real `URL.createObjectURL` refusal reached the download boundary and no visible failure remained after settlement. Failure accessibility snapshots and screenshots accompany both logs.
- First implementation browser run reached the correct save announcement, then exposed an unsupported test-driver `find ... focus` command; corrected to the close button's role/name snapshot reference. A subsequent route assertion was corrected to track Toast identity, because setup performs two legitimate saves. These intermediate failed runs are retained and are not final acceptance.
- `.evidence/013/second-browser.log`: production save, project-copy success and refused download passed with passive focus, atomic status, real Tab/Enter dismissal and no route-change replay; this is intermediate packaged-source evidence.
- `python3 scripts/check.py browser --only runtime --receipt .evidence/013/runtime-browser.json`: 49/49 passed, including the initial three queue/classification/error-map tests and existing firmware ownership/retry tests. This is a partial browser subset.

Final checks, review and landing are recorded below when completed. Global full/quick/browser and CAD acceptance, CAD `worker.onerror`, first-paint timing, measured screen-reader output, human workspace review and ticket 026/final merge remain open.

### Final scoped verification

All checks below executed against clean source pin `30ecb26c9ffff432fbd3517ef3e8a3465cdb6854`, after merging the latest local feature (still `1620af409354adbec350b2fb8b8e03f70ca96585`, already an ancestor). This subsequent ticket-only result record reuses that code evidence; repository checks are rerun for the final documentation.

- `python3 scripts/check.py build lint repo --receipt .evidence/013/final-build-lint-repo.json`: **all passed**, 185.5s (build 74.5s, lint 104.1s, repo 6.9s). The receipt records a clean source revision. The old-CSS count is still reported, not a passed retirement gate.
- `python3 scripts/check.py browser --only runtime --receipt .evidence/013/final-runtime-browser.json`: **50/50 passed**, 15.5s. Includes four notification tests, exact-operation versus unrelated executor/persistence/recovery classification, repeated-event ordering/drain behavior, matching/stale/cancelled delivery-error cleanup, firmware generation failure/retry, stale worker/project/latest-operation notification silence, and prepared board-outline delivery with silent Completed. This is a partial browser subset.
- `python3 scripts/run-wasm-tests.py --files web/src/presentation/notifications.rs web/src/presentation/zmk_firmware_export.rs --result-json .evidence/013/final-presentation-browser.json`: **5/5 passed**. The bridge test preserves all twelve queued results, identical message multiplicity, no replay on version updates, persistent inline text, actual default five-second expiry, and normal unmount/remount with active Toasts. The retained firmware test name still checks the same row, unchanged model, failed delivery absence and exactly one successful retry artifact.
- `python3 scripts/test-toasts.py http://127.0.0.1:4313/ --evidence .evidence/013/final-browser`: **passed** against that freshly packaged production shell. Real save, project-copy download and browser-refused download produce the expected results; atomic status semantics, passive focus, real Tab/Enter dismissal and no route-change replay hold. Screenshot: `.evidence/013/final-browser/export-results.png`.
- The same production-browser command with `--delivery-only --evidence .evidence/013/final-delivery`: **passed**. The browser refuses one actual project-copy delivery; terminal presentation contains exactly one error Toast and no project-copy success Toast.
- `git diff --check`: **passed**. No existing test name was renamed or removed and no test-name baseline changed. New tests are WASM tests, not unexecuted native tests. Native runtime tests and global quick/full/browser/CAD gates were not run as part of these scoped checks.

The earlier test-fixture Signal-capture compile failure (`presentation-browser.log`) is retained and excluded from behavioral RED/GREEN. Corrected fixtures first passed 5/5 in `presentation-browser-r2.json`; the final receipt above adds executed expiry/unmount coverage. Timer ownership was reviewed in the pinned primitive and local `dioxus-sdk-time`/Dioxus source; browser timer handles were not separately instrumented. No large-queue responsiveness benchmark is claimed.

### Closure review

Both real coordinator closure axes approved `1620af409354adbec350b2fb8b8e03f70ca96585..2b25bdc9a48a9a8ba4ef4784ea9ed6bdbbf21484` with zero findings on 2026-10-22:

- Standards: `node:delegated-task:command%3Amcp%3A2a86db4d-5ca5-4383-b139-deebb4fe7ee7%3Adelegate-task%3Arust-ui013-standards-review-20261022-r1`.
- Spec: `node:delegated-task:command%3Amcp%3A2a86db4d-5ca5-4383-b139-deebb4fe7ee7%3Adelegate-task%3Arust-ui013-spec-review-20261022-r1`.

The coordinator reported substantive approval of explicit classification/persistent inline output, ordered queue, exact operation/lease/error/source/latest gates, false-success correction, outline delivery once, stable WorkbenchApp/provider and scoped expiry. Reviewers inspected the author's runtime receipts; they did not rerun those tests. The coordinator explicitly authorized local fast-forward landing after latest-tip verification.

That verification found local `feat/rust-ui-migration` still at `1620af409354adbec350b2fb8b8e03f70ca96585` and the lane clean at the reviewed pin. This resolution changes only ticket documentation, so the clean checked code receipts above remain sufficient. Fresh repository/documentation and diff checks precede the local fast-forward; final exact refs and clean inventory are reported to the coordinator. No origin push or dev merge is performed. Lanes 011/012 must preserve the App wrapper, notification/runtime integration and firmware test adapters when merging.

All global quick/full/browser/CAD, CAD `worker.onerror`, first-paint, measured screen-reader, human workspace review and ticket 026/final-merge gates remain open. This scoped resolution does not close them.

## Follow-up: notification stack interoperability (2026-10-22)

Status: resolved after coordinator review approval; local fast-forward authorized, with landing verification below.

The isolated T3 checkout `/home/chris/.t3/worktrees/ts-boardstudio2/t3-e62983e5`, branch `fix/notification-stack-interoperability`, started clean at `b237903705244a266b07a64e5f23b8a3b5219c03` and fast-forwarded the latest local feature pin `2de58e53aa5ca8a10d0223c2f32278ec42e3c623`. No parent dirty work was copied. Its target is `/home/chris/.cache/boardstudio-target/013-toast-stack-lineage`; ignored settings and CAD caches follow the lane protocol. Original 012/013 lanes remain untouched.

012 supplied `.evidence/012-lineage/header.log` and `header/failure.png` in `/home/chris/.t3/worktrees/ts-boardstudio2/t3-7bb7f80c`: the real `Original` Settings click is intercepted by `div#toast-dxc-275-label`, with thirteen notices stacked into the header menu. The follow-up owns only notification presentation, a focused production-browser regression and this record. Pending results, multiplicity, atomic announcements, dismissal, five-second expiry and persistent inline classification remain required. No dropping cap, domain/header/registry/dock changes or shared Toast API changes are authorized by this slice.

`scripts/test-toasts.py --stack-only` adds actual fourteen-result project-copy bursts and checks Settings reachability, bounded layout, focus-driven scrolling of every dismiss button, Tab/Enter dismissal and F6 notification access at 1440×960, 1024×960, 390×960 and 390×640 in both Themes and Appearances. Existing test modes are retained unchanged. RED/GREEN and affected checks follow below.

### Reproduction and correction

- Baseline package: `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py build --receipt .evidence/013-stack/baseline-build.json` passed (279.0s), recording clean production source `2de58e53aa5ca8a10d0223c2f32278ec42e3c623`. The regression/claim commit is `20f495be5` and changes no production source.
- Actual RED: `python3 scripts/test-header.py http://127.0.0.1:4319/ --evidence .evidence/013-stack/red-header` failed at the unchanged `Original` Settings click: `div#toast-dxc-186-label` intercepted it. The focused `python3 scripts/test-toasts.py http://127.0.0.1:4319/ --stack-only --evidence .evidence/013-stack/red-stack` independently failed because `div#toast-dxc-52-label` covered the Settings trigger after seven actual project-copy exports produced fourteen results. Both logs, accessibility snapshots and screenshots are retained.
- Cause: the primitive renders all pending entries into a bottom-anchored list without a height bound. Its layer 50 also exceeds the header's layer 40. The first scoped correction (`f8f47084e`) bounded scrolling and lowered the notification layer, but the unchanged header regression then exposed the duplicate footer Undo covered by the bottom of the viewport. The final correction (`a0d6a561ca11f97ec51b05ffa0410e5f30e6772c`) also leaves the footer action strip clear.
- The implementation adds one private-shell provider class through the existing public attribute prop and three notification-scoped rules in `web/tailwind.css`: native scrolling at at most 40dvh, bottom 4rem, layer 30, pointer interaction while entries exist, and removal of the list's default margin/padding/markers. The empty list has no interactive area. Every notice stays mounted and focus automatically scrolls its dismiss control into view; this is a presentation bound, not a count cap. The pinned shared Toast, all runtime delivery/classification code, App/provider lifetime, five-second expiry, passive atomic status semantics and keyboard dismissal are unchanged. No new dependency, timer, queue, public API, header, palette, registry or dock edit.
- The initial focused GREEN attempt passed all four desktop Theme/Appearance cases, then its setup used Escape from Export and returned to Layout. A diagnostic run captured that route change. The fixture now explicitly closes the compact menu with its trigger and asserts its closed state; it retains all result-count, click, focus, scrolling and dismissal assertions. The original header test remains byte-for-byte unchanged. Intermediate logs (`green-stack.log`, `green-header.log`, `diagnostic-stack.log`) are not final acceptance.

### Scoped verification and review handoff

Executed against clean code pin `a0d6a561ca11f97ec51b05ffa0410e5f30e6772c`, serving this checkout's own packaged site with `python3 scripts/serve-web.py web/target/site 4319`:

- `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py build --receipt .evidence/013-stack/final-build.json`: passed (53.8s).
- `python3 scripts/test-toasts.py http://127.0.0.1:4319/ --stack-only --evidence .evidence/013-stack/final-stack`: passed all sixteen size/Theme/Appearance cases. Each burst retains fourteen actual action results (seven starts and seven completions) through real Settings interaction, without waiting for expiry or dismissing to unblock it. All fourteen dismiss controls become visible and hittable when focused; Shift+Tab/Tab and Enter remove exactly one, leaving thirteen. F6 reaches the named notification region. All sixteen screenshots are retained; the author inspected the 390×640 Original dark and 1024×960 shadcn light captures.
- `python3 scripts/test-header.py http://127.0.0.1:4319/ --evidence .evidence/013-stack/final-header`: passed unchanged, including the original Settings click, real shell actions, footer/header Undo/Redo, compact menu lifecycle and Theme/Appearance geometry at four widths. Its inert Search expectation is appropriate to this lineage; 012 remains separate.
- `python3 scripts/test-toasts.py http://127.0.0.1:4319/ --evidence .evidence/013-stack/final-toasts`: passed existing save/copy/failure, passive announcement, Tab/Enter dismissal and route-change assertions.
- The same command with `--delivery-only --evidence .evidence/013-stack/final-delivery`: passed, one real browser-refused delivery produces exactly one error and no false success.
- `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py lint repo --receipt .evidence/013-stack/lint-repo.json`: all eleven commands passed at clean `a0d6a561ca11f97ec51b05ffa0410e5f30e6772c` (lint 107.8s, repo 6.1s). This includes rustfmt and native/WASM Clippy. Old-CSS retirement is still a reported count, not a passed zero gate.
- `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/run-wasm-tests.py --files web/src/presentation/notifications.rs web/src/presentation/zmk_firmware_export.rs --result-json .evidence/013-stack/presentation-browser.json`: 5/5 passed at `f8f47084e`. Reused for the final pin because its Rust/provider source is identical: the subsequent changes touch only the viewport CSS and focused Python fixture. This mounted subset verifies batched duplicate delivery, persistent inline text, actual default expiry, normal active-provider unmount/remount and existing firmware ownership/delivery behavior; it does not load or verify the packaged CSS.

The final documentation-only commit reuses the clean code evidence above; a fresh `scripts/check.py repo` receipt and `git diff --check` precede handoff. The final clean pin is reported to the coordinator, with review base `2de58e53aa5ca8a10d0223c2f32278ec42e3c623`. This follow-up is not landed: one completed-slice Spec+Standards review through the coordinator is required before authorization to land. No agents, new threads, watchers, origin pushes, dev changes, discards, resets or visibility widening. Global full/quick/browser/CAD, CAD `worker.onerror`, first-paint, measured screen-reader speech, other browsers, human review and ticket 026/final-merge gates remain open.

### Review correction: accessible burst target

The coordinator reported Spec **APPROVE, zero findings**, for `2de58e53aa5ca8a10d0223c2f32278ec42e3c623..334f97fb4d17eac3a41a3c9a75f7f5cb617f971e`. Standards reported one P3: the burst found its download action by a `button` tag query and `textContent`, violating the plan map's accessible role/name lookup contract.

- Spec receipt: `node:delegated-task:command%3Amcp%3A2a86db4d-5ca5-4383-b139-deebb4fe7ee7%3Adelegate-task%3Arust-ui013-stack-spec-review-r1`.
- Standards receipt: `node:delegated-task:command%3Amcp%3A2a86db4d-5ca5-4383-b139-deebb4fe7ee7%3Adelegate-task%3Arust-ui013-stack-standards-review-r1`.

The correction reuses the test's existing accessible snapshot/ref pattern: find the unique `button` named `Download .boardstudio project` in `snapshot -i`, focus that actual `@ref`, then retain `document.activeElement` as the burst's button handle. Ref resolution occurs before the burst. The same seven `.click()` calls and settlement checks execute within one in-page evaluation; no serial browser clicks, delays, dismissal, retry or assertion weakening was added. A direct comparison against `334f97f` verified that the seven-click loop and every subsequent assertion are byte-identical. The former lookup and its existing unavailable/disabled guard are otherwise unchanged.

Executed checks:

- `python3 scripts/test-toasts.py http://127.0.0.1:4319/ --stack-only --evidence .evidence/013-stack/standards-r2/stack`: **passed all sixteen cases**, with all prior multiplicity, reachability, scrolling, announcement, dismissal and F6 assertions. Log: `.evidence/013-stack/standards-r2/stack.log`; sixteen fresh screenshots accompany it.
- `python3 -m py_compile scripts/test-toasts.py` and `git diff --check`: passed.
- `git diff --exit-code 334f97fb4d17eac3a41a3c9a75f7f5cb617f971e -- web/src/presentation/notifications.rs web/tailwind.css web/crates/ui-shared/src/ui/toast.rs web/crates/runtime scripts/test-header.py`: passed. The delta from the reviewed pin is only this locator correction and ticket documentation, so the coordinator's Spec approval carries forward without a fresh Spec review.

The existing packaged production build and all `334f97f` snapshot/receipts are retained unchanged. No Cargo/build rerun is needed for this Python/docs-only delta. The fresh repository receipt `.evidence/013-stack/standards-r2/repo.json` passed all eight commands at clean `df1f22d4e103aa01949f0ee2e4a01ba8bce96eed` (5.7s). Targeted Standards re-review subsequently approved as recorded below. No extra agents, origin push or dev changes.

### Follow-up review resolution and local landing

The coordinator reported targeted Standards r2 **APPROVE, zero findings**, at `df1f22d4e103aa01949f0ee2e4a01ba8bce96eed`. The prior P3 is resolved: the unique accessible project-copy button ref supplies the active-element handle, while the seven-click loop and assertions remain unchanged, without added delay, retry or dismissal. Receipt: `node:delegated-task:command%3Amcp%3A2a86db4d-5ca5-4383-b139-deebb4fe7ee7%3Adelegate-task%3Arust-ui013-stack-standards-review-r2`.

Spec r1 **APPROVE, zero findings**, at `334f97fb4d17eac3a41a3c9a75f7f5cb617f971e` carries forward because production source is unchanged; its receipt is recorded above. Runtime verification remains author-executed evidence, not reviewer-executed testing. The coordinator explicitly authorized local fast-forward landing to `feat/rust-ui-migration` after latest-tip verification.

Fresh verification found the feature still at `2de58e53aa5ca8a10d0223c2f32278ec42e3c623`, with no attached feature checkout, and this bound fix checkout clean at reviewed `df1f22d4e103aa01949f0ee2e4a01ba8bce96eed`. This resolution changes only ticket documentation; validated production and tests are preserved and their sufficient receipts above are reused. Before the local fast-forward, fresh `git diff --check` and `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py repo --receipt .evidence/013-stack/landing/repo.json` verify the final documentation and clean inventory. The resulting final refs and cleanness are returned to the coordinator. 012 can then merge the landed notification fix for its unchanged Header interoperability check.

Landing is local fast-forward only: no origin or dev push, no extra agents and no changes to original 012/013 lanes. Global full/quick/browser/CAD, CAD `worker.onerror`, first-paint, measured screen-reader speech, other browsers, human review, ticket 026 and final-merge gates remain open. This scoped resolution does not grant global acceptance.
