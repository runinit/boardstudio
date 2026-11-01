# 006: Rust CAD runtime capture

Status: resolved
Type: build
Blocked by: [Rust drag smoke end to end](005-rust-drag-smoke.md)
Parent: [Chromiumoxide browser checks](https://github.com/runinit/boardstudio/blob/511ae990a7205d9228e5d09eb7d46f8340155f8b/docs/plans/chromiumoxide-browser-checks/map.md)
Related: [Real-browser checks drive Chrome from Rust](../../cad-deepening/issues/016-rust-browser-harness.md)

## What to build

The existing CAD measurement workflow invokes the Rust harness instead of Playwright and
produces equivalent measurement artifacts and verdicts. Extend the shared harness only where
this consumer requires it; keep the old capture available until comparison passes.

## Acceptance

- [x] Preserve launcher inputs, capture arguments, timeouts, exit codes, CSV columns/events
  log fields, source/build binding, readiness/currency checks and failed-capture semantics.
- [x] Old/new capture against the same packaged build and inputs proves equivalent events,
  statuses and output schema. Compare nondeterministic timing fields by their documented
  meaning, not byte identity; do not claim a speed improvement from this port alone.
- [x] Cold/warm HTTP-cache and service-worker conditions match the old runner. Prove bypass
  versus blocking explicitly, including a controlled worker fixture; retain response status
  and service-worker-origin metadata without dropped required events.
- [x] Preserve protocol self-check coverage without a browser. Keep any Node-based tooling
  tests that still serve that contract; removing all Node is not this migration's goal.
- [x] Shared isolated browser launch/error handling is reused. Negative controls prove
  malformed replies, page failures, capture identity drift and incomplete observations fail.
- [x] Run affected tooling tests and real old/new capture parity before switching the default;
  retain the old runner if any required proof is blocked. No production CAD behaviour changes.

## Verification and scope

Use TDD at the existing public launcher/output seams: demonstrate the expected regression
before the fix, then run affected tests and typechecking during development. At closure run
the full supported check and combined Spec + Standards review; record terminal verdicts,
source/build identity and any blocked gates honestly. No weakened assertions, masked page
errors, baseline deletions or retry-until-green. Preserve the archived prototypes and unrelated
work. Builds follow repository concurrency/lock rules; evidence uses supported receipts and
logs, not custom hash manifests. All checks must be runnable by humans and CI without agent tools.

## 2026-10-26: implementation inventory and baseline

Ticket 005 was resolved for local scope at `6bcadff`; hosted CI remains separate.
The current launcher in `cad/scripts/measure-cad.py` checks the build and source
binding, staged package, artifact currency and post-capture drift; it still invokes
`measure-cad-runtime.cjs`. That driver owns the CSV and event output, worker reply
identity and completeness checks, response status and service-worker metadata,
cold-cache and warm-start behavior. The U1 path in `measure-cad-page.cjs` also
requires real page interaction, file upload and downloads. The shared Rust harness
provides isolated launch, page errors, evaluation and input; the CAD response and
cache/service-worker observation and U1 paths still need implementation and proof.

On `6bcadff`, `python3 -B cad/scripts/test-measure-cad.py` passed 16 tests and
`node cad/scripts/test-measure-cad-runtime.cjs` passed five controlled protocol
checks plus STL controls. These are baseline tooling checks, not browser or
old/new parity evidence. No Rust CAD capture, regression RED/GREEN, cache fixture,
negative controls, real capture parity, full check or closure review has been
completed for ticket 006. The existing runner remains the default and tickets 007
and 008 have not been advanced.

## 2026-10-26: partial Rust worker-startup diagnostic

Added `cad-response-probe` to the existing isolated Rust browser harness. Given
the URL of the performance host on a server also serving the packaged worker at
`/boardstudio/assets/cad-worker/entry.js`, it starts the real worker, waits for
`ready` with the existing 180-second bound, and prints a diagnostic startup time
and page-target response URL/status/service-worker fields. Run it with
`CHROME_BIN=<chrome> cargo run --locked --manifest-path tools/browser-harness/Cargo.toml --bin cad-response-probe -- <host-url>`
under the project's build lock. It is not connected to the measurement launcher,
does not emit measurement CSV or a receipt, and labels its output diagnostic and
its responses page-target-only.

On the existing, unbound site artifacts and Chrome for Testing 145.0.7622.0,
the probe started the packaged worker and observed the host 200 and favicon 404
(`.evidence/ticket006/worker-final-browser.log`). Worker module requests were not
present in the page-target response stream; their treatment in the existing
runner and any additional target observation need direct comparison before a
complete resources event can be claimed. No source/build provenance is asserted
for this diagnostic. A missing-worker control exited 1 with no stdout; after a
shutdown error initially masked the page failure, both errors are now retained
(`.evidence/ticket006/worker-negative.log` and `worker-negative-fixed.log`).
The focused Rust unit test and clippy passed, as did the existing 16 Python
tooling tests and five Node protocol checks. The initial response-field test
failed to compile before implementation (`.evidence/ticket006/response-red.log`).

This is a partial diagnostic, not the requested capture migration: the supported
launcher, complete operation and U1 paths, response completeness, cache and
service-worker controls, source-bound old/new comparison, negative controls,
full check and closure review remain open. The Node capture remains the default;
ticket 006 stays claimed and 007/008 are unchanged.

## 2026-10-26: worker-operation diagnostic and response comparison

The Rust probe accepts an optional non-startup worker workload and evaluates the
existing Node driver's preparation and operation checks, including reply identity,
revision, and output validation. It is still a diagnostic: it extracts the
existing page callback by source markers at compile time and fails if they are
missing; this needs a shared module before launcher integration. It does not emit
the capture protocol or provide U1, cache, service-worker, or complete responses.

With Chrome for Testing 145.0.7622.0 on the existing unbound site, C1 preview
completed with 492 triangles and 35424 buffer bytes
(`.evidence/ticket006/operation-browser.log`). A missing C1 fixture exited 1 with
empty stdout (`operation-negative.log`). The prior binary rejected the workload
argument (`operation-red.log`); this is a CLI capability control, not a browser
regression test. The Rust probe test passed 1/1 and clippy passed; the existing
Python tests passed 16/16 and Node protocol checks passed 5 plus STL controls.

A separate Node context-response diagnostic on the same unbound site and pinned
Chrome observed 200 responses for `cad-worker/entry.js`, `m1_cad_worker.js`,
`m1_cad_worker_bg.wasm`, and both CAD provider assets
(`.evidence/ticket006/node-worker-responses.log`). The Rust operation probe
observed only host, favicon, and fixture responses. These runs establish a
response-coverage gap, not that worker CDP capture is impossible. The supported
launcher, U1, full source-bound capture, parity, cache/service-worker controls,
negative controls, full check, and closure review remain open; Node remains the
default. No acceptance checkbox has been cleared.

The supported checked `measure-cad.py build --state noop` completed at source
`e666646ad901af371d523b2300adf1a800390217` under the shared build lock
(`.evidence/ticket006/source-build/command.log`, exit 0). On that same packaged
site, sequential direct Rust probe and Node validation-driver C1 preview runs
both exited 0 and reported 492 triangles, 35424 buffer bytes, and zero STEP
bytes (`.evidence/ticket006/source-diagnostic.log` and
`source-diagnostic-node/events.log`). A post-run `measure-cad.py bind` exactly
matched the build's `binding-after.log`. The Node driver observed the five CAD
worker/provider responses; the Rust probe observed none of those, only host,
favicon, and fixture responses. These direct diagnostic runs are not launcher
captures or full parity: their response schemas and cache setup differ, and U1,
the rest of the operation matrix, and failure controls have not been compared.

## 2026-10-26: dedicated-worker response capture slice

Chrome for Testing 145.0.7622.0 CDP probing on the `e666646` source-bound
packaged site established that the host and favicon responses are delivered on
its page session, while all five CAD worker/provider responses are delivered on
its dedicated-worker session. Pausing attachment, enabling Network on the worker,
and only then resuming captured the initial worker script as well as its imports
(`.evidence/ticket006/cdp-target-matrix.log`). A second auto-attached session
resuming its worker did not prevent this sequence in the observed Chrome run
(`cdp-multi-session.log`); it is not proof for other Chrome versions.

The diagnostic now attaches its own flattened page session and captures page and
paused dedicated-worker responses through target detach. It rejects unsupported
attached targets, failed commands, malformed CDP messages, a missing worker, and
an incomplete boundary; page exceptions and the shared handler's invalid-message
handling remain fatal. Redirect responses are read from the subsequent request
CDP event. The real Chrome fixture initially failed because its 302 was missing
(`target-redirect-red.log`), then passed with 200, 201, 302 and 404 response
controls and fatal page-exception/missing-worker controls
(`target-final-real-controls.log`). A controlled protocol test rejects a failed
CDP command and malformed JSON; the probe tests passed 2/2 and harness clippy
passed (`target-final-tests.log`, `target-final-clippy.log`). Run the real fixture
with `CHROME_BIN=<chrome> python3 tools/browser-harness/test-cad-target-responses.py`
after building the probe.

A sequential Node/Rust C1 preview comparison on the same previously bound site
passed with all seven Node URL/status/service-worker response records present
once in Rust, including all five worker/provider assets; Rust additionally saw a
favicon 404. Both reported 492 triangles, 35424 buffer bytes and zero STEP bytes
(`target-final-comparison-counts.log`). The comparison is reproducible with
`CHROME_BIN=<chrome> NODE_PATH=<playwright-node-modules> python3 tools/browser-harness/compare-cad-responses.py`;
it does not independently validate build binding. An initial invocation without
those environment variables failed before the comparison (`target-final-comparison.log`).
The separate attachment and Network enablement add startup instrumentation overhead;
no timing equivalence or speed claim follows from this diagnostic.

This is still not the launcher capture or ticket closure. Service-worker-origin
true, cache and service-worker controls, other workloads, U1, full event/schema and
failure parity, supported full check and closure review remain unverified. The
Node runner remains the default and all acceptance boxes remain open.

## 2026-10-26: shared worker callback and build-lock handoff

The Node capture and Rust diagnostic now use the same in-page worker preparation
function, `cad/scripts/measure-cad-worker.cjs`, without extracting JavaScript by
source markers. Its function body matches the prior Node callback exactly; the
Node controlled protocol checks passed five cases plus STL controls. The Rust
probe's two focused tests and a locked build passed. This does not yet connect
the Rust diagnostic to the supported launcher or implement a capture consumer.

A source-bound build attempt failed before compilation because `capture()` closed
the inherited build-lock descriptor (`.evidence/ticket006-shared-source-build/command.log`).
A process-level regression reproduced the child `Bad file descriptor` failure
(`.evidence/ticket006-shared/lock-red.log`); the launcher now passes the validated
lock descriptor into the capture subprocess, and all 17 Python tooling tests pass
(`.evidence/ticket006-shared/lock-green.log`). This fixes the cause of that failed
attempt; the failed receipt remains failed.

The corrected checked noop build passed on clean source
`ad18d53a6` under the shared lock (`.evidence/ticket006-shared-source-build-fixed/`),
and a post-comparison binding matched its binding-after file exactly. Sequential
C1 preview diagnostics on that site with Chrome for Testing 145 returned 492
triangles, 35424 buffer bytes, and zero STEP bytes in both drivers. All seven Node
URL/status/service-worker records occurred once in Rust; Rust additionally observed
favicon 404 (`.evidence/ticket006-shared/comparison.log`). The preceding dedicated
worker fixture observed host and favicon responses on the page session and worker
assets on the dedicated-worker session, including redirects via request events.
The separate attachment pauses the worker for Network enablement, which affects
startup timing; these comparisons establish neither timing equivalence nor speed.

Node remains the launcher default. The Rust consumer and launcher selection,
startup and U1 workflows, complete output and failure parity, controlled cache and
service-worker behavior, full workload parity, full check, hosted CI and independent
closure review remain open. No acceptance checkbox is cleared by this slice.

## 2026-10-26: opt-in C1 Rust launcher capture

`measure-cad.py runtime --driver rust --workload C1plate:preview` now builds and
runs the Rust consumer through the existing launcher and binding checks. Node is
still the default; Rust explicitly rejects other workloads. The consumer uses the
shared worker validation and dedicated-worker response capture, emits the existing
C1 event/CSV fields, and applies the cold cache setting and the same pre-navigation
service-worker registration block used by the installed Playwright runner. This is
an opt-in C1 slice, not ticket closure.

The first attempted source-bound build at `3ab19470` failed before compilation due
to a Python syntax error in the shared build helper
(`.evidence/ticket006-consumer-build-terminal.log`). The error was corrected and
recorded separately. The final checked noop build at clean source
`51ccacaebe18555e67eac709368543d5c45020e9` passed under the shared lock and
`boardstudio-tests.slice`, 82.81 seconds
(`.evidence/ticket006-consumer-final-build/`). Sequential supported runtime
captures on that build both exited 0, with identical before/after bindings
(`.evidence/ticket006-consumer-final-node/` and `ticket006-consumer-final-rust/`).
The comparison (`.evidence/ticket006-consumer-final-comparison.log`) checked 25
event names and fields, non-timing values, and 25 CSV rows. All six samples in each
runner contained 492 triangles, 35424 buffer bytes and zero STEP bytes. Each of
Node's seven URL/status/service-worker records occurred once in Rust; Rust also
captured a favicon 404. Node/Rust worker-create-to-ready samples were 124.59 ms and
110.00 ms respectively; Rust pauses its worker for additional response attachment,
so the samples are not equivalent timing or a speed claim. These observations have
no benchmark or quiet-window admission.

On the prior `b23a8805` checkpoint, real Chrome controlled fixtures rejected a
missing input, identity drift, missing bodies, a failed worker reply, a malformed
mesh and a page exception with nonzero exits and `result.failed=true`
(`.evidence/ticket006-consumer-controls.log` and `ticket006-consumer-malformed.log`).
The last code change moved the failure event before resources; on final source a
malformed-mesh control again exited 1 with event order failure, resources, result
(`.evidence/ticket006-consumer-final-failure-order.log`). These controls are not a
complete old/new failure comparison; the temporary control harness is not a CI test.

The launcher selection regression was RED before implementation
(`.evidence/ticket006-consumer-red.log`); 18 Python measurement tests, 12 browser
helper tests, the five Node protocol checks plus STL controls, two Rust response
unit tests and final Rust Clippy passed. U1, other workloads, controlled cold/warm
cache and service-worker-origin parity, complete failure/output parity, the full
supported check, hosted CI and independent closure review remain open. The favicon
extra and worker instrumentation timing prevent claiming exact response/timing
parity. No acceptance checkbox is cleared.

## 2026-10-26: opt-in worker workload matrix

The Rust launcher now accepts all 15 non-startup worker workloads in the existing
schedule and invokes the shared Node worker preparation and validation callback.
STEP repetitions start fresh workers and emit `fresh-export-worker-ready`, as in
the Node driver. Startup and U1 remain explicitly unsupported; Node remains the
default. This is a worker slice, not ticket closure.

The launcher selection test failed on C2 before the change
(`.evidence/ticket006-worker-selection-red.log`) and the 18 Python tooling tests
passed afterward (`ticket006-worker-python.log`). The Node protocol controls
passed five cases plus STL controls (`ticket006-worker-node-tests.log`). On
`f0b7c212fc4a2b1b665a749f0c1511ded0a977ce`, a checked noop build completed
(`ticket006-worker-build/`); sequential supported Node/Rust runtime runs on that
build completed all 15 workload pairs using Chrome for Testing 145. The comparison
at `ticket006-matrix-final-summary.log` checks event fields and values, CSV rows,
finite timing samples and response multiplicities: 12 pairs matched, while V1
and both Q workloads differed in floating-point metadata. Each Rust capture also
observed an additional favicon 404. Each STEP capture had six samples and five
fresh-worker events; C1 STEP had 162524 bytes in every sample.

The float discrepancy was reproduced in a no-browser test: parsing
`-1.7500000000000007` produced `-1.7500000000000009`
(`ticket006-float-red.log`). Enabling serde JSON float round-trip parsing fixed
the test; three Rust capture tests and Clippy passed (`ticket006-float-green.log`,
`ticket006-float-clippy.log`). A second checked noop build at
`80f4b6adecc9fb6b1e4c36842b536ddc89c1f590` passed
(`ticket006-float-build/`). On that build, supported sequential Node/Rust V1 and
both Q captures matched event fields and non-timing CSV values, with all Node
response records present and one additional Rust favicon 404
(`ticket006-float-comparison.log`). The comparison checks that each Q capture's
actual worker generation is consistent internally; independent worker generations
are expected to differ. Each capture's before/after binding matched. Worker
attachment pauses startup; none of these runs establishes timing equivalence,
speed, benchmark admission, or a quiet window.

An earlier C1 STEP pair accidentally selected system Chrome 156: Node exited 0,
but Rust exited 1 with `result.failed=true` after rejecting unrecognized CDP
`Network.requestWillBeSentExtraInfo` messages (`ticket006-worker-step-node/`,
`ticket006-worker-step-rust/`). That failed observation remains failed. The
separate 145 pair used the browser of the earlier C1 receipt and passed the
comparison (`ticket006-worker-step145-node/`, `ticket006-worker-step145-rust/`).

The full matrix has not been repeated on the float-fix source; only the three
affected workloads have same-build comparisons there. Startup cold/warm, U1
freeze/page including uploads, downloads and rendering, controlled cache and
service-worker-origin parity, complete old/new failure controls, full supported
check, hosted CI and independent closure review remain open. No acceptance box
is cleared and no default change is justified.


## 2026-10-26: cold-start response lifecycle diagnosis

The uncommitted cold-start draft has not passed observation. The original direct
Node run completed and recorded the landing core-worker entry, JS and WASM
responses plus the CAD worker/provider responses, while Rust completed the CAD
operation but failed closed on `Session with given id not found`
(`.evidence/ticket006-startup-direct.log`). The diagnostic run did not establish
source/build binding or benchmark admission.

A targeted Chrome for Testing 145 direct run with temporary CDP lifecycle logging
reproduced the same error (`.evidence/ticket006-lifecycle-direct.log`): the page
auto-attached its paused `core-worker/entry.js` dedicated worker, navigation
detached that worker, and the outstanding worker `Network.enable` then failed
with -32001. The core worker is relevant: the Node observation includes its
responses. Treating the detached session as irrelevant, or suppressing the
error, would lose required observations. The temporary logging was removed.

The cold path remains blocked until an event-driven boundary can establish that
the landing worker's required response stream is safely drained before navigation,
including failure and closure controls. Waiting only for attachment or Network
enablement is insufficient; adding a delay or ignoring detached workers is not
proof. No cold fix, source-bound cold comparison, warm or U1 work, full check or
closure review is claimed. The existing draft remains uncommitted and Node is
still the default.

## 2026-10-26: U1 implementation investigation

The Rust launcher still rejects `U1:freeze` before browser launch; a focused
selection probe exited 1 with `Rust capture supports only worker workloads`
(`.evidence/ticket006-u1-investigation/selection-red.log`). The probe stopped at
freeze and did not exercise `U1:page`. No U1 implementation or browser comparison
was completed. The checked noop build validator rejects the existing non-document
draft in this worktree, and the validation-receipt route also rejects non-document
changes beyond its tested source. A separate clean source checkout and its build
binding would need to be established for a fresh comparison without disturbing the
draft. This is a verification constraint, not a cold-start fix or an acceptance
result. Node remains the default.


## 2026-10-26: Rust U1 consumer, unbound diagnostic

Implemented Rust U1 freeze and page routing, using the shared isolated browser
harness, the Node workflow's worker/WebGL instrumentation, file input and
completed-download events. It checks the accepted 2x2 project, unchanged
project after export, accepted-revision preview and print, four print bodies,
STEP reply, rendered frame and page errors. Output remains diagnostic and the
launcher retains its existing source/artifact binding gates. Freeze remains
validation-only; Node remains the default. The pre-existing cold draft is still
uncommitted.

The prior selection RED was `Rust capture supports only worker workloads`.
The Python tooling suite passed 18 tests, Node protocol suite passed five
controlled checks plus STL controls, and the Rust capture binary built and
passed its three existing unit tests. A direct Rust freeze on the existing site
exited 0 and produced its rendered image, accepted project, worker log and
three completed downloads (`.evidence/ticket006-u1-unbound/freeze/`). This
site was not source-bound; this is diagnostic evidence only. Direct Rust page
import exited 1 waiting for the accepted editor and recorded failure and result
events (`.evidence/ticket006-u1-unbound/page/`). A separate Node page run on
the same unbound site also timed out at the editor-ready wait
(`.evidence/ticket006-u1-unbound/node-page/`). Both failure logs show the
committed archive is format 6, while this site requires format 7 and provides
no conversion. The diagnostic freeze archive was not substituted for the
committed fixture. No retry was treated as a pass.

U1 page completion, source-bound old/new comparison, complete failure controls,
cold observer fix, timing, warm/cache parity, full supported check and closure
review remain open. No acceptance box is cleared.

## 2026-10-26: U1 fixture and format-reset investigation

The original archive was introduced by `e98438bf9` as a real Runtime project
download. The U1 runbook and scope pin its format 6, revision 3 and project and
board identities, as well as the 2x2 rigid case and four DCS/MX keycaps. Current
Core requires format 7 and refuses earlier versions without conversion; ADR-0034
calls for rebuilding fixtures, not stamping old documents. Thus the observed
format refusal is consistent with current source, not evidence by itself that the
site is stale.

The prior freeze's actual Runtime download has format 7 and revision 3. Comparing
its project JSON with the original after substituting its generated project,
board and outline IDs with the original IDs and normalizing only the version
gives exact equality; `archive.json` and both asset payloads are byte-identical.
The comparison did not modify either archive. The site digest in the fresh
read-only binding `.evidence/ticket006-u1-fixture-binding/binding-before.log`
matches the successful checked/no-op build at `80f4b6adecc9fb6b1e4c36842b536ddc89c1f590`
(`.evidence/ticket006-float-build/`); that commit is an ancestor of `022c3aaf`
and their only committed difference is this ticket. The CAD package digest and
provider input stamp also match. This binds the presently observed artifacts to
that earlier build, but does not make the prior browser run a fresh launcher
receipt at the dirty current source.

Replacing the fixture with the real new download would change the explicitly
pinned project, board and outline identities. Rewriting those IDs or stamping
the old archive would no longer be the unmodified Runtime download and would
conflict with the reset contract. The frozen-identity versus freshly rebuilt
fixture choice therefore remains open; neither archive nor scope was changed.
No new U1 page run, performance result, or acceptance is claimed.

## 2026-10-26: cold response boundary diagnostic

A fresh direct real-browser RED reproduced the missing-session failure and no
resource receipt (`.evidence/ticket006-cold-boundary-red.log`). The observer now
holds the landing page until the three actual core-worker response events and
outstanding CDP setup replies arrive; a failed required status or early core
worker detach fails the capture. The boundary observes response headers, as the
Node response listener does; it does not claim all response bodies have finished.
The navigation-to-landing metric is recorded before the boundary. Unlike Node's
immediate subsequent navigation, Rust can wait after landing, so the isolated
CAD worker starts later. Timing equivalence and coverage beyond the checked
assets remain open; this diagnostic is not a benchmark admission.

The actual Chrome 145 capture passed with all three core-worker and five CAD
worker/provider response records at status 200 and `serviceWorker=false`.
A controlled missing core WASM response failed closed. The reproducible browser
check is `CHROME_BIN=<chrome> python3 cad/scripts/test-cold-responses.py` after
building `cad-capture`; it uses the existing packaged site and is diagnostic
only (`.evidence/ticket006-cold-closure-browser.log`). Rust capture and probe
unit suites passed 3 and 2 tests respectively
(`.evidence/ticket006-cold-closure-tests.log`). The build receipt is for
`ad18d53a`, while the current HEAD and driver draft differ, and the current
binding gates reject the overlapping dirty draft; no source-bound cold result is
claimed. Dedicated early-detach control, timing equivalence, U1 fixture identity,
warm/cache parity, full checks and ticket closure remain open.

## 2026-10-26: unpaused observation investigation

A temporary unpaused auto-attach variant removed the landing response boundary and
issued `Network.enable` after worker attachment. A direct Node/Rust cold pair used
the same existing site and Chrome for Testing 145, with no new source/build binding
or benchmark admission. Node exited 0 and recorded 26 responses; Rust reached
landing and worker readiness but exited 1 when the landing worker detached before
its `Network.enable` reply (`Session with given id not found`). The failed Rust run
had no resources receipt. Both place the landing metric between navigation start and
usable landing, and the worker metric between worker creation and ready; this pair
cannot prove equivalent timing perturbation. Logs and direct outputs are under
`.evidence/ticket006-unpaused-probe/`.

A separate diagnostic instrumented the unpaused CDP event stream, continued after the
worker setup error solely to inspect later events, and still failed the capture. It
observed no landing core-worker responses and no CAD worker entry response, but did
observe the four subsequent CAD worker/provider responses. It also recorded a server
connection reset as navigation ended. This is not a successful retry or a complete
inventory (`.evidence/ticket006-unpaused-stream/`). Ignoring the detached session
would lose required responses. Playwright's installed Chromium implementation also
uses paused auto-attach; its presence does not prove the draft's additional waiting
has equivalent timing or cache effects.

A separate paused diagnostic cannot supply the measured run's `resources` event:
responses, redirects, failures, cache and service-worker origin are observations of
that individual run. Replacing them with another run's records would change the
capture contract. No nonperturbing single-run mechanism or acceptable split capture
has been established, and no observer fix or metric/fixture change is claimed. The
existing paused boundary draft remains uncommitted and is not benchmark-ready; Node
remains the default. Redirect/target correlation, warm and service-worker controls,
source-bound parity and closure gates remain open.

## 2026-10-27: installed driver semantics inspection

The installed Playwright Chromium driver auto-attaches workers with
`waitForDebuggerOnStart: true`. Its worker handler starts Network session setup,
then sends `Runtime.runIfWaitingForDebugger` without awaiting the Network enable
reply; setup and resume failures are caught. The Node capture listens to context
responses and immediately navigates after the landing-ready event. Thus its
observation is of whatever responses arrive in that run, not a promise to wait for
all three landing-worker assets. The Rust draft instead waits for the Network reply
before resuming and waits for three named landing responses before navigation.
Both use an attachment pause, but their sequencing and failure behavior differ;
matching the existence of a pause alone does not establish timing equivalence.

The installed chromiumoxide 0.9.1 page initialization also enables paused
auto-attachment. Its handler records attached sessions, while the existing CAD
observer uses a separate CDP connection. This inspection and the earlier unpaused
experiment do not establish a complete, unpaused single-run observation mechanism.
Reusing a second run's responses or dropping required records would change the
capture contract. No implementation, new browser run, timing equivalence, or
benchmark admission is claimed from this inspection.

## 2026-10-27: back-to-back worker setup comparison

The installed Playwright worker handler sends `Runtime.enable`, starts Network
session setup (including `Network.enable`), then sends
`Runtime.runIfWaitingForDebugger` without awaiting setup. It catches setup and
resume failures. The Rust draft had awaited the worker Network reply before
sending resume and held landing navigation for three response events. To test
that distinction, a temporary Rust variant sent worker Network enable and resume
back-to-back and removed the landing response wait, while retaining fatal CDP
command errors. Both drivers ran sequentially against the same existing packaged
site, origin, inputs and Chrome for Testing 145.0.7622.0, without source/build
binding or benchmark admission (`.evidence/ticket006-order-comparison-145/`).
Node exited 0, navigated through landing to worker readiness, and recorded all
three core worker and five CAD worker/provider responses (200, not from service
worker). Rust reached landing and worker readiness but exited 1: the landing
worker session was gone when its Network setup replied. It produced no resources
receipt. The experiment does not establish whether swallowing the setup error
would preserve responses; that would weaken failure handling without proof.

An initial pair used Chrome 147: Node recorded all eight assets, while Rust
recorded the five CAD assets and no core worker assets, then failed on browser
handler protocol parsing. That run is not a valid clean comparison
(`.evidence/ticket006-order-comparison/`). The temporary code was removed and
the prior fail-closed boundary draft restored. Back-to-back commands alone did
not establish equivalent observation or navigation behavior; timing parity,
source-bound comparison, and ticket acceptance remain open. Node remains the
default.
The restored draft rebuilt under the shared build lock, and
`CHROME_BIN=/home/chris/.cache/boardstudio-cft/verified-145/chrome-linux64/chrome python3 cad/scripts/test-cold-responses.py`
passed its real-browser capture and missing-core-worker failure control
(`.evidence/ticket006-order-comparison-145/restored-control.log`). This verifies
the restored diagnostic, not timing or full capture parity.

## 2026-10-27: per-session ownership and detach policy diagnostic

A direct diagnostic pair used the same existing packaged site, server and Chrome
145, without source/build binding or benchmark admission. Playwright protocol
logging showed all eight required response events on their dedicated worker
sessions: the core entry, JS and WASM on the landing worker before its detach,
then the CAD entry, JS, WASM and two provider responses on the CAD worker. The
context response receipt contains all eight at 200 and `serviceWorker=false`.
This identifies event ownership in that run, not a guarantee for other runs.

A temporary Rust trace with back-to-back Network enable and resume and no landing
wait showed the landing worker attach after navigation to the performance host
had started, followed by its detach without any of the three landing responses.
It failed at the existing boundary. A separate temporary policy tolerated only
session-not-found replies for an already detached worker and checked the same
run's missing landing responses at page closure. It observed all five CAD responses
on the CAD worker but failed with all three landing responses missing; it did not
produce a successful resources receipt. Both Rust runs also reported a browser
handler WebSocket reset and the local server saw a connection reset. Logs and
per-run events are in `.evidence/ticket006-session-trace/`. The tracing itself
can perturb scheduling, and these are individual observations, not timing parity
or a statistical comparison.

The temporary tracing and policy were removed. Suppressing a detached session's
setup failure cannot recover responses that were never observed, and the parent
session did not supply them in these runs. The existing fail-closed landing wait
remains a diagnostic with different navigation timing from Node; no observer fix,
source-bound comparison, or acceptance is claimed. Node remains the default.
Timing equivalence, per-run complete response and redirect/failure correlation,
warm and service-worker controls, U1 fixture identity, full supported checks and
closure review remain open.

## 2026-10-27: early target discovery experiments

The existing Rust capture creates an isolated `about:blank` page first, configures
service-worker registration and cache, then attaches the separate response CDP
session, enables page Network and acknowledges page `Target.setAutoAttach` before
starting landing navigation. Thus the observer registration is complete before
navigation; the delayed observation is worker attachment, not a late call to
`start`. Chromiumoxide has its own page session and auto-attachment.

On the same existing, unbound packaged site and Chrome for Testing 145, a temporary
variant enabled browser-level `Target.setDiscoverTargets` before page attachment
and removed the landing response wait. Discovery reported the core worker target
with an empty URL before the page-session `Target.attachedToTarget`. The latter
identified the core entry and was paused, but arrived as navigation to the
performance host began; no core worker response appeared before detach. The run
failed closed (`.evidence/ticket006-discovery-control.log`). Discovery was enabled
after page creation but before either navigation; this does not test browser-level
auto-attach before page creation.

A second temporary variant attached directly to every discovered worker target
and enabled Network on that session. Its trace showed an early direct attachment
with empty URL and `waitingForDebugger=false`, followed by the page-session paused
attachment with the core entry URL; no core response was observed before detach,
and the run failed closed (`.evidence/ticket006-discovery-attach-control.log`).
Both failures also recorded a browser handler WebSocket reset. These exploratory
variants include CDP tracing and are not timing or benchmark evidence. The earlier
Node trace in `ticket006-session-trace/` observed all three core responses before
navigation and all five CAD responses in that same Node run; these separate runs
are diagnostic comparisons, not a parity receipt.

The temporary variants were removed. The restored pre-existing boundary draft was
rebuilt under `~/.cache/tmp/boardstudio-build.lock` and its real Chrome cold capture
and missing-core-worker failure control passed (`ticket006-discovery-restored-build.log`
and `ticket006-discovery-restored-control.log`). The draft still delays navigation
and is not evidence of measurement parity. A concrete next experiment is to expose
worker session events and Network setup through chromiumoxide's own page session,
registering observation before its auto-attach resume, then compare same-run CDP
traces and navigation ordering against Node. This may require a narrowly scoped
upstream/fork integration; feasibility and timing equivalence are unproven. Browser-
level auto-attach before page creation is another untested candidate. No observer
fix, source-bound parity, or ticket acceptance is claimed; Node remains default.

## 2026-10-27: handler ordering inspection (no new browser run)

Inspection of the vendored chromiumoxide handler shows that its page session
requests flattened auto-attachment with `waitForDebuggerOnStart=true`, then queues
`Runtime.runIfWaitingForDebugger` for a paused child immediately on the page's
`Target.attachedToTarget` event. The handler routes session events through its
session-to-target map; child attachment events received on the page session are
handled by the page target and do not themselves populate that map. Its existing
internal-command response path does not report worker setup errors or timeouts to
the CAD capture. Consequently, adding `Network.enable` ahead of resume alone is
not sufficient evidence of a complete, fail-closed observer.

The retained Playwright protocol trace in `ticket006-session-trace/node.stderr`
shows both `Network.enable` and `Runtime.runIfWaitingForDebugger` sent on each
worker session in that order, before either command reply; both replies succeeded.
In that run the three core-worker responses were received on the worker session
before the performance-host navigation. This is a single traced run, not a claim
that waiting for the Network reply before resume has equivalent timing.

A candidate integration must register capture before navigation, route each child
session's response and redirect events to the owning page capture, account for
setup errors, timeouts and detach, and preserve the old navigation sequence. It
still needs a focused real-browser comparison against Node, including the missing
response control and ordering trace. No handler change or new browser comparison
was made from this inspection; feasibility and the existing gates remain open.


## 2026-10-27: partial handler routing implementation

The vendored handler now associates dedicated worker sessions with their owning
page and routes worker `Network.responseReceived` and `Network.requestWillBeSent`
events to that page's listeners. The association is removed on detach or page
destruction. A synthetic child-session routing test first failed with no event
and then passed; the final test uses a real CDP Network response payload. The
focused handler lifecycle suite passed 4 tests under the shared build lock
(`.evidence/ticket006-handler-lifecycle.log`).

This is an incomplete integration, not a capture fix: the CAD observer still
uses its separate connection and waits at the landing boundary. The handler does
not yet enable Network before worker resume or surface setup failures/timeouts
to the observer. The routing change has not been exercised in a real browser,
and no new Node comparison or missing-resource control was run. Preserve the
existing Node default and do not interpret this slice as timing parity or ticket
acceptance.

## 2026-10-27: direct handler integration attempt

A temporary implementation sent `Network.enable` on the handler's attached
worker session and resumed only after its successful reply, surfaced setup errors
and timeouts through the handler, and collected worker responses through page
listeners without the landing boundary wait. It built under the shared lock.
The first Chrome for Testing 145 cold fixture failed: `Network.enable` returned
`Session with given id not found`, the required core entry response was absent,
and the handler also reported a WebSocket reset
(`.evidence/ticket006-integrated-cold.log`). This is a failed run, not proof
that every handler integration is impossible. The negative fixture and Node
comparison did not run after this failure. The temporary integration was removed
to avoid changing the shared handler's behavior; the prior partial routing and
boundary draft remain. The failed experiment does not establish timing parity or
close ticket 006. A capture without the navigation wait still needs an earlier
attachment mechanism and same-run response and failure evidence.

## 2026-10-27: approved U1 format-7 fixture replacement

The user approved replacing the obsolete format-6 fixture and updating its generated
identities. `cad/test/fixtures/performance/U1.boardstudio` is now the exact,
unmodified Runtime download from `.evidence/ticket006-u1-unbound/freeze/` (SHA-256
`87e2fe1f1424b26dd5def247a9b7ec1718a1f839ea9746b031e8323bce42b5a0`). The
capture itself was unbound; the earlier recorded artifact comparison ties its site
bytes to the checked build at `80f4b6adecc9fb6b1e4c36842b536ddc89c1f590`, not
to a fresh source-bound browser receipt. The scope and runbook pin format 7,
revision 3 and its generated project, board and outline IDs. A fresh comparison
normalized only those three IDs (including board-derived closure IDs) and format:
project JSON was exactly equal to the prior fixture, archive metadata and both
asset payloads were byte-identical, and each payload matched its SHA-256 name.
No archive contents were rewritten.

The checked web build completed (`command.log` exit 0, 98.25 seconds), but its
clean/no-op receipt was rejected because the checkout is dirty
(`.evidence/ticket006-u1-format7-build-checked/binding-rejection.log`). The first
launch used an abbreviated source SHA and stopped before building; the second
lacked `NODE_PATH` and stopped before building. The corrected build and browser
runs used the shared build lock and `boardstudio-tests.slice`.

Node U1:page imported the replacement, rendered and exported both downloads,
and its browser process exited 0 with a successful result. The launcher invalidated
it because unrelated source changed during the run; before/after bindings and
all outputs are in `.evidence/ticket006-u1-format7-node/`. The full Python download
checker correctly refused the changed binding. Separately, ZIP path/uniqueness/CRC
checks and `node cad/scripts/check-cad-page-downloads.cjs` passed on those Node
outputs, including all four STL topology/material checks. This is diagnostic
output inspection, not an admitted capture or full download-check pass.

A direct Rust U1:page diagnostic against the same packaged site and Chrome 145
exited 1 waiting for the accepted editor; the browser handler also reported a
WebSocket reset. Its Library body shows no format-refusal message, but the cause
of the failed import is not established (`.evidence/ticket006-u1-format7-rust-direct/`).
The direct run has no source binding. Rust U1 completion, a stable same-source
Node/Rust comparison and the full independent download checker remain open;
no performance or ticket acceptance is claimed.

## 2026-10-27: U1 Rust upload reproduction

A second direct Rust U1:page run against the retained packaged site and Chrome 145
failed waiting for the accepted editor and again reported `WebSocket protocol reset
without closing handshake` (`.evidence/ticket006-u1-rust-repro-1027/`). The run
held the shared build lock and its recorded before/after source and artifact bindings
matched. It used the existing harness binary and is diagnostic: its build-to-source
binding is not established, and it is not an admitted capture.

The failure page reports `Storage is still initializing; try again shortly.` Runtime
emits that error from `begin_open` when a real file import arrives before storage
startup finishes. This supports an early upload as the immediate import failure; it
does not explain the browser handler reset. The Library saved-project-list loading
state is independent of Runtime storage initialization and cannot prove import
readiness. No readiness workaround or retry was admitted on this evidence. A fresh
source-bound harness/site comparison, Rust completion and independent checks for
both drivers remain open.

## 2026-10-27: storage readiness and U1 diagnostic

Runtime exposes storage initialization readiness and retains initialization errors; both
Library import inputs stay disabled until startup succeeds and show the retained failure.
The Rust U1 consumer waits for the enabled file input before assigning the archive.
The mounted Chrome regression first failed because the input was enabled during
initialization (`.evidence/ticket006-readiness-red-render.log`); after the change it
passed 1/1, covering the landing and menu controls, a notification-driven transition,
and a failure that remains visible after another status (`.evidence/ticket006-readiness-green-both.log`).

The checked site build completed, but the measurement tool rejected its binding because
source changed during the build (`.evidence/ticket006-readiness-source-build/`). The
Rust harness build succeeded under the shared lock. One Rust and one Node U1:page
validation ran under that lock against the resulting site, with matching individual
before/after bindings, but the site build drift makes both diagnostic and unadmitted.
Node exited 0 and `check-cad-page-downloads.py` passed on its output
(`.evidence/ticket006-readiness-node-diagnostic/`, `.evidence/ticket006-readiness-download-check.log`).
Rust reached the accepted-editor wait and exited 1 with 20 Chromium 156 protocol
message decoding errors and a WebSocket reset (`.evidence/ticket006-readiness-rust-diagnostic/`).
The earlier Chrome 145 reset is not resolved by this result. A stable source-bound
build, Rust completion, and admitted Node/Rust parity remain open.

## 2026-10-27: canonical U1 upload path

The Rust consumer now canonicalizes the fixture path before passing it to CDP,
rejecting canonicalization, non-file and non-UTF-8 errors explicitly. The focused
regression test failed on the original `tools/browser-harness/../../cad/...` path
and passed on the canonical path; the capture binary's four unit tests passed
(`.evidence/ticket006-upload-red.log`, `.evidence/ticket006-upload-green.log`).
The earlier Chrome 145 CDP control established that the original path produces
`NotReadableError` while the canonical path to the same file reads successfully.

The freshly rebuilt Rust binary ran U1:page with Chrome 145 under the shared build
lock, imported the fixture, rendered and exported both downloads, and exited 0
(`.evidence/ticket006-u1-canonical-1027/`). Before/after source and artifact
bindings matched. The independent Python download checker passed on that output
(`.evidence/ticket006-u1-canonical-check.log`); the earlier Node Chrome 145 run
and its checker also passed (`.evidence/u1-cft145-test-comparison-1027/`). Both
are diagnostic: the retained site build's source binding was previously rejected
because source changed during the build, so this is not an admitted source-bound
comparison. The Rust log reports `WebSocket protocol reset without closing
handshake` and classifies it as an expected post-boundary reset after clean child
exit; it was not suppressed. A stable source-bound site build and comparison,
remaining ticket controls and closure review remain open.
## 2026-10-27: U1 review corrections

The download checker now requires a successful source-bound build receipt and
checks its existing binding contract against the capture. A successful build
subprocess alone is insufficient: the retained format-7 build receipt is rejected
because its binding was rejected. The Node importer waits for an enabled input.
U1:page validates the approved project, board, outline, format and revision pins
both before launch and on the accepted project; U1:freeze retains generated
identities. These checks do not turn prior diagnostic captures into admitted proof.
Fresh source-bound U1 browser comparison and the remaining ticket gates are open.

## 2026-10-27: Rust runtime admission guard

The launcher rejects Rust `runtime` for `startup:cold` because its timing is
unqualified, and for `U1:page` because it produces validation-only output without
runtime samples. Both errors direct users to Node runtime or validation diagnostics.
Rust validation and Node runtime remain selectable. The focused launcher regression
failed on both previously admitted Rust modes before the guard; afterward
`python3 cad/scripts/test-measure-cad.py` passed all 22 tests. This does not establish
timing parity, runtime samples, or ticket acceptance; the existing gates remain open.

## 2026-10-27: U1 desktop layout differential

Direct Chrome for Testing 145 U1:page captures against the same retained
`web/target/site` reproduced the viewer height difference. Both had a 927 px
Case panel; Node's collision section and footer were 112 and 45 px, while Rust's
were 132 and 57 px. Those 20 and 12 px differences consume the 32 px difference
in the flexed viewer. The initial snapshots also showed Node light and Rust dark.
Changing only Rust's color preference to light did not change its dimensions.
The vendored chromiumoxide viewport initialization enables touch unconditionally;
Case controls have larger minimum sizes under `pointer: coarse`. The U1 Rust
runner now sets light preference and disables touch before navigation, matching
Node's light, non-touch layout semantics. Both runners wait for loaded fonts,
non-coarse pointer, light preference, exact geometry and saved state, and write
the same DOM layout/readiness snapshot. Existing Fit draw and WebGL finish waits
remain in place.

The focused direct comparison (`.evidence/ticket006-u1-touch/`) exited 0 for
both drivers. Both observed light theme, loaded fonts, zero touch points, the
same accepted project and Case child heights, and viewer rectangle
`(272.328125, 144, 938.078125, 485.8125)` at 1513 x 1043. Node reported a fine
pointer; Rust reported neither fine nor coarse, so full input-media parity is not
claimed. The baseline and light-only captures are retained in
`.evidence/ticket006-u1-layout-red/` and `.evidence/ticket006-u1-layout-green/`.
The final per-run process inventories and cleanup checks are in each `cleanup.json`:
all tracked live descendants had exited, and the owned temporary profiles were
removed. Earlier diagnostic runs were not inventoried as completely; a subsequent
process check found no Chrome 145 processes from those runs.

These are direct validation diagnostics against the same retained site, not fresh
source-bound launcher captures: the site was built at `bf8ab3ae3ed3`, while the
workspace HEAD had advanced to `4c1d4640e284a11f03dfbfa123dcb8923fd1d738` by
comparison time. The working tree also contained concurrent unrelated changes.
No cumulative draw-count or timing parity, independent download admission, full
capture parity, or ticket closure is claimed.

## 2026-10-27: U1 pointer-media launch parity

**Superseded:** Omitting the Playwright default changed the historical Node reference.
The default has been restored; the comparison below does not establish parity with it.
A U1-scoped Rust launch argument and a fresh comparison remain pending.

Playwright's installed headless launcher supplies a `--blink-settings` default
forcing fine pointer and hover; chromiumoxide does not. A Chrome 145 probe with
otherwise identical Playwright settings observed fine pointer and hover with the
default, and `pointer: none` and no hover with that one default omitted. The U1
Node capture now uses Playwright's `ignoreDefaultArgs` to omit it, and records the
omission in the browser event. Other workloads retain their existing defaults.
No `matchMedia` override or pointer-capability emulation is used.

Sequential direct U1:page validation captures under the shared build lock used the
same retained site and Chrome 145. Both completed successfully and their full
`page-layout.json` snapshots matched, including neither fine nor coarse pointer,
zero touch points and the viewer rectangle. The probe and Node and Rust captures
tracked descendants by PID and process start time in a subreaper; respectively 16,
11 and 17 tracked processes were reaped with none remaining. Evidence is in
`.evidence/ticket006-u1-pointer/`. The Rust capture logged its previously known
post-boundary WebSocket reset after clean child exit.

These direct captures use the retained site built at `bf8ab3ae3ed3`, not a fresh
source-bound launcher build; they do not establish timing, download admission, or
complete ticket acceptance. Chrome's reported absence of a pointer in this
headless environment remains a limitation of the comparison.


## 2026-10-27: U1 Playwright-default pointer comparison

The shared launcher now accepts additional per-run Chrome arguments; its original
entry point delegates with no additional arguments. Rust U1 passes the installed
Playwright headless default `--blink-settings=primaryHoverType=2,availableHoverTypes=2,primaryPointerType=4,availablePointerTypes=4`; other Rust workloads do not.
The Node reference retains its defaults. U1 setup remains inside the failure
receipt boundary.

The first comparison failed: chromiumoxide prefixed the supplied switch a second
time, producing `----blink-settings` and Rust reported no pointer. After stripping
the leading dashes for the builder, sequential Chrome for Testing 145.0.7622.0
U1:page validation runs under the shared build lock both exited 0. Both reported
fine pointer, hover, fine any-pointer, any-hover, zero touch points, loaded fonts,
light theme and a 1513 x 1043 viewport. Their viewer rectangle was
(272.328125, 144, 938.078125, 485.8125); both screenshots were 1513 x 1043.
Screenshots were not pixel-identical. Layout snapshots differed in one status
element rectangle (Node zero-sized; Rust 270 x 60.78125), so full layout parity
is not claimed. Evidence including actual process command lines and cleanup is
in `.evidence/ticket006-u1-playwright-fixed/`; Node and Rust tracked 12 and 16
processes respectively, with no tracked processes or profile paths remaining.
The failed first comparison is in `.evidence/ticket006-u1-playwright-default/`.

The retained site was built at `bf8ab3ae3ed3`, not the current source; these
direct captures do not establish source-bound acceptance, timing parity, download
admission, or ticket closure. Setup-failure receipt behavior was not independently
exercised in this comparison. The previous pointer comparison's claimed
`cleanup.json` files are not present in that evidence directory.

## 2026-10-27: retained U1 visual and failure inspection

The retained Chrome 145 screenshots in `ticket006-u1-playwright-fixed` visibly
show the same assembled case, controls, Saved indicator, geometry-ready status,
and four keycap findings. A pixel comparison found 60,844 differing pixels out
of 1,578,059, mean absolute channel difference 0.075, and maximum channel
difference 98. Within the 400 x 330 model region, 202 pixels differed. The
screenshots and DOM snapshots are separate observations, and their data does
not establish whether the differences arise from capture timing, rendering, or
another cause. Pixel identity is not established or required by this inspection.

The differing status rectangle belongs to the `div[role=status]` inside the
save-state `details`; both snapshots contain the same saved text, while the
geometry-ready and case-body status rectangles match. The save popover is not
visible in either screenshot. The snapshots do not record the disclosure's
open state or timing, so the cause of its different rectangle remains open.
No capture behavior was changed on that evidence.

A direct U1:page invocation of the existing Rust binary with `CHROME_BIN` unset
exited 1 and emitted `failure` followed by `result` with `failed: true`, with
no U1 success event (`.evidence/ticket006-u1-setup-failure/`). This fails before
Chrome launch and before `cad_u1::run`; it does not exercise the inner U1 setup
failure or `page-failure.log`. No browser was started for this control. The
retained comparison remains unbound to current source; source-bound parity,
inner setup-failure receipt, and the other open acceptance gates remain open.


## 2026-10-27: launch and Node diagnostic review corrections

The shared additional-argument entry point now admits only a syntactically bounded
`--blink-settings=...` switch and rejects other switches before launching Chrome.
The focused regression first failed because a `--user-data-dir` override reached
the launcher; it then passed, including harness-owned switches and an ordinary
Blink setting (`.evidence/ticket006-launch-red.log` and `ticket006-launch-green.log`).
Node's extra layout readiness wait and snapshot now run only for validation; the
existing runtime workflow retains its prior path. The launcher Python suite passed
23 tests, and the Rust capture binary built under the shared lock.

Sequential direct Node and Rust U1:page validation with Chrome 145 against the
retained site both exited 0, emitted successful results, and produced equal layout
snapshots (`.evidence/ticket006-review-real/`). The subreaper recorded 29 process
identities and no remaining tracked processes; a subsequent process inventory
showed no Chrome processes. These are diagnostics on the retained site, not
source-bound acceptance or benchmark evidence. The requested controlled failure
inside U1 CDP setup was not exercised; its page-failure receipt, failed result,
absence of U1 success, and cleanup remain an open gate. Source-bound parity,
benchmark and other ticket acceptance gates also remain open.

## 2026-10-27: user decision on response parity

The Rust runtime must preserve Node's existing navigation sequence and record the
responses actually observed in that same run. Observation setup, protocol or
premature detach failures must fail the capture; a fixed landing-asset inventory
and a wait that changes the workload are not requirements. The current landing
boundary and runtime admission guard remain in place until a replacement is proven.
The retained handler and separate-session experiments above do not establish that
replacement: the direct handler attempt awaited the worker Network reply before
resume and failed with a missing session. The candidate matching the observed
Playwright ordering sends worker Network enable and resume in that order without
awaiting either reply, while tracking both outcomes and routing responses and
redirects from the same run. Same-site Chrome 145 comparison and failure controls
remain required; individual timing samples do not establish timing parity.

## 2026-10-27: queued worker setup detach accounting

A controlled handler sequence attached a paused worker and detached it before its
queued Network enable and resume were submitted. The new regression failed for
missing detach error (`.evidence/ticket006-queued-detach-red.log`). The handler now
reports and removes queued setup for a detached worker, reports worker submission
errors, and returns to target polling after a paused worker attachment so its
ordered setup can be submitted before consuming further incoming messages. The
vendored library suite passed 10/10, including the regression and Network-before-
resume check (`.evidence/ticket006-queued-detach-green.log`).

One sequential Node/Rust `startup:cold` validation comparison used Chrome for
Testing 145 and the same retained, unbound packaged site under the shared build
lock (`.evidence/ticket006-queued-detach-real/`). Node exited 0 and observed all
three core-worker and five CAD worker/provider assets. Rust exited 1, reporting
worker detach during setup, missing-session errors for Network enable and resume,
and a WebSocket reset; its 24-response receipt lacked all three core-worker
assets, though it observed the five CAD assets. The owned-process subreaper
tracked 30 identities, killed remaining owned processes and recorded no survivors
or temporary profile directory. This is a failed Rust comparison; earlier worker
attachment remains necessary to establish startup response capture, and this
result does not close ticket 006 or establish source-bound acceptance.

## 2026-10-27: lifecycle review remains open

The existing uncommitted handler candidate is not ready for acceptance. Worker
instrumentation and its fatal errors currently apply to every handler consumer,
while the observer only checks whether any worker attached. An attachment that
is not waiting for the debugger is not established as observed, and submitted
setup can remain unresolved when its owner is destroyed or transport closes.
Errors queued during target polling or timeout eviction can also be stranded
when the handler returns Pending without another wake. The prior failed real
comparison remains the current evidence; no new runtime or test result is
claimed by this review.

A minimal implementation direction is an explicit CAD-only opt-in on the handler
before its drain starts, with private per-session setup state accounting for both
queued and submitted commands. Fail the observation on an unpaused attachment,
setup failure, timeout, owner destruction or transport closure before readiness;
return queued errors before Pending. Keep the Node navigation sequence and avoid
an asset inventory. Exercise the lifecycle controls and shared consumers, then
run a real same-site comparison with owned Chrome process cleanup before claiming
response parity. This direction is not implemented or verified here.

## 2026-10-27: CAD-only handler opt-in, partial lifecycle repair

The candidate now enables worker Network observation explicitly on the handler
before the CAD capture and diagnostic probe start their handler drains. The
handler defaults to disabled for other consumers. It rejects a known dedicated
worker attached without a debugger pause, reports queued or submitted setup when
its owner is destroyed, reports transport closure during setup, and drains
queued setup errors before returning Pending. Existing setup command and detach
accounting remains in place.

Under the shared build lock, `cargo check --locked --manifest-path
 tools/browser-harness/Cargo.toml --bins` passed and the vendored handler tests
passed 6/6, including default-off ordering, unpaused attachment, queued detach,
owner destruction, response routing, and existing close controls. These added
assertions were not separately run red before the change. Submitted-command and
transport-close failure controls, prompt wake controls, shared-consumer runtime
checks, and a clean Chrome 145 same-site comparison remain open. No browser was
launched in this iteration. The runtime admission guard remains in place and
response parity is not established.

## 2026-10-27: handler-integrated CDP ordering trace

A targeted sequential Node/Rust cold comparison on the same retained, unbound
site and Chrome 145 traced the actual browser connection (`.evidence/ticket006-handler-trace/`).
Node exited 0: its page session A89F52DBBF5E7B5C17D4CC91EDF283D9 received
paused core worker 59344598C95F60AA86A13B6402130B8C, sent Network enable and
resume, received both successful replies and all three core responses, then sent
the performance-host navigation. Rust exited 1: page session
0C178A801B3075464A4ABE69B268D0B1 sent that navigation before receiving paused
core worker 6D973AB33C36E204783279CDB69840ED. It sent Network enable and resume
after attachment, but navigation detached the worker and both commands failed
with session not found. The CAD worker setup succeeded and its responses arrived.
The handler correctly failed the incomplete observation. The temporary CDP
logging was removed; tracing may affect scheduling. Cleanup tracked 30 process
identities and reported no survivors or temporary profile.

The Rust landing predicate uses direct JavaScript evaluation while Node uses
Playwright's role locator and its visibility wait. The trace establishes the
race, not which difference causes it or a general timing guarantee. An actionable
next experiment is to match Playwright's role/visibility readiness and scheduling
without adding an asset wait, trace both sessions again, and retain fail-closed
setup accounting; alternatively test browser-level auto-attachment before page
creation with ownership and setup failures preserved. Neither alternative is yet
proved, and the source-bound comparison and ticket acceptance remain open.

## 2026-10-27: role and visibility landing candidate

The cold Rust capture now resolves the exact button role and accessible name using
`driver::find_role`, requires a single match, and checks its visibility with
`driver::is_visible` before continuing. It retains the existing timeout and does
not wait for worker assets. This is a candidate alignment with Playwright's
`getByRole(...).waitFor()`; the CDP accessibility lookup and retry scheduling are
not proven identical to Playwright. The prior traced run above is the failing
real-path baseline, not a separately constructed regression test.

One sequential, instrumented Node/Rust Chrome for Testing 145 `startup:cold`
validation ran under the shared build lock against the same retained, unbound
site, origin `http://127.0.0.1:37143` (`.evidence/ticket006-role-trace/`). Both
exited 0 and emitted successful result events. Node page session
E1AF1A09FD378C50C2D2252C52EC4A85 attached paused core worker
A1F76FAA8FA995D6E9DB8726E95040F6; Rust page session
84E307AEF17E4D5E80BEA10086386414 attached paused core worker
0AA2E7A737BA0F81325BB8A4658E349A. Both sent Network enable then resume,
received successful replies and the core entry, JS and WASM responses (200,
not from a service worker), then sent performance-host navigation. Node CAD
worker 7A22AFD72E68C6173F8C3532CF39A1B4 and Rust CAD worker
91E4CB221FE3C5C6EEE08EA7CB9AD8E1 likewise received successful setup replies
and all five CAD entry/provider responses (200, not from a service worker).
Neither trace recorded a redirect. There were no CDP setup errors; Rust logged
an expected post-boundary WebSocket reset after clean child exit, and the server
logged a connection reset. The full response receipts differ: Rust observed an
additional favicon 404. This single traced run does not establish response parity,
a causal timing guarantee, or source-bound acceptance. Tracing may affect timing.

Cleanup tracked 29 process identities, killed remaining owned processes, reaped
children, and reported no survivors or temporary profile; a subsequent process
inventory found no test Chrome, driver or crashpad. Temporary CDP trace source
was removed. The handler's existing fail-closed setup accounting is retained;
submitted-command, transport-close and prompt-wake failure controls and the other
acceptance gates recorded above remain open.

## 2026-10-27: owner detach and reset review fixes

A controlled handler regression independently failed on page-session detach (no
fatal setup error) and abrupt transport reset (only `Broken pipe`), recorded in
`.evidence/ticket006-lifecycle-review-red.log` and
`.evidence/ticket006-reset-review-red.log`. The handler now tracks each worker's
parent session, removes its queued setup and ownership on parent detach, and
reports unresolved queued or submitted setup. A transport error also queues a
distinct unresolved-setup error before the handler terminates. The focused
control exercises queued and submitted setup separately for both sequences.
The vendored library suite passed 11/11 under the shared build lock and
`boardstudio-tests.slice` (`.evidence/ticket006-lifecycle-review-green.log`).
No Chrome was launched for these handler-only controls.

The installed Playwright `playwright-core/lib/coreBundle.js` marks a request as
a favicon when its URL ends exactly in `/favicon.ico` or its redirected-from
request was marked, and suppresses its public context response event. The trace
observed that 404 in both browser connections; the differing public receipts are
therefore expected from that reference behavior. The Rust collector does not yet
implement that request/redirect identity rule. Its separate request and response
streams do not by themselves establish cross-stream ordering, so a suffix-only
filter would not establish precise parity. This remains open along with the
source-bound and other acceptance gates; no new browser comparison is claimed.

## 2026-10-27: worker-entry redirect provenance remains blocked

A focused Chrome for Testing 145 fixture redirects the worker entry to a script
that performs a separate redirect and a missing fetch; the page also redirects
its favicon to the same destination as the unrelated redirect. The clean probe
fails closed with `response URL does not match request provenance`
(`.evidence/ticket006-redirect-clean-red.log`). The regression remains red.

The ordered diagnostic CDP capture (`.evidence/ticket006-redirect-full-trace.log`)
showed page session `7244A9C2001FCC24972A9BF12B2FD8BF` requesting the entry
with ID `3E96E55705FAB3DD6035D2B2C811AF3D`, followed by attachment of worker
session `57023F936118FF5D5A9893A0D83D7A2E` with that target ID, and a 200
response on the worker session for `/worker-final.js` with the same request ID.
No entry redirect event or 302 appeared in the observed stream. The separate
worker redirect did have a request event with `redirectResponse`, as did the
favicon redirect. Diagnostic instrumentation was removed after capture.

Installed Playwright on the same fixture and Chrome reported entry URL status
200, the unrelated redirect 302 and destination 201, and the missing fetch 404;
it did not report the entry 302 or final URL
(`.evidence/ticket006-redirect-playwright-core.log`). Its response URL comes from
the request object. This does not prove the missing redirect provenance; no
collector exception for mismatched URLs was added. A sound way to recover the
entry redirect chain remains open.

## 2026-10-27: worker-entry handoff response

For an initial worker response with no request in the worker session, the collector
accepts the parent request only when its request ID equals the attached worker's
target ID. It records that request in the worker session and emits its original
URL with the observed response status and service-worker field. Subsequent
responses and all other session/request combinations retain strict URL and
provenance checks. This matches the installed Playwright observation; it does not
infer or emit the entry redirect or a 302 that Chrome did not expose.

The previously recorded Chrome 145 fixture failed with the provenance mismatch
above. After this change, the real fixture passed against Playwright on the same
fixture server: original worker entry 200, unrelated redirect 302 and destination
201, missing fetch 404, and favicon redirect excluded without excluding the shared
destination. Page exception and missing worker controls failed closed. A focused
Rust unit check passed for differing response URLs outside the handoff and missing
response URLs. These results do not close the remaining ticket gates.

## 2026-10-27: retained-site cold comparison

A sequential Chrome for Testing 145 validation on the same retained site produced
26 responses for each of Node and Rust, with matching URL, status and
service-worker response multisets (`.evidence/ticket006-handler-smoke-cold/`).
Both capture runs exited 0; the Rust drag smoke passed 15/15. The comparison used
a dirty candidate and an unbound retained site, so it is not source-bound
acceptance. Cleanup tracked 68 owned process identities and recorded no survivors
or remaining temporary profile. The runtime admission guard remains in place.
Full source-bound, cache, service-worker and timing parity remain open.

## 2026-10-27: warm startup validation candidate (incomplete)

A local uncommitted candidate adds the Rust warm selection, same-page prime and
measured landing navigations, Node-shaped service-worker state event, warm cache
and registration settings, and a validation-only runtime guard. The response
collector rejects an empty response observation. The launcher regression first
failed because warm selection was unsupported and the runtime guard was absent;
after the candidate, the two focused launcher checks passed. The existing Rust
capture unit checks passed 5/5 under the shared build lock and
`boardstudio-tests.slice` (`.evidence/ticket006-warm-rust-final.log`).

These checks do not establish browser behavior or service-worker origin parity.
No source-bound Chrome comparison, HTTP-cache control, service-worker-origin
true/blocked control, or full benchmark qualification has been performed for this
candidate. The retained site receipt predates this candidate, and the official
build-binding contract rejects a dirty noop build. Node remains the runtime
reference; this candidate must not be treated as ticket closure.

## 2026-10-27: warm service-worker diagnosis (still blocked)

On the retained, unbound site, a focused Chrome 145 Rust rerun reproduced the
approximately 60-second evaluation timeout. Before awaiting `ready`, the page
URL was the expected landing URL and `getRegistrations()` returned one scoped
registration with no active, installing, or waiting worker and no controller.
Temporary CDP instrumentation observed a service-worker target attach paused,
followed by submission of `Runtime.runIfWaitingForDebugger` and
`Target.detachFromTarget`. Detach returned success, but no response was observed
for that service-worker resume; a later dedicated-worker resume returned success.
The trace is consistent with a resume/detach race but does not prove the cause.
See `.evidence/ticket006-warm-debug3/`; cleanup recorded no adopted children.
Instrumentation was removed. No fix, post-fix comparison, error control, or
source-bound acceptance is claimed.

## 2026-10-27: warm service-worker resume sequencing (diagnostic)

The CAD-enabled handler now waits for a successful paused service-worker resume
response before requesting detach, and reports resume, detach, timeout and
premature session/owner closure failures. Other handler consumers retain the
existing policy. The ordering regression failed before the change because detach
was already queued; the final vendored handler library suite passed 15/15,
including resume-error and premature-detach controls.

Sequential Node and Rust warm validation captures with Chrome 145 against the
same retained, unbound site both exited 0 and observed respectively 20 and 22
responses marked service-worker-origin. Total response counts were 186 and 45,
so full response parity is not established. The capture preceded the final
additional owner-destruction and transport-closure guards; the final handler
suite passed after those guards. The subreaper recorded 10 reaped descendants and
no remaining children. See `.evidence/ticket006-warm-sequenced/` and
`.evidence/ticket006-service-final-tests.log`. Source-bound acceptance, full
response parity, failure controls in a real browser and timing qualification
remain open.

## 2026-10-27: retained service-worker network session (partial)

For CAD capture, paused service workers now queue Network enable before resume,
retain their flattened session until teardown, and use the existing worker session
ownership, ordered event routing, and setup failure accounting. Other consumers
retain the original service-worker detach behavior. The focused ordering regression
failed before this change (resume only), then passed; the vendored handler library
suite passed 13/13, and the focused default-off control passed.

A sequential Chrome for Testing 145 warm validation against the same retained,
unbound site completed for both drivers. Node recorded 186 responses (20 marked
service-worker-origin); Rust recorded 188 (22 marked service-worker-origin). Their
URL/status/origin multisets agree except for two additional Rust 200 responses,
for the core worker JS and WASM, both marked service-worker-origin. The source of
this difference remains unresolved; no responses were filtered to force parity.
The subreaper recorded 10 reaped descendants and no remaining children. Evidence:
`.evidence/ticket006-service-network-warm/`, `.evidence/ticket006-service-network-red.log`,
`.evidence/ticket006-service-network-green.log`, and
`.evidence/ticket006-service-network-default-green.log`. This is not source-bound
acceptance or timing qualification; exact warm response parity remains open.

## 2026-10-27: warm response timing trace (unresolved)

A fresh sequential same-site Chrome 145 run reproduced Node 186 and Rust 188
responses, with 20 and 22 service-worker-origin responses respectively. The
multisets differed only by Rust's additional 200 service-worker-origin core-worker
JS and WASM responses. Both runs exited 0; the retained site and dirty candidate
are not source-bound. Evidence is `.evidence/ticket006-trace-investigation/`.

Node's actual Playwright protocol trace shows the measured landing's second core
worker attaching just before Playwright sends navigation to the performance host.
Chrome emits the worker JS request, then detaches that worker after navigation,
without a response for that request or a WASM request in that worker session.
The installed Playwright network manager ignores responses without an associated
request and has specific handling for service-worker responses, but this trace
does not show it suppressing either of these two responses. It does not establish
the Rust event ordering or prove the cause of the differing observations. No
filter was added: discarding valid responses would conceal a timing difference.
The subreaper recorded ten reaped descendants and no remaining children. Exact
warm parity, controlled browser failure/absence checks for this investigation,
source binding, cache and timing qualification remain open.

## 2026-10-27: measured landing worker ordering (diagnostic)

One sequential Chrome for Testing 145 warm comparison on the same retained site
completed for Node and Rust (both exit 0), under the shared build lock. Node
recorded 186 responses, 20 service-worker-origin; Rust recorded 188, 22. The
only multiset differences were Rust's additional 200 service-worker-origin
core-worker JS and WASM responses. Evidence is
`.evidence/ticket006-narrow-trace-20261027/`; the cleanup receipt records ten
reaped adopted children and no remaining children.

Node's measured landing worker attached at 14:19:13.498 UTC, performance-host
navigation was sent in the same millisecond, and the worker detached at
14:19:13.501 without the two responses in that session. Rust's measured landing
worker attached at 14:19:16.643, queued Network enable and resume, and both
commands returned successfully. Its JS and WASM responses arrived at 16.648
and 16.651, while its visible-button check was still completing; performance-host
navigation began at 16.652 and the worker detached at 16.662. Both drivers use
visible landing readiness before navigating. This observed ordering supports a
scheduling/readiness-check timing difference; it does not prove a semantic
boundary divergence or establish that scheduling alone explains every run. No
filter or behavior fix was justified. Temporary logging was removed. Logging and
paused worker setup can perturb scheduling, and the retained site and dirty
candidate are not source-bound. This run is diagnostic, not timing or ticket
acceptance; source-bound comparison, cache controls, failure controls and the
remaining qualification gates stay open.

## 2026-10-27: delayed activation validation checkpoint

A real Chrome 145 delayed-activation control passed with a 35-second delay.
The runtime timing guard remains in place. This checkpoint does not close the
source-bound comparison, cache and service-worker controls, full watchdog failure
control, or benchmark parity; ticket 006 remains incomplete.

## 2026-10-27: HTTP cache control isolation (diagnostic)

At clean source HEAD `efaae2a9`, the existing Node control failed with server
counts 1 and 1. Its cache context blocked service workers; the failure was not
caused by the warm context's worker. A separate Chrome for Testing 145.0.7622.0
probe used `Cache-Control: public, max-age=3600` and a fresh blocked-worker
context. Two fetches produced one server hit. On a fresh page CDP session,
`Network.setCacheDisabled` alone left the count at 1; enabling Network on that
session first made the next fetch reach the server (count 2). CDP response
metadata showed network, disk cache, network, respectively. In a two-session
control, enabling Network on one session and disabling cache on the other left
the count at 1; disabling it on the enabled session increased the count to 2.
Blocked registration yielded zero registrations.

A separate controlled worker served two responses from CacheStorage with
`fromServiceWorker=true` and zero server hits even with HTTP cache disabled.
Thus a server-hit increase is not a valid unconditional assertion for a
worker-intercepted request. The failed control omitted `Network.enable` in its
own CDP session; Node's capture sends it before disabling cache. Rust source
initializes Network on its page session before capture sends the cache command,
but this probe did not drive the Rust capture and is not Rust runtime proof.
Evidence: `.evidence/ticket006-cache-probe/probe.cjs`, `probe.log`, and
`cleanup.json`; the final probe observed 17 descendants and left none. No
product or capture fix is justified by these observations. The source-bound
warm captures supplied for this investigation remain observations at
`efaae2a9`, not cache-control proof or full ticket acceptance. A real Rust
cache/block control, full failure controls and the remaining qualification gates
remain open.

## 2026-10-27: Q generation comparison and controlled cache capture

The 15-pair `c8712acb` comparison did not pass the two Q rows. Rechecking all
six retained replies for each Q workload showed that each reply's binding and
both analysis references contain the same per-worker UUID, and that this UUID
is stable within each run but differs between drivers. The worker creates it
with `crypto.randomUUID()`; qualification authenticates references against the
worker generation. After replacing only those three generation values with a
common placeholder, all other retained-reply fields match exactly, excluding
the timestamps and durations already outside semantic comparison. The exact
UUIDs and results are in `.evidence/ticket006-c871-parity/q-generation-comparison.log`.
This is a comparison of the existing captures, not a new benchmark admission.

A controlled landing served cacheable page and dedicated-worker resources, a
real service worker, and the existing packaged CAD worker and performance
fixture. The existing Node and Rust `startup:cold` and `startup:warm` capture
paths each exited 0 with a real C1 reply; both missing-CAD-worker controls
exited 1 with a 404 and no retained reply. Cold page fetches hit the origin
both times and no service worker registered; warm page fetches hit once and
both drivers observed activated, controlling workers and service-worker-origin
responses. In both modes the separate dedicated worker's two fetches hit the
origin once: the observed HTTP-cache bypass is page-session scoped, matching
both drivers, and does not establish bypass in worker sessions. Both cold and
warm replies matched on state, ordinal, 492 triangles, 35424 buffer bytes,
zero STEP bytes, and metadata. These are observations, not equal response-count
requirements. The fixture script is `.evidence/ticket006-real-fixture/control.py`; captures
and asserted results are in `.evidence/ticket006-real-fixture-asserted/` and
its launch log records no owned children remaining. Chrome for Testing 145 was run sequentially under the
shared build lock, `boardstudio-tests.slice`, CPU quota 400%, and 12 GiB limit.

These controls are diagnostic: at checkout `2740a52e`, the current packaged
site digest `1cc6ed371db972d62684c8f6f28e214085ac0eeb8214130b4520845d8611ceec`
does not match the `c8712acb` build receipt's site digest
`37b24b492d0e476f263f2c3b416e48483619cccbfb68a84de2e08efe02f770e7`.
The CAD package and official provider input digests match that receipt, but the
site as a whole is not bound to it. A current source-bound build and control,
remaining failure and full qualification gates, and Playwright removal remain
open.

## 2026-10-27: timing decision and Rust default

The user approved a one-sided timing criterion: for each cold/warm worker-startup
and validated-reply metric, the upper end of the 90% paired bootstrap interval
must be at most 1.15. On source `f0b6904`, 12 counterbalanced pairs per mode
gave upper bounds of 0.862 and 0.947 for cold, and 0.950 and 0.963 for warm.
All 48 captures succeeded under low observed CPU and I/O pressure; evidence is
in `.evidence/ticket006-timing-quiet-f0-*`. The earlier two-sided rule fails on
the lower cold worker bound; this decision accepts faster readings without
claiming identical measurement overhead.

The user requested cutover to Rust. The CAD launcher defaults to Rust and admits
startup runtime; U1:page remains validation-only because it has no runtime
samples. Node remains explicitly selectable while final dependency retirement,
full verification and review are outstanding.

## 2026-10-27: verification and open parity gate

The full supported check later passed on clean source
`cca95c5d0709c4fe9ad0ba82ebaacef5e88e3743`; its receipt is
`.evidence/full-check-cca95c5d-EZBJYe/receipt.json`. This is full-check evidence,
not a new source-bound cache/service-worker comparison. The controlled fixture
above remains diagnostic because its site digest did not match its build receipt.
Independent Spec review flagged that distinction; a source-bound control or an
explicit decision on the acceptance requirement remains necessary. Independent
Standards review passed; closure review of this documentation follow-up and hosted
CI remain unrun. This ticket remains claimed and no acceptance box is cleared.

The later retirement decision supersedes the preceding statement that Node remains
selectable: the user explicitly retired historical Cadrum reproduction, and the
current CAD launcher accepts only Rust; historical runner sources are archived.

## 2026-10-27: source-bound cache and service-worker fixture

The official checked build at clean source `8537778b63bcae6e3c68f6b6ee302fe850af0546`
is retained in `.evidence/ticket006-cache-bound-853-build-full/`. The fixture's
before/after bindings matched that build receipt. Sequential Chrome for Testing
145.0.7622.0 runs used the archived Node driver solely as an isolated reference
and the current Rust capture against the same packaged CAD assets and controlled
landing (`.evidence/ticket006-reference-853/receipt.json` and `comparison.json`).
The current supported launcher is Rust-only; the archived Node driver is not a
supported workflow.

For each driver, cold page fetches reached the origin twice and service-worker
registration was blocked; warm page fetches reached it once and the active worker
controlled the page, with 200 responses marked service-worker-origin. In both cold
and warm runs, the dedicated worker's two cacheable fetches reached the origin only
once: cold page HTTP-cache bypass did not bypass that worker's cache. Each successful
capture retained a C1 reply (492 triangles, 35424 buffer bytes, zero STEP bytes).
Each missing-CAD-worker control exited 1, recorded its own non-service-worker 404,
failure, resources and failed result, and retained no reply. Response multiplicities
were recorded, not asserted equal between runs; these are same-run observations.

A separate CDP semantics fixture, not either capture, distinguishes blocking
registration from bypassing an installed worker. Fresh blocking had no registration
or controller and used the origin; blocking registration with an installed controller
still served page and worker requests from the service worker. CDP bypass with an
installed registration served both from the origin and reported no controller. The
first live bypass check incorrectly expected a controller and failed
(`.evidence/ticket006-sw-semantics-853.log`); the corrected live check passed all
three cases, and the retained-observation offline check agreed
(`.evidence/ticket006-sw-semantics-853-corrected.log`,
`.evidence/ticket006-sw-semantics-853-offline-check.log`). This fixture does not
establish generic service-worker bypass in the captures.

This strengthens the previously unbound cache control, not complete ticket closure.
The approved timing result remains the separate `f0b6904` observation above; the full
check passed on `cca95c5d0709c4fe9ad0ba82ebaacef5e88e3743`, not this source or
documentation commit. Hosted CI and closure review remain unrun; other acceptance
requirements remain open and the ticket stays claimed.

## Comments

### 2026-10-27: local closure

Resolved for local scope. Earlier open-gate statements above describe their dated state.
Launcher and schema comparisons, failure controls and tooling evidence are recorded above;
the user accepted the one-sided timing criterion at `f0b6904`, not timing identity or a
speed claim. The source-bound cache fixture at `8537778b` and separate service-worker
semantics fixture establish the conditions described immediately above: cold cache bypass
is page-scoped, and blocking registration is not bypass of an existing worker. Historical
Cadrum reproduction was explicitly retired; its old driver is archived, not supported.

The full supported local check passed on clean source `8cee115c2`
(`.evidence/full-check-8cee115c-fVsvpT/receipt.json`): 2,396 native tests and 690
browser tests, including 113 page tests. It used 400% CPU, 12 GiB, no swap and two
concurrent browser launches, with zero OOM events. Independent final Spec and Standards
reviews of `0b1ed972..8cee115c` passed; Spec found no actionable mismatch. These results
apply to that source, not this documentation-only closure commit. Hosted CI has not run;
the [CI verification plan](008-retire-playwright-dependencies.md#ci-verification-plan)
requires actual run URLs and verdicts.
