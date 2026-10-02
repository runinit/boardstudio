# Ticket 06 performance candidate protocol

Status: full acceptance remains open. Reviewed pointer repairs pass one focused
session and two complete `b9748745` candidate sessions, but the paired attempts
stopped on reference-server availability and candidate startup-readiness errors.
The [first attempt](runs/paired-pointer-b9748745-20261002T0014Z/stopped-aggregate.json),
[retry](runs/paired-pointer-b9748745-retry-20261002T0018Z/stopped-aggregate.json)
and [public import diagnosis](candidate-import-diagnostic-b9748745-20261002.md)
remain retained; no five-session aggregate is claimed. A new source completion
for active-project restoration and instance selection requires a fresh release
before final paired timing. The [unchanged reference gates](runs/reference-gates-20261002T002536Z/assessment.md)
completed separately: UI/live frozen timing comparisons fail; CAD completed all
five sessions and per-row budgets pass, but its frozen comparison is ineligible
because OS and core/renderer WASM identities differ. All nine pinned inputs
remained unchanged.

Historical `8f509433` evidence below remains a failure at 100/200-key caps. The
public reference import adapter was repaired after its navigation-readiness race;
budgets and accepted fixtures remain unchanged.

Evidence-only performance changes are integrated on
`codex/m1-production-20261001`; the adapter was prepared from final-release
integration `8f509433` with later evidence commits. Pointer archives were generated from the verified public Core package
whose source commit is retained in each fixture manifest. The exact maintained
release was served at `http://127.0.0.1:46880/`. Do not use measurements from
the 4cb diagnostic build as M1 release evidence.


The [stopped aggregate](runs/paired-pointer-8f509433-20261001T2328Z/stopped-aggregate.json)
retains all completed and interrupted observations. Candidate session p95s were
22.9/135.3/495.8 ms and 23.1/134.4/500.6 ms for 30/100/200 keys,
against unchanged 33/50/100 ms caps. Reference 30-key observations alone cannot
establish a paired result. The [public import diagnosis](reference-import-diagnostic-20261001.json)
records the repaired asynchronous hydration/readiness check.

## Existing oracles to preserve

- Pointer-to-painted interaction: use 30, 100 and 200 keys; 10 warmups and 100
  measured movements; assert a visible part transform changes; preserve p95
  limits of 33, 50 and 100 ms. The source is
  `app/e2e/pointer-performance.spec.ts`.
- Mounted workbench: preserve the reference host, 1280x720 viewport, DPR 1,
  four 100/200-key single/row cases, 10 warmups and 100 measured samples per
  case, five serial sessions, and the comparator in
  `app/scripts/run-performance.mjs` against
  `app/performance-baseline.json`. Keep worker and painted endpoints and the
  existing exact limits/comparator unchanged.
- Live scenarios, if the candidate exposes equivalent user workflows: retain
  five hardware-accelerated sessions, the existing CPU/browser/GPU/viewport
  provenance, the existing sample coverage, and
  `app/scripts/compare-live-performance.mjs`'s reference plus
  `max(4 ms, 10%)` allowance. State separately whether the CLI's optimization
  improvement criterion passes; a migration-only non-regression result is not
  an optimization claim.
- CAD comparisons are independently owned by the coordinator and must retain
  all 30 frozen scenarios, eligible matching WASM/renderer identity, existing
  fixture/sample checks, budgets and `cad/bench/compare.mjs`. No candidate
  result can qualify while the browser host is busy with a build, wasm-opt, or
  browser QA.

The historical pointer spec and workbench spec expose benchmark globals and
mount their own fixture. Those are reference harness seams, not suitable
candidate interactions under the M1 evidence rules. Candidate pointer timing
must start from physical browser pointer input delivered to the rendered
workbench and end after its visible transform changes on a painted frame. The
candidate must load an archived project through its visible file-import flow;
do not call application debug globals, session internals, direct worker
constructors, or private mutation handlers.

## Candidate setup and run record

Before a measurement session, record the immutable release URL and route, exact
source commit plus dirty/untracked content hashes, build command and completed
asset hashes, fixture and archive hashes, browser version, Node version, CPU,
GPU/renderer path, viewport, DPR, cache state, power/thermal state, and UTC start
and end time. Keep the raw input/output and every failed attempt. A repair gets
a new run record; never overwrite a failed run.

For pointer sizes, `generate-pointer-archives.mjs` reproduces the existing
reference benchmark fixture shape (30/100/200 keys, three parts per key) as an
input generator. It opens every fixture with the public Rust `CoreEngine`, packs
and unpacks it with the public Rust archive API, and writes a unique fixture
directory with source/module/WASM/project/archive SHA-256 identities. Point it
at the exact already-built public core artifacts; it never invokes a build:

```sh
BOARDSTUDIO_CORE_MODULE=/absolute/path/to/boardstudio_core.js \
BOARDSTUDIO_CORE_WASM=/absolute/path/to/boardstudio_core_bg.wasm \
node .scratch/m1-production/evidence/performance/generate-pointer-archives.mjs
```

