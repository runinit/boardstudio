# 013: Toasts

Status: claimed
Type: build
Blocked by: [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [010: Header card](010-header-card.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [mapping](../../rust-ui-migration/mapping.md), [spec](../../rust-ui-migration/spec.md), [shared contract](../map.md#shared-contract)

## What to build

Transient results of one-shot actions appear as Toasts instead of inline text.

## Acceptance criteria

- [ ] A Toast provider is mounted in the shell; saving, copying, export started or finished and one-shot failures use it
- [ ] Blocking and persistent states stay inline (Export blockers, Case readiness, field errors); the classification rule in the mapping is applied to the shell's messages
- [ ] Toasts are announced to assistive technology and dismissible by keyboard; tests cover both
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

Work in progress on `lane/013-toasts`; closure review and local landing are held pending final verification and substantive coordinator approval.

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
