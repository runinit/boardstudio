# 006: Rust CAD runtime capture

Status: claimed
Type: build
Blocked by: [Rust drag smoke end to end](005-rust-drag-smoke.md)
Parent: [Chromiumoxide browser checks](../map.md)
Related: [Real-browser checks drive Chrome from Rust](../../cad-deepening/issues/016-rust-browser-harness.md)

## What to build

The existing CAD measurement workflow invokes the Rust harness instead of Playwright and
produces equivalent measurement artifacts and verdicts. Extend the shared harness only where
this consumer requires it; keep the old capture available until comparison passes.

## Acceptance

- [ ] Preserve launcher inputs, capture arguments, timeouts, exit codes, CSV columns/events
  log fields, source/build binding, readiness/currency checks and failed-capture semantics.
- [ ] Old/new capture against the same packaged build and inputs proves equivalent events,
  statuses and output schema. Compare nondeterministic timing fields by their documented
  meaning, not byte identity; do not claim a speed improvement from this port alone.
- [ ] Cold/warm HTTP-cache and service-worker conditions match the old runner. Prove bypass
  versus blocking explicitly, including a controlled worker fixture; retain response status
  and service-worker-origin metadata without dropped required events.
- [ ] Preserve protocol self-check coverage without a browser. Keep any Node-based tooling
  tests that still serve that contract; removing all Node is not this migration's goal.
- [ ] Shared isolated browser launch/error handling is reused. Negative controls prove
  malformed replies, page failures, capture identity drift and incomplete observations fail.
- [ ] Run affected tooling tests and real old/new capture parity before switching the default;
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