The script prints the unique fixture directory. Supply that directory, the
release URL, and an exact release record containing `sourceCommit` and
`assetHashes` to `run-candidate-pointer-sessions.mjs`:

The release record includes the full source state, build command/toolchain,
asset hashes and the coordinator's quiet-host signal. For example, the runner
requires `sourceCommit` and an object-valued `assetHashes`; it stores the entire
record unchanged with every run.

```sh
BOARDSTUDIO_FIXTURE_DIR=/absolute/path/to/generated/fixtures/<unique-id> \
BOARDSTUDIO_CANDIDATE_URL=https://candidate.example/boardstudio/ \
BOARDSTUDIO_RELEASE_RECORD=/absolute/path/to/release-record.json \
node .scratch/m1-production/evidence/performance/run-candidate-pointer-sessions.mjs
```

Run fixture generation only after the coordinator provides the verified core
provider package/build artifacts. Run the pointer sessions only after the
coordinator signals that release build and browser QA are complete and the host
is quiet.

The runner invokes the browser driver in five fresh `agent-browser` sessions
by default. Each session uploads each archive through the real `.boardstudio`
input and delivers movement with native browser mouse controls. A generic page
observer timestamps the received `pointermove` event, the transformed DOM
mutation and the first following `requestAnimationFrame` opportunity; it
neither calls application internals nor injects edits. Each fixture has 10
warmups and 100 measured samples in every session. Raw samples, environment,
exact browser command output, asset/source identity and failed attempts are
retained under a unique run directory. The existing pointer caps apply to each
session p95.

The candidate timing start is the received native event. The existing
Playwright pointer harness arms `beginPointerSample` before its
`page.mouse.move` call and waits for its own first-frame helper. This observer
ends at a DOM-mutation-following animation-frame opportunity. Keep those raw
timing semantics explicit; without a valid equivalent endpoint, do not claim
exact relative comparison to historical pointer samples.

The existing five-session workbench comparator remains unpaired/ineligible for
the candidate: its reference runner invokes a hidden benchmark function and
measures worker-reply-to-React-layout endpoints, while the candidate must be
driven through public UI input and the accepted pointer endpoint is pointer to
painted transform. Do not apply the workbench relative limits to this different
endpoint. Root will run the unchanged reference performance runner after the
candidate build; exact comparative workbench/CAD evidence remains unperformed
unless a valid same-operation public comparator is established.

Use the current REVIUNG41 and Sofle project copies as accepted functional
fixtures for the functional browser gates. The 30/100/200 benchmark archives
above exercise the reference's accepted generated assembly workload and are
additional performance inputs; they do not replace REVIUNG41/Sofle behavior
comparisons. These fixtures are evidence inputs only; generation must not add a
second implementation of domain rules.

Drive the candidate's labelled `.boardstudio` import control with the generated
archive. Use the real keyboard editor canvas and real browser mouse/pointer
input. Capture transform mutation and its following animation-frame opportunity
with browser-level observation only; the observer must not issue edits or
affect event delivery. For every pointer sample retain the received native
event timestamp, first changed DOM transform timestamp, following animation
frame timestamp and target part identifier. Retain raw samples before
calculating p50/p95. The observation must show that the target stays in view
and is not occluded. This endpoint is a paint opportunity, not physical display
presentation.

## Fresh paired pointer observations

The same driver has an optional public-UI adapter for the unchanged React
reference distribution in `app/dist`. The reference app imports each archive
through its visible **Open project…** control and observes the actual rendered
`.wb-scene-part` target. The candidate uses its actual `.boardstudio` file input
and rendered part. Both adapters select the same `SWn`/generated-key mapping,
use native browser mouse movements, and measure from capture-phase `pointermove`
receipt through the target SVG transform mutation and the next rAF opportunity.
The endpoint is a common browser-observed paint opportunity; it does not claim
physical display presentation. Existing benchmark globals and hidden hooks are
not called.

`record-reference-dist.mjs` pins every byte in the prebuilt `app/dist` tree by
SHA-256. Its record currently covers 143 files with tree hash
`5a3d371f115c5bd325f35e7efacf3a3ffde74587de538611b3ef2f7f940dabbd`.
`release-8f509433.json` records the final candidate build's 47 root-route asset
hashes and source/build provenance; the measured-run URL and quiet-host signal
are pinned in the record. For a new run, use the route-verified release record
and run the paired form with
`BOARDSTUDIO_REFERENCE_URL`, `BOARDSTUDIO_REFERENCE_RECORD`,
`BOARDSTUDIO_CANDIDATE_URL`, and `BOARDSTUDIO_RELEASE_RECORD` set. The runner
alternates candidate/reference order by fresh session and retains every raw
sample and failed attempt. It marks observations eligible only when archive
bytes, target identity, browser/renderer, CPU, OS, viewport/DPR, and sample
counts match in all five sessions for all three sizes. A paired p95 delta is an
observation only: no relative threshold is defined or introduced. The frozen
33/50/100 ms absolute candidate caps stay unchanged.

