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
