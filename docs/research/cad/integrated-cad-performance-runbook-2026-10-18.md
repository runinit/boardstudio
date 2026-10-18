# 011 executable preparation

This is untimed preparation at parent `1abc1b3793e8acf6af3e6bba84d87cd47ea40d20`.
007's actual movement/default-budget work is still active. Root owns stable-source
acceptance and the quiet measurement window; this note does not change ticket 011.
No build, benchmark, package installation, CAD implementation or shared contract
change belongs to this preparation slice.

The [retained plan](integrated-cad-performance-plan-2026-10-18.md) governs the finite
comparison. Historical Cadrum stays read-only and detached at
`6c9c4867755bd2316f751783729a3ff9778fc04a` in
`/home/chris/.t3/worktrees/ts-boardstudio2/t3-2088b1aa`.
Its 20,294,309-byte provider is release/locked/**no-opt**, with a successful old
15-check smoke. It is not an optimized artifact or a full page/worker build.
The older 19,785,714-byte package mentioned in the original plan is unbound and
must not be used for deltas.

Retained admissions under that historical lane's `.evidence/011-historical-baseline/`
are provider-only: C1, C2, C3, P1, V2 and K1 appearance. Preserve their independent
analytic/material/source checks and original source/artifact bindings. They are not
admissions of the eventual post-007 provider. Panasonic's historical refusal is
superseded by the current 155-face complete-surface admission; it must be included
in fresh default-budget admission and never counted as a refusal-speedup.

The exact layered Case/PCB, source PCB composition, printable keycaps and owned
qualification-v2 work already delivered remain unchanged. Qualification source
proofs are distinct from actual repair movement measurements. Unsupported classes,
cone/reflection collision proofs and parity/global containment remain Unknown;
no general Boolean requirement is introduced.

## Prepared tools and exact inputs

[measure-cad.py](../../../cad/scripts/measure-cad.py) adds a finite schedule,
read-only binding/size inventory, ordinary build capture, packaged-worker runtime
capture and descriptive CSV reduction. It never cleans, installs, creates a
worktree, changes source, or starts an automatic run matrix. The small
[browser driver](../../../cad/scripts/measure-cad-runtime.cjs) uses existing
`playwright-core`, the existing preview server/headers and **actual packaged Rust
CAD worker**. No provider double, alternate kernel or new dependency is used.

[Frozen requests and their origins](../../../cad/test/fixtures/performance/README.md)
retain C1plate/C2trayboss/C3gasket/P1analyticPCB/K1keycap. V1switch/V2vendor read the
original committed STEP files and verify manifest SHA before dispatch. Qvendor
reads the unchanged shared generated-analysis fixture, authenticates original0805,
obtains real owned worker references, and uses only the fixture's authored rows,
threshold and limits. Fixture example reports/certificates are never installed.

Paired runtime rows are preview and exact STEP for C1/C2/C3/P1, default V1/V2
imports, K1 appearance and **separately admitted** K1 printing. Refinement at0.01
and owned Qvendor default/tight queries are current-only rows: old Cadrum has no
equivalent refinement-option/owned-query boundary. No missing baseline row is a
presumed win. Legacy keycap printing still requires its own output-quality
admission; the operation name is not proof of equivalent fused print material.

The driver checks success, captured identity, buffer shape/finite coordinates,
body IDs and complete current vendor face/occurrence coverage outside the timer.
These are supplemental guards. Reuse the retained **independent** material,
volume, source-face witness, placement and STEP checks on the fresh bound outputs
before admitting them. No tolerance or repair budget is widened for measurement.
Keep selected budget, declared precision, actual movement and direct source-band
proof separately in the root's admission note. Null/unknown stays null/unknown.

## Safe preparation commands (no builds/browser execution)

Run from this lane. Output directories must be new and below this lane's
`.evidence`; failures are never overwritten. `bind` is read-only, even when its
checkout is the historical baseline. It uses the existing receipt digest helper,
official CAD input helper/stamp and Git object IDs, not a new source hash manifest.

```sh
python3 -B cad/scripts/test-measure-cad.py
node --check cad/scripts/measure-cad-runtime.cjs
node cad/scripts/test-measure-cad-runtime.cjs
python3 -B cad/scripts/measure-cad.py plan --output .evidence/011-plan
python3 -B cad/scripts/measure-cad.py bind --profile checked --output .evidence/011-current-binding
python3 -B cad/scripts/measure-cad.py bind --checkout /home/chris/.t3/worktrees/ts-boardstudio2/t3-2088b1aa --profile checked --output .evidence/011-old-binding
```

The schedule is finite: five alternating pairs per admitted runtime row/profile,
one cold plus five warm observations per session, separate20-operation retention
sequences, cold/warm startup blocks, and36 build invocations (three cycles ×
two backends × two profiles × clean/no-op/incremental). Read the CSV in order;
do not launch rows concurrently. A failed pair remains recorded and invalidates
that pair. Admission failures stay explicit missing comparisons, not latency
samples. No acceptance threshold or statistical-significance claim is invented.

## After root supplies stable source and a quiet window

Root first records the post-007 accepted full commit, artifact build/admission
references, exactly admitted rows, missing baseline capabilities, cache/provisioning
state and the quiet-window interval in an ordinary `.evidence/011-admission.log`.
This is the existing external coordination requirement, not permission for this
preparation lane to start measurements. `--source-head` must match exactly; a
runtime capture also requires that admission note and a successful full-build
capture whose source/profile/provider/site binding still matches.

Provision **separate measurement workspaces** for each source/profile/clean cycle
with distinct resolved root/CAD targets and generated outputs. Historical
`t3-2088b1aa` and its artifacts remain read-only: the build action explicitly
refuses that path. No reset, branch rebinding, deletion or old-source restoration
into parent is part of this protocol. Root provisions the historical source via
T3 when ready. Verify the pinned final container image digest/SDK and verified
prebuilt OCCT archive from the retained plan; normal baseline builds link that
archive and compile the bridge, not the whole SDK/OCCT. If provenance/prerequisites
drift, stop rather than letting the helper pull/build a fallback SDK image.

Example **later** single build observation, using root-provided absolute paths
and full source commits in these shell variables. `HARNESS` is this lane, while
`MEASURE` is the dedicated build workspace. Do not point `MEASURE` at the protected
historical lane. Checked uses `--fast` (release/no-opt CAD and unoptimized page);
release uses the ordinary optimized build. Their numbers never share a group.

```sh
python3 -B "$HARNESS/cad/scripts/measure-cad.py" build --checkout "$MEASURE" --source-head "$ACCEPTED_HEAD" --profile checked --pair 1 --state clean --output "$HARNESS/.evidence/011-run-old-checked-1-clean"
python3 -B "$HARNESS/cad/scripts/measure-cad.py" build --checkout "$MEASURE" --source-head "$ACCEPTED_HEAD" --profile checked --pair 1 --state noop --output "$HARNESS/.evidence/011-run-old-checked-1-noop"
```

For Cadrum, only after verifying the pinned local image/archive, prefix those
commands with `CADRUM_IMAGE_READY=1`. The capture uses `python3 scripts/build-web.py
--fast` or `python3 scripts/build-web.py` in `MEASURE`, with normal project flags,
jobs/debug settings and a7200-second watchdog. No concurrency override is added.
Clean means the dedicated generated outputs do not exist; the tool refuses to
clean any existing output. Keep provisioned registries/toolchains/images/archive
caches and ordinary OS page cache documented. Clean is not cold-disk/provisioning.

For incremental, root adds the same harmless comment to each registered
`cad/wasm/src/lib.rs`, runs `build ... --state incremental`, and restores **only**
that owned comment after capture. The exact diff is in the binding log. Confirm
the official CAD stamp changes and the build log actually recompiles CAD; no-op
stamp hits must not be relabelled incremental. Do not modify the protected baseline
lane or use a native-test-only edit/mtime change as the stimulus. Record state
before/after the owned restoration, since later runtime binding must match the
source actually used to build. Build logs preserve stage commands and timings
printed by the normal helper; `stdout-timing.csv` timestamps each observed output
line/stage boundary and the outer capture records total wall seconds.

After each clean/incremental build, `sizes.csv` records raw and deterministic
gzip9/mtime0 bytes for provider, workers, renderer and full packaged site. These
are separate product views: only sum the site group for the **nonduplicated**
shipping total; do not add the copied provider group again. Stamps alone do not
authenticate outputs: package/site digests and byte equality of packaged CAD
JS/WASM to the official provider are checked separately. Never use the retained
old no-opt package size against a new optimized package as a delta.

Example **later** admitted worker observation:

```sh
NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules python3 -B "$HARNESS/cad/scripts/measure-cad.py" runtime --checkout "$MEASURE" --source-head "$ACCEPTED_HEAD" --profile checked --pair 1 --workload C1plate:preview --build-run "$BUILD_CAPTURE" --admission-note "$HARNESS/.evidence/011-admission.log" --output "$HARNESS/.evidence/011-run-new-checked-C1preview-1"
```

Replace `--workload` with a row from `schedule.csv`; replace the output name on
every invocation. Use `--pair 0 --growth` for the separate20-operation sequence.
For `startup:cold`/`startup:warm`, the driver creates a new browser process/profile
each time; cold disables HTTP cache and service workers, warm primes navigation
with caches enabled and waits for an available service-worker registration to
activate before measured navigation. Record registrations/controller/resource
responses. OS file cache remains ordinary. Browsers are owned and closed after
capture; no user/global browser cache is flushed.

Operation cold means modules ready and bytes resident in a fresh CAD worker,
then dispatch to completed identity-checked typed reply. Warm repeats use fresh
request IDs and the same accepted revision/geometry/options. There is no Runtime
asset cache in the isolated worker host: vendor warm is repeated importer work,
not an application-cache hit. STEP creates a fresh worker/module for **every**
observation, including warm browser/network runs, preserving real export lifetime
and `m1-independent-step-cache-miss`. Worker ready is timed separately. All
latencies use a single host/page clock for both endpoints; no page/worker time
origins are subtracted. Each operation has a180-second watchdog with no retry.

Process sampling records per-PID PSS/RSS in KiB at50ms, separating the owned
Chromium descendant tree from the Node controller. Sum a **single timestamp's**
browser rows for tree PSS/RSS, then find the peak; never sum process high-water
marks. `events.log` and `memory.csv` share Unix wall timestamps for phase alignment,
with monotonic sample timestamps retained. Each reply holds its result for a
fixed100ms outside latency, drops page references, then dwells100ms for retained
samples. Natural GC stays unchanged. Keep idle, reply-retained, after-drop and
growth20 samples separate. Sampling can miss short peaks; unavailable/disappearing
PIDs stay explicit and must not be counted as zero. Worker WASM capacity is not
exposed by the production API; do not widen it to invent a number. The returned
typed-buffer/STEP bytes are separate metrics, not total kernel memory.

After independent admission and interference review, reduce capture directories:

```sh
python3 -B "$HARNESS/cad/scripts/measure-cad.py" report --inputs "$OLD_RUN" "$NEW_RUN" --admission-note "$HARNESS/.evidence/011-admission.log" --output "$HARNESS/.evidence/011-report"
```

`summary.csv` reports the median/min/max of **session** medians and session count;
the five warm repeats remain one session. `pairs.csv` retains each pair's absolute
`new_minus_old` and ratio `new_over_old`: for time/memory/bytes a ratio above1 means
larger/slower, not an improvement. Growth pair0 is not a paired timing. Failed or
drifted captures make reduction fail and remain in `failures.log`. Compare the
run list to `coverage.csv`/`report.log` for missing rows and success/failure counts;
partial output is never full acceptance. Do not use duplicate session directories.
Mixed source/artifacts within a backend/profile or different driver/workload
bindings across backends also fail reduction.

## Explicit prerequisites and limits

- No builds/timings ran in this slice. The current lane has no generated targets,
  provider or site. Root's already-green native/page/security/review evidence is
  reused; none was repeated or relabelled performance evidence.
- Root must freeze accepted007 actual movement/default-budget semantics and the
  stable010/qualification-v2 source, then refresh official artifacts/admissions.
  Direct source proofs may include repair; they do not separately measure movement.
- The retained old provider/smoke is fresh historical **no-opt provider-only**
  evidence. Full old packaged page/worker, optimized products, three build cycles
  per profile and fresh stable-current products still need provisioning/capture.
  `runtime` deliberately refuses provider-only packages or stale official stamps.
- Independent admissions for the fresh artifacts and K1 printing remain required.
  The earlier C2/P1 oracles and corrected PCB chord-support check remain retained;
  do not replace them with this driver's nonempty-buffer checks.
- **U1 remains a concrete missing comparison**, pending one frozen accepted
  `.boardstudio` project admitted by both complete loaders, exact selections,
  camera/viewport, and accepted-revision/render/download markers. Use the existing
  actual-page 2×2 rigid Case/physical-PCB preparation in `test-drag-interactions.cjs`
  as the starting procedure; freeze its accepted project outside timers after
  stable source. Retain parent checks for source PCB composition/ring islands.
  No project is fabricated or made baseline-compatible by deleting new geometry.
- The executable startup block measures **usable landing**, isolated real-worker
  readiness and first C1 typed reply. It does **not** measure usable editor,
  first visible project geometry, application asset-cache startup or U1 STEP
  download. Those endpoints need the admitted frozen project above.
- `K1keycap:print` measures real fused-print worker delivery only. The full binary
  STL/identity manifest/ZIP/download must additionally use the existing Runtime
  `export_keycaps_stl` action in U1, with25 successful per-key STLs and unchanged
  socket/material/legend checks. No JS STL substitute is timed as Runtime export.
  Page STEP download likewise remains distinct from the worker STEP byte reply.
- The browser driver has only untimed syntax/protocol validation in preparation;
  the first later untimed admission must validate it against both actual packaged
  workers. Any failure is diagnosed before timed pairs, with original logs kept.
  There is no final011,012, independent review, hosted-CI or migration-complete claim.

For a quiet window, root names its start/end and sole operator, pauses all agent
builds/checks/benchmarks using these targets/CPU, verifies no Cargo/wasm-pack/Dioxus/
container build is active, and checks load/CPU pressure/governor before/after each
pair. Capture records processes, pressure, tool versions, relevant environment,
resolved paths, source/diff, official input stamp, package/site digests, and workload
Git blobs. Record retained registry/toolchain/cache/image state in the admission
note, including the historical container's effective jobs (its narrow mounts
exclude root `.cargo/config.toml`). If interference occurs, preserve and invalidate
the whole pair, obtain a new quiet window and explain any subsequent replacement.

Preparation validation: three Python protocol tests and four controlled Node
protocol checks pass; the Node checks execute no browser/provider and deliberately
reject a154-face Panasonic payload. Browser-driver syntax, exact retained-input
equality, new-document links and Git whitespace checks pass. Read-only checked
bindings confirm the old official stamp matches its no-opt package, while this
lane has no package/stamp/site/targets. These are not acceptance measurements.
Evidence: this lane's `.evidence/011-preparation/`; no parent evidence was copied.