The reference adapter's feasibility was established by source inspection:
`ProjectStart` and `Workbench` expose `.wb-open-project`, and the real archive
input runs `onImport`; `CanvasObjects` renders `.wb-scene-part` with its own
`transform` and native pointer handlers. The reference and candidate use the
same imported archive bytes and browser-level endpoint, unlike the historical
workbench worker-to-layout comparator, whose endpoints cannot be aligned with
this interaction. No measurement is eligible until final release QA completes
and the coordinator confirms the host is quiet.

## Maintained `8f509433` pointer observations

The quiet-host candidate run retained two complete sessions before the
reference adapter's navigation race was diagnosed and the wrapper was stopped.
Candidate session p95 values were 22.9/135.3/495.8 ms and 23.1/134.4/500.6 ms
for 30/100/200 keys, against the unchanged 33/50/100 ms caps. Thus 30 keys were
within the cap in both sessions; 100 and 200 keys exceeded their caps in both.
Each completed scenario has 100 samples, 100 changed target transforms and zero
missed input markers. Two reference 30-key scenarios completed at 21.7 and 20.3
ms; both later reference sessions stopped at 100-key import because immediate
navigation queried before the React public controls mounted. Their 30-key
differences are descriptive partial observations only. They do not establish a
three-size, five-session paired comparator or a relative pass/fail result.

The readiness fix waits for either `.wb-project-trigger` or `.wb-open-project`
after navigation, opens the visible Project toolbar when mounted, then waits for
the visible Open project control. A separate no-timing browser diagnosis
successfully imported the same 30/100/200-key archives through the visible
ProjectStart/Workbench controls and observed 90/300/600 scene parts. See
`reference-import-diagnostic-20261001.json` and
`runs/paired-pointer-8f509433-20261001T2328Z/stopped-aggregate.json` for the
complete diagnostic and raw run paths. The URL-corrected candidate release
record is `release-8f509433-route-verified.json`; the original input record is
preserved because its hash is embedded in every run record.

Compare candidate sessions only when the exact reference can be replayed under
the same browser, host, archive bytes, viewport, DPR, warmup/sample schedule and
hardware acceleration with equivalent endpoints. For the existing workbench
five-session comparator, use a candidate adapter only if it can time the same
worker-complete and painted endpoints through actual UI operations on an
identical accepted workload. If a public interaction cannot create the same
single/row preview operation or one endpoint cannot be observed defensibly,
record the comparator as unavailable/ineligible; do not substitute the old
debug benchmark globals or invent a new limit.

Before release measurement, obtain the coordinator's quiet-host signal after
the final release build and browser QA. Recheck URL and release asset hashes
against that build before the first run. Performance data from the React
reference or the historical performance documents supplies context only; it
is not a fresh comparison for the exact integrated source.

## Inspection notes

At this snapshot, `web/src/runtime.rs` contains the real archive import flow and
`web/src/presentation.rs` owns the labelled file input and SVG pointer handlers.
The existing app performance harness lives in `app/src/bench-workbench.tsx`;
its `window.runWorkbenchBenchmark` and pointer globals are explicitly excluded
from candidate operation. The production source has no exposed public
performance API. The maintained 8f509433 pointer run is incomplete and fails the
100/200-key caps in both complete candidate sessions; no full paired comparator
or final five-session pointer gate is established. The coordinator's unchanged
reference workbench and CAD suites remain separate gates.

## Cached CAD worker diagnostic

`cad-preview-diagnostic/` contains a separate, ineligible diagnostic of the public worker request path. `server.mjs` serves an existing immutable release site root and the frozen `cad/bench/fixtures/gasketed-pair.json`; `page.html` asks the public core worker to prepare that assembly, keeps one prepared body, then sends stable-identity public `preview` frames to a single CAD worker. The measurement starts immediately before `Worker.postMessage` and ends at the matching reply event. It excludes renderer and paint work, and includes cross-worker message delivery. One first request primes the worker, followed by 20 samples.

The captured 4cb release returned 20 warm samples with median 0.3 ms, p95 0.6 ms, and maximum 0.9 ms; the initial prime took 81.2 ms. This does not show timer-yield nesting dominating this one-body warm worker path. It is diagnostic only and cannot establish or compare the frozen 38 ms cache-hit painted-time oracle. Full build, worker/WASM, fixture, and source provenance is in `cad-preview-diagnostic/provenance.json`; raw samples are in `cad-preview-diagnostic/raw-result.json`. The diagnostic used a separate browser session and its temporary server has been stopped.

The follow-up batch run also kept the full two-body fixture and synthetic 18/58-body cached batches on the same worker and identity. Synthetic bodies clone the core-prepared `gasket-tray` body with unique IDs and names while retaining its geometry and revision. Each scenario had one prime request followed by 20 warm requests. The two-body fixture measured p95 1.6 ms; synthetic 18-body p95 was 58.0 ms and synthetic 58-body p95 was 231.0 ms. These runs overlapped the parent's focused page build and are explicitly ineligible. The synthetic results show cumulative worker request cost at these batch sizes, consistent with a per-body yield, but do not establish failure of the frozen painted-time budget. See `cad-preview-diagnostic/batch-result.json` for every sample.
