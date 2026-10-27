# 005: Rust drag smoke end to end

Status: claimed
Type: build
Blocked by: none
Parent: [Chromiumoxide browser checks](../map.md)
Related: [Real-browser checks drive Chrome from Rust](../../cad-deepening/issues/016-rust-browser-harness.md), [Prove browser harness fidelity before cutover](004-feasibility-proof.md)

## What to build

A developer can run the supported real-page drag smoke through a minimal Rust chromiumoxide
harness and receive the same checks, failure verdicts and verified success receipt as the
existing Playwright runner. Deliver the narrow end-to-end path, not a generic browser framework.
Reuse the existing spike; do not repeat a broad feasibility exercise. Keep the old runner
available for same-build comparisons until the parity gate passes. Keep its dependencies
available to consumers not yet migrated.

## Acceptance

- [ ] The harness has its own manifest/lockfile outside the app workspace. Implement only
  consumer-required helpers: isolated launch, role/name lookup, bounded waits/actionability,
  trusted input, evaluation, fatal error capture and screenshots as required.
- [ ] Portable discovery honours CHROME_BIN first. Each run owns writable config/profile
  storage and cleans up its processes on success/failure; fresh runs do not share IndexedDB.
  Missing prerequisites and startup failures are actionable, without personal absolute pins.
- [ ] Pin the library and CI browser together; log versions and retain the existing launch
  flags/viewport. No machine-global profile or sandbox-policy changes.
- [ ] Preserve every existing drag-smoke assertion, including Part/Column/Stagger/Ctrl-offset,
  trusted pointer capture/live preview, durable saves, complete-document Undo/reload, timing,
  and collision-check observations. Preserve launcher arguments, exit codes, receipt fields,
  provenance validation and failure logs. Never emit a success receipt for partial runs.
- [ ] Same-source/build old/new runs demonstrate matching checks/verdicts and comparable
  receipts. A reproduced app failure remains a failure in both; successful cutover requires
  the full gate to pass, not merely matching failures. Escalate a blocking app defect separately.
- [ ] Negative controls prove page exceptions AND unhandled rejections fail the run; startup,
  command/protocol failure, bounded timeout and assertion failure remain distinguishable.
  Lookup/input tests cover exact/scoped names and absent/ambiguous/non-actionable targets.
- [ ] Capture full redacted invalid-event evidence where it occurs. Identify schema mismatch
  and required-event impact; default dropping/warning is not proof of harmlessness. Required
  event loss or unresolved impact blocks cutover. Do not swallow evaluation errors in waits.
- [ ] The supported check entry point and CI exercise the Rust path after parity, with harness
  formatting/lint coverage and reproducible browser/prerequisite setup. Mounted WASM runner
  stays unchanged; other Playwright consumers keep working during this slice.

## Verification and scope

Use TDD at the existing public launcher/output seams: demonstrate the expected regression
before the fix, then run affected tests and typechecking during development. At closure run
the full supported check and combined Spec + Standards review; record terminal verdicts,
source/build identity and any blocked gates honestly. No weakened assertions, masked page
errors, baseline deletions or retry-until-green. Preserve the archived prototypes and unrelated
work. Builds follow repository concurrency/lock rules; evidence uses supported receipts and
logs, not custom hash manifests. All checks must be runnable by humans and CI without agent tools.

## Comments

### 2026-10-26: vertical slice implemented; parity gate BLOCKED by two app defects (ticket stays claimed)

Built the minimal excluded crate `tools/browser-harness/` (own `[workspace]`,
`Cargo.toml` pins chromiumoxide `=0.9.1`, own `Cargo.lock`; lib + `drag-smoke` +
`negative_probe` binaries). Launcher opt-in: `BOARDSTUDIO_DRAG_DRIVER=rust`
(default stays `node`); `check.py` `LINT` appends harness fmt/clippy (gate `lint`
step passes, 4.9 s). All Cargo/wasm work under the build lock; isolated per-run
profile/config dirs with whole-run-dir cleanup; CHROME_BIN-first discovery.

Contract port: every old-runner assertion is preserved, including Control-first
gesture for the Control-swapped offset handle (the toolbar swaps the stagger
handle label only after Control is down), pointer-capture/pointerdown-trust/
movement checks, in-page exact verdicts (all document/state comparisons run
page-side so no float repr crosses the bridge), durable-save wait, exact
page-signature Undo equality with `diff_paths` diagnostics, exact scene restore,
receipt byte-compat (launcher checksum verifies), timing bounds, quaternion
collision observations/provenance validators. `force: 0.5` on held buttons
matches Playwright's reference input exactly (page-observable pressure 0.5).
Negative probe: 12/12 controls green (page exception + unhandled rejection fail
the run; command/timeout/startup/assertion failures distinct; exact/scoped/
absent/ambiguous/zero-area/hidden lookups; IndexedDB isolation between contexts).
Lib unit tests 8/8; seam tests `test-browser-tools.py` 6/6, `test-check.py` 31/31.

RED->GREEN record: stale-node races (-> resolve+describe retried as a unit, plus
`no match` vs detached distinction), Control-after-locate ordering (-> Control
down first + wait for the swapped handle), inlined-fixture paren imbalance
(-> node --check on rendered output), zero-area fixture with UA border/padding
(-> border:0;padding:0), deliberate-rejection allowlist in final error scan,
hit-test flake on a re-rendered toolbar (-> re-resolve every 100 ms poll;
hit-test now names the covering element).

PARITY (same source/build, `web/target/site` 04:36Z): Rust passes all 15 checks
when the app cooperates (full PASS receipts verified by the launcher). The old
runner deterministically FAILS Part-drag Undo (`undone.signature !=
before.signature`). Root cause (SUPERSEDED 2026-10-26: unproven, see the correction at the end): the Undo save
(rev N+1 = restored content, proven by waitDocument matching) is then clobbered
by a STALE save carrying rev N / dragged content (final doc byte-equals the
after-drag doc; revision regresses N+1 -> N). The same stale-save Undo race was
also observed for Matrix move on healthy state. Separately, drag sequences in
BOTH runners trigger a WASM `unreachable` panic at timing-variable points
(after Matrix move in Rust flow; around Ctrl-offset/matrix-select in old flow;
matrix-move-alone on healthy state is clean; pageerror: `unreachable`), which
kills the app and leaves the `#__dx-toast` panic toast (max z-index,
still hit-testable) covering the toolbar — the harness now fails fast with
`the app panicked: ...` instead of burning the 60 s click bound. Both defects
(attribution SUPERSEDED, see the correction at the end) are app-side (harness implements the contract exactly; the panic also fires
under the reference Playwright driver) and are escalated separately — NO app
code was changed here, and the ticket is NOT resolved by matching failures.

Gate status: `check.py lint` passes (incl. harness entries). Full gate blocks at
the drag smoke on the app defects above (old default fails Part Undo; Rust fails
fast on the panic toast when it fires). Parity gate (matching old/new verdicts)
fails: old fails Part Undo deterministically while Rust passes it — opposite
verdicts on the same build, explained by the race interleaving, not by port
infidelity (identical centers/coords/params verified: Part center 606.50,583.64
both runners; down/move records byte-identical modulo capture retargeting).

Open: app-bug escalation (stale-save Undo race + `unreachable` panic);
parity re-run after app fix; CI wiring of the Rust path only after parity truly
passes.

### 2026-10-26: two independent reviews done; findings answered (ticket still claimed)

Standards review (claude-sonnet-5-5, commit `6a45f68ac` vs `d800bff12`,
ticket005 paths only): no orchestrator-standard violations beyond judgement
calls; evidence rules met. Fixed as committed follow-up: removed the
`/usr/bin/chromium` fallback in `resolve_chrome` (CHROME_BIN-first + PATH
probes only; runner always sets CHROME_BIN); `canonical` scene signatures and
receipt bytes now fail loud (`expect`) instead of `unwrap_or_default`;
`Error::new` replaces both `fn fail` copies; exec-bit check factored out.
Deferred by design (not fixed): dependabot/CI-cache coverage for the new crate
(CI wiring waits for parity per this ticket); `BOARDSTUDIO_DRAG_DRIVER` default
in three places; bespoke `iso_now`/hex (avoids new deps); tuple/i64 shapes.
Build-lock path `~/.cache/tmp/boardstudio-build.lock` follows the session
resource guards; nothing else contends for the harness target dir.

