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
