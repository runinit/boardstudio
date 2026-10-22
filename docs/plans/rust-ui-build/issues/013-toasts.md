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

Status: claimed; landing HOLD for coordinator Spec+Standards review of the completed slice.

The isolated T3 checkout `/home/chris/.t3/worktrees/ts-boardstudio2/t3-e62983e5`, branch `fix/notification-stack-interoperability`, started clean at `b237903705244a266b07a64e5f23b8a3b5219c03` and fast-forwarded the latest local feature pin `2de58e53aa5ca8a10d0223c2f32278ec42e3c623`. No parent dirty work was copied. Its target is `/home/chris/.cache/boardstudio-target/013-toast-stack-lineage`; ignored settings and CAD caches follow the lane protocol. Original 012/013 lanes remain untouched.

012 supplied `.evidence/012-lineage/header.log` and `header/failure.png` in `/home/chris/.t3/worktrees/ts-boardstudio2/t3-7bb7f80c`: the real `Original` Settings click is intercepted by `div#toast-dxc-275-label`, with thirteen notices stacked into the header menu. The follow-up owns only notification presentation, a focused production-browser regression and this record. Pending results, multiplicity, atomic announcements, dismissal, five-second expiry and persistent inline classification remain required. No dropping cap, domain/header/registry/dock changes or shared Toast API changes are authorized by this slice.

`scripts/test-toasts.py --stack-only` adds actual fourteen-result project-copy bursts and checks Settings reachability, bounded layout, focus-driven scrolling of every dismiss button, Tab/Enter dismissal and F6 notification access at 1440×960, 1024×960, 390×960 and 390×640 in both Themes and Appearances. Existing test modes are retained unchanged. RED/GREEN and affected checks follow below.