Spec review (first delegate refused on its own tool limits; retry with
claude-opus-5-5 completed): verdict "not resolved" — agrees the ticket stays
claimed. Findings: (a) invalid-protocol-event evidence missing — ANSWERED BY
EXPERIMENT: `surface_invalid_messages()` was enabled and immediately caught
real drift (19 `untagged enum Message` parse failures per run: Chrome 156 vs
chromiumoxide 0.9 snapshot), failing the teardown scan even though all drag
checks passed. Failing every run on benign skew makes the gate unusable, so
tolerance was restored with a code comment recording the evidence; joint
library+browser pinning at cutover is the real fix, and every required
protocol interaction already has a timeout/assertion that fails loudly on
actual loss. Cutover stays blocked on this too. (b) panic-toast fail-fast
kept deliberately: same FAIL verdict, clearer message than a 60 s hit-test
timeout (old runner would also time out on a dead app). (c) viewport
assertion is NOT creep: the old runner asserts the same Case-viewer-inside-
viewport check. (d) `end_drag().await.ok()` sites all return the original
error; cleanup is best-effort. (e) parity/CI gaps acknowledged — both
deliberately held (parity blocked by the app defects above; CI after parity).

### 2026-10-26: correction of the PARITY diagnosis (causes now proven; ticket still claimed)

The "stale save clobbers the Undo save" root cause above is **retracted**: no
instrumented save sequence showed it. The old runner's Undo failure and the
`unreachable` panic had separate causes:

1. **Runner wait bug (Node driver only).** Under the pinned playwright-core
   1.62.1, `page.waitForFunction` with a promise-returning predicate
   evaluates once and resolves with the first falsy value (probe: one
   evaluation, result `false` after ~30 ms while the flag flips at 500 ms).
   `waitDocument` and the matrix-state wait used such predicates, so the Undo
   wait returned before the save landed. The original script against an app
   build carrying the app fixes still fails `Part drag: Undo restored the
   complete saved document`; the same script with Node-side polling (same 20 s
   bound, same comparisons, every assertion unchanged) passes 15 checks, and
   the Rust driver passes 15 checks on the identical build (equal
   `build_sha256`, head `04016e7a0`). No app save defect was found.
2. **App panic (both drivers).** The 1.8 s commit-toast timer touched overlay
   signals after the overlay unmounted (scope and selection switches remount
   it); Dioxus raised `ValueDroppedError`, which traps as WASM `unreachable`
   and leaves `#__dx-toast`. Reproduced by the mounted regression test
   `commit_toast_timer_survives_overlay_unmount` (red on the old timer body,
   green with `try_peek`/`try_write`).
3. **Anchor pick overlay mount hang (layout browser launch).** The overlay's
   stale-arm effect read the signal it then wrote, so it re-ran forever and
   froze the main thread; the layout launch could not finish. Present before
   these fixes (already on the Ticket 08 base); fixed by reading with `peek`.

The layout test `detached_part_pick_anchors_from_the_half_origin` failed historically;
the diagnosis that an unsplit fixture has no half-origin ring was incorrect. On
the rebased worktree the mounted test passed in the focused anchor-pick run
(9 passed), which checked for a major ring. A subsequent focused run passed
the stronger assertion for the Half origin's white current dot (1 passed).
The layout `matrix_transform_inspector` detach-pose
test was red on the base, and page-suite relation-highlight failures also failed
before Ticket 08. Follow-up focused receipts on the rebased worktree cover the
detach test and Relations module; they do not establish full-suite acceptance.
005's other gates are unchanged: joint library+browser pinning, protocol
event-safety, and CI cutover. Playwright remains the default driver.

### 2026-10-26: local-page protocol schema probe (cutover still blocked)

On `6346fe98`, a temporary isolated Chromium `156.0.8078.47` probe enabled
`surface_invalid_messages` and loaded the checked local page plus one fetch.
It captured 16 invalid messages, all `Network.requestWillBeSentExtraInfo`.
Direct deserialization of each captured `params` with the pinned
`chromiumoxide_cdp 0.9.1` event type failed on missing
`clientSecurityState.privateNetworkRequestPolicy`; replacing the received
`localNetworkAccessRequestPolicy` key with the expected key made all 16
payloads deserialize. The redacted capture and direct-deserialization results
are in `.evidence/protocol-capture-6346fe98.log` and
`.evidence/protocol-schema-6346fe98.log` respectively. The probe source was
removed after execution.

This identifies the mismatch for this local-page sample, not all messages during
the drag smoke. It does not establish required-event safety, joint CI pinning,
or parity on a passing full check. Those gates remain open; the failed default
check and its browser failures remain failures.

A subsequent diagnostic enabled `surface_invalid_messages` during the actual
Rust drag runner on the existing checked site. It recorded 88 invalid messages,
all `Network.requestWillBeSentExtraInfo` with the same missing old policy key
and present new policy key; the redacted events are in
`.evidence/protocol-drag-events-595463c.jsonl`. The Part, Column, Stagger,
Ctrl Offset and Matrix drag/Undo checks and default collision observation were
logged before the handler's 88 errors caused the run to fail at teardown. The
runner did not produce a success receipt. This is evidence about the observed
messages, not a passing parity run or proof for other Chrome versions or other
consumers. The temporary handler instrumentation was restored after the run.

### 2026-10-26: candidate Chrome for Testing comparison

A candidate Chrome for Testing `145.0.7622.0` (Chromium revision `1566023`,
close to chromiumoxide 0.9.1's CDP snapshot `1566079`) was downloaded to an
isolated cache from the [Chrome for Testing version catalog](https://github.com/GoogleChromeLabs/chrome-for-testing).
A diagnostic Rust drag run with invalid messages surfaced passed all 15 checks
and recorded zero invalid messages. The diagnostic temporarily changed the
handler; it is not a cutover receipt.

With that instrumentation removed, Node and Rust each passed 15 checks on the
same existing site build and candidate Chrome. Launcher-verified receipts
`20261026T214238Z-45801fbfa40e` and `20261026T214336Z-0e4ec9a7bd64` both
record source head `697faa1`, a clean source diff, the same build digest, browser
version and check names. Logs: `.evidence/parity-cft145-node.log` and
`.evidence/parity-cft145-rust.log`. This comparison does not independently
establish that the existing site was built from that source head. It does not
clear the failed default check, prove CI provisioning, pin a browser, or prove
required-event safety for the other consumers. Ticket 005 remains claimed.

### 2026-10-26: fresh-build comparison and provisioning boundary

On clean `4c6eaec1c431deee9fd651983f40220fecb96bba`, ran the checked
`check.py build` under the build lock (pass, 68.6 s), then ran the Node and Rust
launchers sequentially against that unchanged site with isolated Chrome for Testing
`145.0.7622.0` and chromiumoxide `0.9.1`. Both passed 15 checks and their
launcher-verified receipts have the same clean source digest, build digest
`6f9242d46d35eb540dfb9f5eaae11fc1e9ecfa83aafcd6c84f84df29f129cfdc`,
viewport and ordered check names. Node receipt: `20261026T222606Z-bd77152ab857`;
Rust receipt: `20261026T222702Z-7ab800106a3e`. Timing and measured viewer
geometry vary between runs. Logs are `.evidence/ticket005-fresh-build-4c6eaec.log`,
`.evidence/ticket005-fresh-node-4c6eaec.log`, and
`.evidence/ticket005-fresh-rust-4c6eaec.log`. This is a local same-source,
same-build drag comparison, not a passing full gate or CI proof.

The [version-specific Chrome for Testing catalog](https://googlechromelabs.github.io/chrome-for-testing/145.0.7622.0.json)
lists Linux x64 Chrome and matching ChromeDriver downloads, revision `1566023`.
A CI cutover must provision and verify an exact browser version alongside the
locked library, pass its executable via `CHROME_BIN` to the real-page runner,
log both versions, and preserve the existing mounted WASM ChromeDriver runner
and other Playwright consumers. Current CI merely checks the hosted Chrome and
ChromeDriver versions and runs the default Node path; it does not provision this
pair. Availability of the catalog and this local candidate do not establish a
supported CI installation or required-event safety beyond the observed diagnostic.
Do not switch the default until those requirements and the full gate pass.

The previous full check at `6346fe98` remains failed/incomplete: Parts reported
33 failures, Keycaps was incomplete, and page listing and page suite did not run
(`.evidence/drag-save-undo-default-6346fe98.json.md`). This comparison does not
replace that receipt. Ticket remains claimed; CI provisioning, full supported
check, protocol-event impact and closure review remain open.

### 2026-10-26: pinned provisioning and fatal event preparation

Starting at clean `187885fe40af7b9f514dd8596b962aea68f96c4d`, paired the
existing chromiumoxide `=0.9.1` lock with Chrome for Testing `145.0.7622.0`.
`scripts/install-drag-chrome.sh` downloads the Linux x64 archive, checks its
SHA-256 and executable version, and CI provisions it in `RUNNER_TEMP`. The
existing system Chrome, ChromeDriver, mounted WASM runner, other Playwright
consumers and Node default are unchanged. CI does not yet select this browser
or run the Rust path: cutover remains gated.

The Rust handler now surfaces invalid messages, logs redacted event payloads,
and fails on handler errors. The existing negative probe now checks an
unhandled rejection through the same `PageErrors` collector and fatal verdict
as the drag consumer. On Chrome 145, the negative probe passed, including
exception, rejection, command, timeout, startup, lookup and isolation checks;
format and clippy also passed (`.evidence/ticket005-final-probe-187885fe.log`).
On Chrome 156.0.8078.47 the same probe failed as intended with one invalid
`Network.requestWillBeSentExtraInfo` event and its redacted payload
(`.evidence/ticket005-chrome156-negative-187885fe.log`). This confirms the
incompatible event is observable and fatal; it does not establish safety for
other consumers or browser versions.

The actual Rust drag consumer passed 15 checks on Chrome 145 with invalid
messages surfaced and no handler errors, receipt `20261026T223007Z-123d7dc720b1`
(`.evidence/ticket005-event-runtime-187885fe.log`). It used the existing site
build digest `6f9242d46d35eb540dfb9f5eaae11fc1e9ecfa83aafcd6c84f84df29f129cfdc`
and a dirty source tree based on `187885fe`; a subsequent change expanded
redaction of header text, cookies and request bodies, and the negative probe
and clippy were rerun on that change. This receipt is focused runtime evidence,
not a clean-source parity or full-gate result. Provisioning passed locally after
correcting Chrome's trailing whitespace in version output
(`.evidence/ticket005-provision-fixed-187885fe.log`); hosted CI has not run.

The original full check remains failed/incomplete (Parts 33 failures, Keycaps
incomplete, page listing and suite not run). Full supported check, required
event safety for other consumers, CI Rust-path cutover, and closure review remain
open. Ticket stays claimed; no default-driver change is authorized by these
focused results.

### 2026-10-26: independent review follow-up

On `cf5fd1a`, a focused redaction regression failed on the exposed request-body
entry (`.evidence/ticket005-review-red.log`). The handler now redacts
`postDataEntries` and `cookieLine` recursively. Both consumers close and reap
Chrome and await the handler drain on successful and failed runs before the
handler verdict; a failed or timed-out shutdown cannot produce a receipt.
The drag consumer checks page errors before writing the receipt.

The harness fmt, clippy and nine unit tests passed, as did the existing negative
probe on Chrome 145 (`.evidence/ticket005-review-final-checks.log`). The
consumer controls in `scripts/test-drag-browser-consumer-controls.py` exercised
an actual slider assertion failure, a page exception and an unhandled rejection:
each exited 1 with the expected error and no receipt
(`.evidence/ticket005-review-committed-controls.log`). The Rust drag consumer
passed 15 checks against the existing site build on Chrome 145, receipt
`20261026T223553Z-f9b33f30d307`
(`.evidence/ticket005-review-final-drag.log`). The test-control script was
edited during that run; the receipt identifies the source state at launch and
is not clean-source parity evidence.

A deliberately late invalid protocol event was not injected at the shutdown
boundary; the drain ordering is implemented but that specific regression is
not independently demonstrated. The earlier full default check remains
failed/incomplete (Parts 33 failures, Keycaps incomplete); no full check or
closure review is claimed here. Ticket remains claimed.

### 2026-10-26: shutdown and diagnostics follow-up (partial)

The handler now scrubs authorization and cookie values in nested CDP name/value
objects, and reports all collected handler and page errors. A separate uncommitted
shutdown follow-up attempts to retain scenario, page and shutdown failures together,
bound shutdown operations and preserve the profile when exit is unconfirmed. That
follow-up is not part of the committed slice and still needs direct verification.

The focused regression initially failed for a leaked authorization value and a
missing second page error (`.evidence/ticket005-followup-red.log`); the harness
tests and clippy passed after the change (`.evidence/ticket005-followup-final-tests.log`,
`.evidence/ticket005-followup-clippy.log`). The Chrome 145 negative probe passed on the uncommitted follow-up
(`.evidence/ticket005-followup-probe.log`). A later fmt, 11 unit tests and clippy
run also passed on that worktree (`.evidence/ticket005-current-checks.log`). Abnormal exit, unconfirmed termination,
and simultaneous scenario/page/shutdown failures have not received dedicated
red/green regression checks, so this follow-up is not yet verified for closure.
The original full default check remains failed/incomplete (Parts 33 failures,
Keycaps incomplete); no full check or closure review is claimed.

The final-event boundary remains unresolved. In chromiumoxide 0.9.1,
`Handler::poll_next` returns `None` immediately on the `Browser.close` response,
without reading subsequent websocket messages. A local dependency patch or
upstream change could drain through websocket closure and expose a reliable
completion signal, but needs a late-event regression and proof of ordering.
Sending `Browser.close` through the existing generic `Browser::execute` avoids
that special close-response branch, but the handler treats websocket EOF as
pending rather than terminating; it therefore cannot currently confirm a drain.
A separate raw CDP transport could own and observe the connection to closure,
but would require a larger integration and proof that it sees the relevant
sessions and errors. None of these alternatives is demonstrated here, and no
member visibility was widened.

Also, chromiumoxide's launch-error cleanup itself calls unbounded `kill().await`
and `wait().await` and panics on cleanup errors before returning a `Browser`.
The shutdown changes above cover only a successfully returned browser; launch
failure termination and profile preservation require further work.

### 2026-10-26: source inspection of the end-of-event boundary

On `6648a9b`, the three-file shutdown draft remained uncommitted. Inspection of
the installed chromiumoxide 0.9.1 source found another requirement for a local
patch: `Handler::on_event` queues target listener events, and the handler polls
those listener queues before, not after, reading websocket messages. Returning
at websocket EOF without first delivering queued events could therefore still
drop a final page exception. The detached page listener tasks must also finish
before the consumers inspect `PageErrors` and write a receipt. A patch needs to
prove all three stages: read through a terminal transport condition, deliver
queued events, and join the listener tasks. No dependency patch or upstream
revision has been verified here, and no member visibility has been changed.

This inspection ran `git diff --check` successfully; it did not run a build,
regression, browser check, or review. The existing shutdown draft, its abnormal
exit and unconfirmed-termination checks, combined diagnostics, launch-error
cleanup, controlled late-event regression, full gate, and closure review remain
open. The earlier focused receipts above are not new verification of this draft.

### 2026-10-26: implementation baseline (incomplete)

On `787a454`, with the three-file shutdown draft still uncommitted, the harness
unit tests passed (11 tests) and the Chrome 145 negative probe passed under the
build lock and `boardstudio-tests.slice` (`.evidence/ticket005-subagent-baseline-probe.log`).
Neither check exercises an event arriving after the close response, listener
completion, abnormal exit, or launch-error cleanup. The final-event boundary and
the draft's verification remain open; these results are not a closure receipt.

### 2026-10-26: bounded shutdown slice (partial)

For a successfully launched browser, close, wait, forced kill, final wait and
handler drain are bounded. A failed close triggers kill immediately; abnormal
exit, forced termination, shutdown and handler errors remain failures. Both
consumers report scenario, shutdown and page errors together and preserve the
run directory when process termination cannot be confirmed.

Harness formatting and 11 unit tests passed, and the existing negative probe
passed on Chrome 145 under the build lock and `boardstudio-tests.slice`
(`.evidence/ticket005-bounded-shutdown-tests.log` and
`.evidence/ticket005-bounded-shutdown-probe.log`). These checks do not inject
an abnormal exit, unconfirmed termination, or simultaneous scenario, page and
shutdown failures; no dedicated RED/GREEN regression was run for this slice.
Launch-error cleanup, final-event delivery and listener completion remain open,
as do the full gate and closure review. This is not a ticket closure receipt.

### 2026-10-26: additional event-delivery ordering finding

At `9127cb266`, inspection of chromiumoxide 0.9.1 found that draining only at
websocket termination is insufficient. The handler can read a session event,
queue it on a target listener, and then read `Target.targetDestroyed` in the
same websocket polling loop. The latter removes the target before the next
listener poll, dropping the queued event. A patch must deliver queued listener
events before a subsequent message can remove their target, as well as drain
through transport termination and join the consumers' listener tasks.

No implementation or controlled late-event regression was completed in this
inspection. The existing public listener and page-creation helpers return no
listener task handle; the binary consumers and shared driver are separate
crates, so an internal join boundary still needs to be designed without
widening the existing API. This is an unresolved design problem, not proof that
such a boundary is impossible. The late invalid-protocol and late page-exception
RED/GREEN checks, launch-error cleanup, full gate and closure review remain open.

### 2026-10-26: local chromiumoxide event-queue patch (partial, consumer blocked)

On clean base `a324aeb41c45c6d9382b4e1e2a10544d5f5a3b7f`, vendored
chromiumoxide 0.9.1 under the harness and patched only its event/transport
lifecycle. `Browser.close` now returns protocol errors and no longer ends the
handler on its response. The handler reads until transport termination,
retains destroyed targets' listener queues until delivery, drains remaining
listeners before terminating, and reports unexpected termination and transport
errors. Socket flush/readiness errors are propagated. No existing public API or
member visibility was widened. Harness Clippy uses `--no-deps` to retain its
previous registry-dependency lint boundary for the new local fork.

A local WebSocket regression sent a successful close response, a late session
event, target destruction, an invalid event, and a close frame in sequence. It
failed on upstream with the late invalid event missing
(`.evidence/chromiumoxide-lifecycle-red.log`) and passed with the patch; the
listener received its event and closed after draining. A second focused test
verifies a close protocol error and unexpected transport close are failures.
Both tests passed after the final transport changes
(`.evidence/chromiumoxide-final-checks.log`). Harness unit tests passed 11/11;
format and the harness Clippy command with `--no-deps` passed.

The Chrome for Testing 145 negative probe ran all its scenarios but failed
shutdown because Chrome reset the WebSocket without a closing handshake
(`.evidence/chromiumoxide-harness-probe.log`). The reset remains a fatal
transport error: this patch does not establish that all events reached the
consumer across that reset. It is not a passing browser receipt. The consumer's
page listener tasks are still detached and are not joined before inspecting
page errors; a real late page-exception boundary has not been proved. The
launch-error cleanup, full supported check, CI cutover, parity and closure review
remain open. Ticket 005 remains claimed.

### 2026-10-26: Chrome 145 shutdown source and partial listener ownership

At `1897387722103b14a6e950b0f41888da28af1ce5`, the original Chrome 145
negative-probe failure above remains a failure. The exact [Chrome 145 DevTools
HTTP handler](https://chromium.googlesource.com/chromium/src/+/refs/tags/145.0.7622.0/content/browser/devtools/devtools_http_handler.cc)
posts an `Inspector.detached` message followed by `ServerWrapper::Close` when
an agent host closes, and its destructor explicitly accounts for last-minute
messages during session disconnection. `ServerWrapper::Close` calls
[HttpServer::Close](https://chromium.googlesource.com/chromium/src/+/refs/tags/145.0.7622.0/net/server/http_server.cc),
which removes the connection and schedules its destruction; that path does not
perform a WebSocket close handshake. This explains why a reset is compatible
with expected shutdown, but neither a successful close response nor a zero
process exit establishes delivery of all queued messages. A premature loss can
therefore not safely be classified as a clean shutdown from that evidence
alone. The reset remains fatal.

The consumer changes in this working tree retain the page exception tasks in
`PageErrors` and join them after handler termination before the drain succeeds;
late registration after termination fails. A timed-out drain aborts the task set
and fails shutdown, so completion of each listener is not established on that
failure path. Both consumer binaries use the same collector for all their
listeners. Harness unit tests (11), vendored lifecycle tests (2), formatting,
and harness Clippy passed under the build lock and `boardstudio-tests.slice`
(`.evidence/ticket005-listener-join-tests.log`,
`.evidence/ticket005-listener-join-checks.log`). These existing checks do not
prove the new consumer join boundary. No controlled late consumer exception or
invalid-event negative control has yet been run, and no fresh Chrome probe or
drag was run: the reset ambiguity has not been fixed. The full supported check,
parity, CI cutover, launch-error cleanup and closure review remain open.

### 2026-10-26: consumer controls and unresolved transport boundary

The consumer now owns and joins its page listener tasks before its final
verdict. A Chrome 145 negative probe triggers a rejection immediately before
shutdown without waiting for its listener, and observed that rejection at the
final verdict. The same run failed on `Connection reset without closing
handshake`; this remains a fatal error, not a successful probe
(`.evidence/ticket005-late-consumer-probe.log`). This observation does not
force the listener to lag the handler and is not a RED/GREEN proof of the join.

A controlled WebSocket peer sends an invalid `Runtime.exceptionThrown` after
the `Browser.close` response and a close frame. The real consumer handler drain
records its payload and its fatal verdict rejects it. The harness's 12 tests,
the two vendored lifecycle controls, project-scoped harness Clippy and format
checks passed (`.evidence/ticket005-consumer-verified-tests.log`,
`.evidence/ticket005-join-vendor-tests.log`, and
`.evidence/ticket005-consumer-verified-clippy.log`). An earlier Clippy run
without `--no-deps` failed on an existing vendored lint; the first run with the
new control failed on two test lints, corrected before the final checks.

Chrome 145's [WebSocket reader](https://chromium.googlesource.com/chromium/src/+/refs/tags/145.0.7622.0/net/server/web_socket.cc)
does send a reply when the client starts the close handshake, but
`HttpServer::DoReadLoop` closes the connection after that frame, while DevTools
messages are independently posted from the UI thread to the socket thread.
An acknowledgement therefore cannot establish that later UI-thread messages
were delivered; `Browser.close` response, TCP loss, close acknowledgement and
process exit are distinct observations. No client-initiated shutdown change was
made without an ordering guarantee. A deterministic delayed-listener control,
a defensible complete transport boundary, full supported check, parity, CI
cutover, launch-error cleanup and closure review remain open.

### 2026-10-26: transport control and observation-boundary blocker

At base `667eb82d42e4396856ee3de28f3ad695220fdf81`, a controlled peer
replies successfully to `Browser.close` and then drops the WebSocket without a
close handshake. The actual consumer handler drain records the reset and
`HandlerErrors::fail_if_any` rejects the verdict. The harness's 13 tests,
including the existing late-invalid-event control, and harness Clippy passed
(`.evidence/ticket005-transport-control-checks.log`). The earlier Chrome 145
late-rejection probe observed the rejection before the consumer verdict but
failed on that reset; it does not demonstrate a deterministic delayed-listener
join or a successful real-page run.

The pinned Chrome 145 [target handler](https://github.com/chromium/chromium/blob/145.0.7622.0/content/browser/devtools/protocol/target_handler.cc)
sends `Target.detachedFromTarget` before removing the child session and
detaching the client; its `DetachFromTarget` response follows that operation.
The [DevTools session](https://github.com/chromium/chromium/blob/145.0.7622.0/content/browser/devtools/devtools_session.cc)
receives renderer notifications asynchronously. The detach marker therefore
orders messages already dispatched to the browser client but does not prove
that a page exception produced earlier has crossed the renderer boundary.
The [HTTP handler](https://github.com/chromium/chromium/blob/145.0.7622.0/content/browser/devtools/devtools_http_handler.cc)
posts outbound messages to the socket thread; the [WebSocket implementation](https://github.com/chromium/chromium/blob/145.0.7622.0/net/server/web_socket.cc)
stops sending once a client close frame is processed. A close handshake is not
a barrier for messages still awaiting dispatch. Neither alternative proves
the existing end-of-run error capture contract, and reset is still fatal.

The earlier assertion that this alternative requires a semantic decision is
superseded by independent review: the existing Node consumer checks page errors
before its receipt and `browser.close`, and this ticket does not require
observation after the page target closes. An ordered target-close boundary is
compatible with that contract. Define the observation interval as events
delivered to the browser-side session before an explicit correlated
target-close or session lifecycle marker, drain the ordered handler and join
listeners through that marker, then perform separately reported browser cleanup. Events still in the
renderer or produced after that marker would be outside the verdict, even if
their underlying page activity began during the checked flow. Invalid events
and exceptions received within the interval must still fail, and transport
loss before the marker must fail. Disposal failures are fatal, and a post-boundary
reset can be treated as expected only after all page boundaries are proved and
separate bounded cleanup and process termination succeed. This is not yet
implemented; it needs controls at the real consumer seam. No current evidence
establishes a stronger renderer-to-consumer barrier.

The independent disposal fix makes collision and qualification context disposal
fail the scenario verdict, retaining the scenario error if both fail. Its focused
regression check passed after failing on the ignored-disposal behavior. Formatting
passed; Clippy was blocked by an existing vendored Chromium lint. The pinned
Chrome 145 negative probe reached its controls but failed on the existing
WebSocket reset at shutdown. The ordered event boundary remains open.

The deterministic late-exception join control, successful Chrome probe,
full supported gate, parity, launch-error cleanup and closure review remain
open. Ticket 005 remains claimed.

### 2026-10-26: target marker listener delivery increment

The vendored handler now retires a target's listener queues at its active
session's detach marker as well as at target destruction. Retired queues are
delivered and closed while the transport remains open. A detach for another
session does not retire the active session's listeners; destruction removes
the target's remaining session mappings. Lifecycle markers carried by a parent
session are processed before routing the event to that parent. No public
visibility or transport reset handling changed. The existing redundant borrow
in the vendored browser URL formatting was also removed to resolve the previous
Clippy failure.

A controlled handler regression queues an event without polling its listener,
checks unrelated target and session markers, and then verifies delivery and
stream closure at each matching detach or destroy marker, including markers
carried by a parent session. Before the fix the detach case failed because the
stream stayed open (`.evidence/ticket005-target-marker-red.log`); the final
three vendored lifecycle tests passed (`.evidence/ticket005-target-marker-final-tests.log`).
The harness's 13 library tests passed before the final test-only refinement
(`.evidence/ticket005-target-marker-harness.log`); harness Clippy including
the vendored dependency and formatting passed on the final patch
(`.evidence/ticket005-target-marker-final-clippy.log`,
`.evidence/ticket005-target-marker-final-fmt.log`).

This proves delivery only for events already received by the handler; it does
not establish renderer-to-browser dispatch, a real consumer join boundary, or
successful Chrome shutdown. The prior Chrome reset and full-check failures
remain failures. The late-exception consumer control, successful Chrome probe,
full supported gate, parity, launch-error cleanup and closure review remain
open. Ticket 005 remains claimed.

### 2026-10-26: consumer target-close boundary

Starting from clean `b390a246287b81a0e016fa77ce1cdeb8fb6de794`, the drag
consumer explicitly closes its observed main and scenario pages, waits for
the matching target-destroyed event and that target's page-error listener to
finish, and disposes scenario contexts. Missing markers, transport loss,
listener failure, close and disposal failures remain fatal; scenario errors
are retained alongside cleanup failures. The negative probe uses the same
boundary. This observes events delivered before the marker, not events still
in the renderer. No handler visibility or reset handling changed.

The actual drag consumer control throws from `beforeunload` on page close.
On the base it exited nonzero without a receipt but omitted the close-time
exception (`.evidence/ticket005-boundary-red.log`); after the change it
reported that exception and the original assertion failure, still nonzero
without a receipt (`.evidence/ticket005-boundary-green.log`). This is a late
page exception control, not a forced scheduling delay of the listener. The
existing controlled handler test independently covers an unrelated target or
session marker and delayed listener delivery; all three lifecycle tests passed
(`.evidence/ticket005-boundary-lifecycle.log`). Harness tests passed 13 library
and one scenario verdict test, and harness Clippy and formatting passed
(`.evidence/ticket005-boundary-focused.log`, `.evidence/ticket005-boundary-fmt.log`).
The first lifecycle filter matched zero tests; the correctly named filter
produced the three-test result above.

The pinned Chrome 145 negative probe reached both context disposals and
collected its late rejection, but failed on the same WebSocket reset
(`.evidence/ticket005-boundary-probe.log`); the reset remains fatal. An
unnecessarily broad workspace test also failed two vendored Chrome integration
tests because their parallel processes shared a profile lock
(`.evidence/ticket005-boundary-checks.log`); this is not a passing workspace
test. Full supported check, parity, CI cutover, launch-error cleanup and closure
review remain open. Ticket 005 remains claimed.

### 2026-10-26: bounded Chrome teardown reset

Starting from clean `5aaefe715c4b56ddbf4ac1e7e6b7047d50504e14`, classify only
the WebSocket `ResetWithoutClosingHandshake` as expected when the consumer's
observed target boundaries completed, the handler was clean before close,
`Browser.close` was acknowledged, the separately waited child exited
successfully without a kill, and the handler and listeners drained. All other
handler errors and unsuccessful or incomplete cleanup remain fatal. The
boundary remains limited to events delivered before the target marker; it is
not a renderer-to-browser delivery guarantee.

The prior pinned Chrome probe failed on this reset after its target boundaries
(`.evidence/ticket005-boundary-probe.log`). On this change the Chrome 145
negative probe passed and collected its late rejection
(`.evidence/ticket005-reset-probe.log`). Real consumer controls killed the
consumer's own Chrome while its target was open and observed a fatal verdict
with no receipt; assertion, exception, rejection, and close-time exception
controls also failed without receipts (`.evidence/ticket005-reset-controls.log`).
The harness tests passed 13 library and one scenario test; build and Clippy
passed (`.evidence/ticket005-reset-focused.log`), as did formatting.

The Rust launcher passed 15 checks on pinned Chrome and verified receipt
`20261026T230931Z-b84b6c4f5700` on the existing site build digest
`6f9242d46d35eb540dfb9f5eaae11fc1e9ecfa83aafcd6c84f84df29f129cfdc`
(`.evidence/ticket005-reset-rust-smoke.log`). Its source identity is the dirty
implementation tree based on `5aaefe715`; this is not a fresh-build parity
receipt. The earlier failed probe and workspace test with shared profile-lock
failures remain failures. Full supported check, fresh parity, CI cutover,
launch-error cleanup and closure review remain open; ticket stays claimed.

### 2026-10-26: launch and profile cleanup

Starting from `3cc8782824eae667bf000e5fa071256ba19fd081`, bound the
vendored launch failure's child kill and reap, retain the original launch
error alongside cleanup failure, and preserve the owned profile whenever
termination is unconfirmed. Both consumers explicitly remove the run directory
after confirmed shutdown and treat removal failure as fatal before success.
The private harness uses the existing `ChromeMessage` error variant as the
conservative unconfirmed-cleanup signal; no public API was widened.

Focused controls exercised a spawned child failing initialization and verified
it was reaped, and forced a real directory-removal error and verified receipt
creation failed. Harness tests passed (14 library, 2 drag tests), Clippy, build,
and formatting passed (`.evidence/ticket005-launch-cleanup-final-tests.log`,
`.evidence/ticket005-launch-cleanup-final-clippy.log`, and
`.evidence/ticket005-launch-cleanup-final-build.log`). No pre-fix RED or
forced OS-level kill failure was obtained. Pinned Chrome 145 negative probe
passed (`.evidence/ticket005-launch-cleanup-final-probe.log`); the Rust drag
runner passed 15 checks and verified receipt `20261026T231400Z-e735e2a01127`
on the existing site build (`.evidence/ticket005-launch-cleanup-final-drag.log`).
This is not a fresh-build parity receipt. Earlier failed full and probe gates
remain failures; full supported check, fresh parity, CI cutover and independent
closure review remain open. Ticket 005 remains claimed.

### 2026-10-26: review follow-up for launch and profile lifetime

At `0d293bb`, independent review found the launch deadline ended at URL
resolution and successful launch cleared profile preservation before the child
was reaped. The same deadline now covers the WebSocket handshake and routes
failure through the existing bounded kill/reap path. A launched profile stays
preserved until the consumers explicitly remove it after confirmed termination.
The redundant receipt cleanup argument and its plumbing-only test were removed;
the consumer's existing verdict still checks cleanup before writing a receipt.

The stalled-handshake control failed on the original implementation at the
outer bound (`.evidence/review-handshake-red.log`), then passed and confirmed
the child was reaped. The launch-cancellation profile control also failed on
the original implementation (`.evidence/review-cancellation-red.log`) and
passed after the fix. All 16 library tests and the drag binary test passed;
Clippy, build and formatting passed (`.evidence/review-focused-tests.log`,
`.evidence/review-drag-tests.log`, `.evidence/review-clippy.log`,
`.evidence/review-build.log`, `.evidence/review-fmt.log`). The pinned Chrome
145 negative probe passed (`.evidence/review-probe.log`). The real consumer
control forced a profile-removal error, observed it in the final error and saw
no receipt (`.evidence/review-consumer-controls.log`); the scenario also failed,
so this does not isolate cleanup as the only failure. No OS-level kill failure
was forced. The Rust drag smoke passed 15 checks on pinned Chrome 145 against
the existing site build (`.evidence/review-smoke.log`); this is not fresh-build
parity. Full supported check, fresh parity, CI cutover and independent closure
review remain open; ticket stays claimed.

### 2026-10-26: fresh parity on reviewed head

On clean `3eea7b854aab261219a7a08e4e12bf4e50edf77a`, the supported
`python3 scripts/check.py build` passed under the build lock and capped
`boardstudio-tests.slice` (60.4 s). The build regenerated the packaged site;
its own freshness checks reused current provider artifacts. Sequential Node and
Rust launcher runs in the same slice both passed 15 checks with Chrome for
Testing `145.0.7622.0` and the locked, vendored chromiumoxide `0.9.1` patch.
The launcher verified both receipts: source head and clean source digest
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`, site digest
`f96920045ed4f07eba4cf815e50dcf6d21f5870702ce87f894c0ef61278645ef`, browser,
viewport, ordered check names and passing verdict agree. Node receipt:
`20261026T232050Z-e8bee5c15895`; Rust receipt: `20261026T232152Z-5602bc62f0a2`,
under `web/target/test-results/drag-browser/`. Logs are
`.evidence/ticket005-fresh-build-3eea7b85.log`,
`.evidence/ticket005-fresh-node-3eea7b85.log` and
`.evidence/ticket005-fresh-rust-3eea7b85.log`. The Rust run reported the
post-boundary WebSocket reset classified as expected after clean child exit.
The preceding reviewed-head negative probe is recorded above and was not rerun.

This comparison does not close the full supported gate: the previous Parts and
Keycaps failures remain open, as do CI cutover and independent closure review.
Ticket 005 remains claimed.

### 2026-10-26: Parts upload and Keycaps resource investigation

The original full receipt `.evidence/drag-save-undo-default-6346fe98.json`
remains failed: Parts first failed on a missing uploaded parameter and Keycaps
did not start its tests after a renderer script-load timeout. The page suite
was not run. The separate concurrent diagnostic had a Library rename timeout;
it is not an original full-receipt failure.

In `upload_preview_refresh_does_not_keep_a_later_failed_apply_draft`, the test
drains held effects and checks for the accepted asset for at most 100 iterations
with a 20 ms timer, then indexes the uploaded parameter. The prior injected
2.5-second delay reproduced the missing-key failure before the upload landed
(`.evidence/c7e1-inject-2500-full-parts.log`); this proves that delay can cause
the symptom, not that it caused the original failure. Prior CPU-stress probes
also produced renderer script-load timeouts, without establishing the original
Keycaps cause. The shared bounded wait would extend this test's bound; no such
change or product fix was made without evidence that it addresses the original
failure and preserves the required failure signal.

On clean `2612afc329ef362741ad63d223579399a35a0d76`, the supported partial
`check.py browser --only keycaps,parts --keep-going` ran under the build lock
and `boardstudio-tests.slice` with `BOARDSTUDIO_CHECK_JOBS=1`. Keycaps passed
31/31 and Parts 68/68 (`.evidence/ticket005-serial-keycaps-parts.json` and
`.evidence/ticket005-serial-keycaps-parts.log`). This single serial subset
shows a usable scheduler setting for this run, not a proven stable resource
envelope or a causal explanation for the earlier failures. It does not replace
the failed full receipt. Full supported verification, CI cutover, and closure
review remain open; no default driver was changed.

### 2026-10-26: Parts and Library test diagnostics

The Parts upload test now waits for the accepted asset with a named 2,000 ms
bound, preserving the queued Apply before the upload effect runs. It waits for
the projected filename in place of the 30 ms sleep, and removes the helper's
unreachable false branch. The Library rename wait retains its 5,000 ms bound
and accepted-name assertion; a timeout now reports accepted name, lifecycle,
durability and status, including when no accepted snapshot exists.

Prior RED evidence: the original full receipt failed Parts with a missing key
(`.evidence/drag-save-undo-default-6346fe98.log`); the instrumented 2.5-second
delay also failed with the missing key after 100 iterations
(`.evidence/c7e1-inject-2500-full-parts.log`). That injected run is a diagnostic,
not a clean reproduction or proof of the original cause. A separate concurrent
diagnostic failed the Library's gated rename wait after 5,000 ms
(`.evidence/drag-diagnostic-concurrent-6346fe98.log`); its cause remains unknown.
No new RED was induced for these diagnostic corrections.

GREEN: the serial, resource-sliced partial browser run passed Parts 68/68 and
Library 18/18 (`.evidence/ticket005-correction-browser.log` and receipt). The
Library predicate changed while that run was in progress, so its final source
was separately checked: 18/18 passed (`.evidence/ticket005-correction-library-final.log`
and receipt). These results do not establish the original failure causes or
replace the failed full gate. Keycaps was not changed or retested; its original
failure occurred before tests started. Full browser verification, CI cutover,
and independent closure review remain open.

### 2026-10-26: Spec follow-up

Based on `0eceab475b05553ff3458275d466bba7be7c0aef`, this follow-up
uses exclusively created random run directories, limits pending wait and click
operations by their existing deadlines, and retries only transient
pre-dispatch click states. The abandoned PID/counter directory control failed
before the change and passed after it (`.evidence/ticket005-spec-collision-red.log`,
`.evidence/ticket005-spec-final-tests.log`). A pending page promise exceeded
its wait deadline on pinned Chrome 145 before the change
(`.evidence/ticket005-spec-deadline-pinned-red.log`); the updated pinned negative
probe passed (`.evidence/ticket005-spec-pinned-probe.log`). An earlier control
accidentally used Chrome 156 and also failed with a protocol mismatch
(`.evidence/ticket005-spec-deadline-red.log`); it is not a pinned receipt.

The final scoped harness tests passed (17 library, 1 drag binary); Clippy and
formatting passed (`.evidence/ticket005-spec-final-tests.log`,
`.evidence/ticket005-spec-clippy.log`, `.evidence/ticket005-spec-fmt.log`). The
pinned Chrome 145 Rust drag smoke passed 15 checks against the existing site
build (`.evidence/ticket005-spec-drag.log`), not a fresh-build parity run.
The real click consumer was exercised through a local CDP relay to pinned
Chrome 145. The relay rejects the first selected press or release with a
distinct protocol error and forwards all other commands. With the original
click implementation restored, the press control failed: two dispatches,
success returned, and one page click (`.evidence/ticket005-dispatch-original-red.log`).
With the fix, both press and release through both backend and role click
consumers returned the original protocol error after one dispatch and recorded
zero page clicks (`.evidence/ticket005-dispatch-final-green.log`). Run the control
with `CHROME_BIN=<pinned-chrome> cargo test --manifest-path tools/browser-harness/Cargo.toml --lib dispatch_failure_is_not_retried_or_masked -- --ignored --nocapture`.
The WebSocket reset printed during clean browser shutdown is recorded in both
logs. The normal harness tests passed (17 library, 1 drag binary; the browser
control is run explicitly); formatting and final Clippy passed
(`.evidence/ticket005-dispatch-tests.log`, `.evidence/ticket005-dispatch-final-fmt.log`,
`.evidence/ticket005-dispatch-final-clippy.log`). The initial Clippy run failed
on a nested conditional in the new control and is preserved in
`.evidence/ticket005-dispatch-clippy.log`. Prior full-gate failures, full
supported verification, CI cutover and independent closure review remain open.

### 2026-10-26: In-flight click timeout recheck

At `a89b3e9b7286a786436811a322d54bfe0add5261`, the click operation's
outer deadline could cancel a `CommandFuture` after CDP had sent a mouse press
or release. The pinned Chrome 145 relay forwarded each selected input and
withheld its response: both backend and role clicks returned an actionability
timeout and left the target usable (`.evidence/ticket005-inflight-red.log`).
The actionability deadline now ends before dispatch; a deadline during input,
including chromiumoxide's own command timeout, reports an ambiguous input and
attempts bounded target termination. Other protocol errors remain fatal and
unchanged. The relay control confirmed each command reached Chrome, returned
one ambiguous failure without retry, and found the target closed in all four
press/release and backend/role cases (`.evidence/ticket005-inflight-final.log`).
The existing protocol-error relay also passed for all four cases
(`.evidence/ticket005-inflight-protocol.log`). The drag consumer propagates
scenario failures before writing a receipt; the relay control did not inject
this fault into the full receipt-writing process.

Partial profile/config creation now explicitly closes its temporary directory
and reports cleanup failure alongside the creation failure. That exceptional
cleanup failure was not injected. Harness tests passed 17 library and one drag
binary test (two Chrome controls ignored in the ordinary suite), and Clippy and
formatting passed (`.evidence/ticket005-inflight-tests.log`,
`.evidence/ticket005-inflight-clippy.log`, `.evidence/ticket005-inflight-fmt.log`).
The pinned Chrome 145 negative probe passed and the Rust drag smoke passed 15
checks against the existing site build (`.evidence/ticket005-inflight-probe.log`,
`.evidence/ticket005-inflight-drag.log`); this is not a fresh-build parity run.
Two intermediate builds failed to compile while implementing close-result
formatting; the final controls and checks above ran after those corrections.
The earlier failed full receipt remains failed; full supported verification,
CI cutover, and independent closure review remain open.

### 2026-10-26: refreshed parity after harness changes

On clean `dda082fddd1ad4cae24de4da4fc0828c4bc5038f`, the supported
`python3 scripts/check.py build` passed in 57.6 seconds under the build lock and
`boardstudio-tests.slice`. The checked build regenerated the packaged site and
reused current provider artifacts. Sequential Node and Rust launcher runs in the
same slice each passed 15 checks with Chrome for Testing `145.0.7622.0`.
The launcher verified both receipts. They agree on source HEAD, clean source
digest `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`,
site digest `44c6a27042b19479c66d4c50a16814dd9ea54b1f138ee99cb70c4f726097aeed`,
1513 x 1043 viewport, ordered check names and overall passing verdict. Timing and
measured values vary. Node receipt:
`web/target/test-results/drag-browser/20261026T233809Z-8537c0ec1540/receipt.json`;
Rust receipt:
`web/target/test-results/drag-browser/20261026T233918Z-38e529d497f3/receipt.json`.
Logs: `.evidence/ticket005-fresh-build-dda082fd.log`,
`.evidence/ticket005-fresh-node-dda082fd.log`, and
`.evidence/ticket005-fresh-rust-dda082fd.log`. Build receipt:
`.evidence/ticket005-fresh-build-dda082fd.json`. The Rust run reported the
expected post-boundary WebSocket reset after clean child exit.

This is a local same-build comparison, not a full gate or CI cutover. The prior
full receipt remains failed on Parts and Keycaps; full supported verification,
CI cutover and independent closure review remain open. Ticket 005 stays claimed.

### 2026-10-26: consumer-control registration and process discovery

The default check at `89c0fe050` failed in tooling because the consumer-control
script was unregistered (`.evidence/full-default-89c0fe05-20261026/check.log`).
The Rust launcher now invokes the controls and the registration check records
that delegation. The pre-boundary transport-loss control remains necessary:
the controlled peer test drops transport after `Browser.close`, whereas this
control interrupts the actual drag consumer while its target is open and checks
for a fatal shutdown and no receipt.

The prior targeted run reported five controls passing and transport discovery
failing without a kill; no log was saved. A no-kill real-consumer probe found
Chrome owned by a Tokio worker task, not the leader task, and the selected
`/usr/bin/chromium` resolved to a running `/usr/lib/chromium/chromium`
(`.evidence/ticket005-process-discovery-probe.log`). The first profile-based
fix still failed without a kill (`.evidence/ticket005-consumer-discovery-fix.log`);
a second probe showed Chromium had rewritten its command line into one argument
(`.evidence/ticket005-profile-discovery-probe.log`). The control now finds a
direct child across the consumer's tasks by its unique isolated profile and
signals it through a pidfd. All six controls passed on local Chromium 156,
including pre-boundary termination, fatal shutdown and no receipt
(`.evidence/ticket005-consumer-discovery-final.log`). Chromium 156 also emitted
protocol errors in other controls; these are not a successful browser smoke.
The script-registration test passed 31/31
(`.evidence/ticket005-registration-check.log`). The earlier failed full check
and intermediate control failure remain failures.

The single default full check on committed `b8779283`, with `NODE_PATH`, the
build lock, `TMPDIR=/home/chris/.cache/tmp` and `boardstudio-tests.slice`, exited
1 (`.evidence/ticket005-full-b8779283.log`). Repo, tooling, lint, build and
native test phases passed. The default Node drag smoke passed 15 checks, the
CAD browser smoke passed 364 checks, Parts passed 68/68 and Library passed
18/18. Keycaps failed before identifying a test: the runner timed out loading
scripts and ChromeDriver reported a renderer timeout. This is not a passing
full gate or an identified cause of the earlier Keycaps failure. CI cutover and
independent closure review remain open; ticket 005 remains claimed.

### 2026-10-26: supported Rust launcher with integrated controls

On clean `835d41c04396daaa7be7f1ffbbeec6269683caa8`, the checked
`python3 scripts/check.py build` passed in 59.6 seconds under the project build
lock and `boardstudio-tests.slice` with CPUQuota 800%. Its partial build receipt
is `.evidence/ticket005-rust-e2e-835d-build.json` and log is
`.evidence/ticket005-rust-e2e-835d-build-corrected.log`. An initial service
invocation used the wrong working directory and failed before starting the build;
its original log is `.evidence/ticket005-rust-e2e-835d-build.log`.

The supported Rust launcher ran once in the same CPU envelope, building its
locked harness under the project lock and using Chrome for Testing
`145.0.7622.0` and chromiumoxide `0.9.1`. All six integrated negative controls
(assertion, exception, rejection, close-exception, cleanup, and transport-loss)
exited 1 without a receipt and met their expected error checks. The actual smoke
passed all 15 checks and the launcher verified its receipt. The receipt records
clean source digest `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`,
site digest `783c8373704b2bef50fe6e48b8fe86fa64408982f7af9454a22fcbde7ddc0029`,
and viewport 1513 x 1043. It is at
`web/target/test-results/drag-browser/20261027T003552Z-a5170fe79339/receipt.json`;
the launcher log is `.evidence/ticket005-rust-e2e-835d-launcher.log`. The run
reported the expected post-boundary WebSocket reset after clean child exit.

This is scoped Rust launcher evidence, not fresh Node/Rust parity, full default
acceptance, or CI cutover. The prior Keycaps startup failures remain unexplained;
the instrumented full pass does not establish their cause or resolution.

### 2026-10-26: closure-review follow-up for CI and consumer controls

Moved the six consumer fault controls out of the ordinary Rust launcher into a
separate Linux CI step after the supported web check builds the harness. Each
control retains its expected failure and no-receipt assertion. The CI web check
selects the Rust driver and passes the provisioned Chrome 145 through the
drag-specific browser override; the launcher supplies it as `CHROME_BIN` to the
driver. Explicit `CHROME_BIN` still takes precedence. Other browser consumers
and the mounted WASM ChromeDriver keep their existing environment. The local
Node default remains available for comparisons. CI failures in either step fail
the job; hosted CI has not been exercised.

On source `46f09a11b52cadb11365f7f73ba444a82651fd72` with the implementation
diff present (source digest `18a49299bbfee450f65326c191597e8adb277bd3c4c9ce8c6689d92d02124615`),
the checked build passed and sequential Node and Rust launchers each passed
15 checks on the same build digest
`ed53fb72108110b22e932fab4925cc742528c8eda23cca1e18f62ed36dc6ea60` and
Chrome 145. The launcher verified both receipts; ordered check names and source
and build identities match. Receipts: `20261027T003955Z-427938c65a9b` (Node)
and `20261027T004050Z-6e20b64413b5` (Rust), under
`web/target/test-results/drag-browser/`. Logs: `.evidence/ticket005-closure-fix-build.log`,
`.evidence/ticket005-closure-fix-node.log`, `.evidence/ticket005-closure-fix-rust.log`.
The six focused controls passed their failure/no-receipt checks
(`.evidence/ticket005-closure-fix-controls.log`). The browser-selection test
first failed on the wrong override precedence and passed after correction;
`check.py repo tooling lint` passed (`.evidence/ticket005-closure-fix-check.log`).

This is scoped local evidence, not hosted CI or a new full supported check.
Earlier full failures remain failures; the instrumented Node-default full pass
does not establish the cause or resolution of prior Keycaps startup failures.
Ticket remains claimed pending the remaining closure gates and review.

### 2026-10-26: CI wiring review follow-up

The Rust launcher creates the build-lock parent before invoking `flock`. Cargo
metadata, run from the same repository directory and environment as the locked
build, supplies the target directory for both the launcher and the separate
consumer controls. This honors Cargo's target-directory configuration and
relative environment overrides without changing CI's Rust pin or other drivers.

Before the fix, a fresh `HOME` failed at `flock` because the lock parent did
not exist; Cargo metadata also showed a relative `CARGO_TARGET_DIR` resolved
away from the hard-coded binary. After the fix, a real locked harness build
from a fresh home succeeded using a relative target override, and all six
consumer controls passed on pinned Chrome 145
(`.evidence/ticket005-wiring-check.log`). The override pointed through a
symlink to the existing harness target, so this was not a clean rebuild. A
separate relative Cargo config target resolved correctly; a marker wrapper at
that configured location confirmed all six controls executed that binary
(`.evidence/ticket005-wiring-config-controls.log`). The supported Rust launcher
also passed 15 checks and verified its receipt with a relative target override
pointing to the existing target (`.evidence/ticket005-wiring-launcher.log`).
`python3 scripts/test-check.py` passed 31 tests, Python compilation and
`git diff --check` passed. No fresh full check or hosted CI was run. Earlier
full failures remain failures, and the instrumented full pass does not establish
their cause or resolution; remaining closure gates and review stay open.

### 2026-10-26: launcher build-target recheck

Cargo metadata reports the target directory but not the executable's target
triple or platform suffix. With `CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu`,
a real locked build reported `target/x86_64-unknown-linux-gnu/debug/drag-smoke`
while the original resolver returned `target/debug/drag-smoke` (RED). The
launcher and standalone consumer controls now each build under the existing
lock and use the executable from that build's Cargo artifact message. This
also removes the metadata call. Captured build errors include Cargo stderr and
rendered compiler diagnostics with credential redaction.

The same custom-target build returned the actual existing executable (GREEN).
All six standalone controls passed their expected failure/no-receipt checks on
local Chromium 156 (`.evidence/launcher-target-controls.log`). An invalid target
surfaced Cargo's target-specification error (`.evidence/launcher-target-error.log`);
the focused browser-tools tests passed 8/8, `test-check.py` passed 31/31, and
Python compilation and `git diff --check` passed. The custom-target launcher
ran against the existing site but failed with Chromium 156 protocol-message
errors (`.evidence/launcher-target-smoke.log`); it is not a passing smoke.
No full check, pinned-browser run, hosted CI, or closure review was performed;
the ticket remains claimed and earlier failures remain failures.

### 2026-10-26: final-gate consumer controls and lock blocker

On clean `beb431dc2451853a7b30fb0d11bc106e58419065`, the six standalone
Linux consumer controls passed their expected failure and no-receipt checks,
using Chrome for Testing `145.0.7622.0` and the `boardstudio-tests.slice`
8-CPU envelope. The harness build acquired the project build lock. The CI-step
log is `.evidence/ticket005-final-beb431dc/consumer-controls-ci.log`.
An initial invocation supplied only the drag-specific browser variable, but
this standalone script requires `CHROME_BIN`; it failed at the first control
before starting Chrome. That setup failure remains recorded in
`.evidence/ticket005-final-beb431dc/consumer-controls.log`; the CI-configured
invocation above corrected the environment.

The full default check was not run: holding the project build lock around the
whole scheduler would deadlock when the Rust drag launcher independently
acquires that same lock, while omitting the outer lock would leave the other
build phases unprotected. The full receipt, artifact matrix and browser/site
acceptance are therefore not available for this source. Hosted CI remains
unrun; previous full failures and the unexplained Keycaps failures remain
unchanged. Ticket 005 stays open pending a supported lock-safe full gate.

### 2026-10-26: build-lock cancellation repair

The validated inherited lock descriptor now reaches serial and parallel scheduler
children and the Python web, CAD, WASM and Rust harness build subprocesses. The
standalone harness retains its `flock` path; an environment value without a valid
descriptor still fails validation. Build tools inherit the descriptor across exec.

The process-level regression uses FIFOs to hold a representative Cargo/wasm-pack
child, kills its launcher, and checks an unrelated nonblocking contender before and
after releasing the child. It covers serial and parallel scheduler launches, web,
CAD and WASM helpers, the harness, standalone `flock`, and the nested scheduler to
harness path, plus a fork-and-exec build grandchild; the standalone, nested and
grandchild cases also kill the intermediate lock holder. Before the fix,
the harness case allowed the contender through while Cargo was alive (RED,
`.evidence/lock-cancellation-red.log`); all nine cases passed after the fix
(`.evidence/lock-cancellation-green.log`). Focused browser-tools, scheduler,
web-build, WASM-runner and CAD-build tests passed; Python compilation and
`git diff --check` passed. No Cargo build, full check, browser acceptance or hosted
CI was run for this repair. Ticket closure and earlier failures remain unchanged.

This shared-lock protocol is Unix-specific (`fcntl.flock` and inherited file
descriptors); standalone locking additionally requires the `flock` utility. The
lock is inode-based: removing or replacing the lock file while a build is active
would let a new contender lock a different inode, so the shared lock file must not
be removed or replaced during builds.
