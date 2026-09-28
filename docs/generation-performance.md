# Generation and editing performance

Execution of PLAN G1–G8 and V1, starting from detached revision
`0ca5e3f1f71c376135286345f9974066ed8cac45` plus the existing uncommitted
live-preview implementation. No geometry tolerances or benchmark budgets change.

## Phase 3 bounded gasket comparisons

The [continued OCCT screening](occt-optimization-round2.md) measures eight more
variants across five browser sessions and five bottom fixtures. Dedicated cuts,
combined cutters, and analytic profile holes give roughly 4–6% diagnostic gains;
none independently meets the 10% promotion threshold. All 45 native, independent
STEP, and browser mesh comparisons pass. The [E0–E9 coverage table](generation-optimization-status.md)
explicitly retains tabbed meshing, bounded reuse, build tuning, wider correctness,
and full-app acceptance as unfinished work. No production change is selected.

The [2026-09-28 comparison report](gasket-comparison-results.md) adds isolated
Cadrum output suppression and height-band construction tests, Manifold preview,
and Monstertruck feasibility checks. Five fresh browser sessions measured the
same bottom at 108.4 ms median for Cadrum control, 108.8 ms for lean output,
151.6 ms for Cadrum profiles, and 24.1 ms for Manifold preview. Monstertruck did
not complete its first circular-hole Boolean. Geometry checks and failed attempts
are retained with the report. Production code is unchanged; these single-bottom
diagnostics do not replace the full-application Phase 2 results below.

## Latest Phase 2 status

**Phase 2 performance acceptance now passes.** The final unchanged frozen CAD
run (`cad/bench/results/phase2-scene-final/`) passes all **30 scenarios**, including
completion-time, paint-opportunity and sampled RSS limits. Cached-gasket paint
p95 is **31.4 ms**, below the unchanged **38 ms** limit. Controlled alternating
builds attribute the improvement to avoiding redundant preparation and drawing
of already displayed geometry; revision checks and paint timing are preserved.

The final five-session live comparison (`app/performance-results/phase2-scene-live/`)
passes regression and incremental-improvement criteria. Numeric p95 is **171 ms**
(reference 279 ms), Undo **91.3 ms**, gasket **370 ms** (reference 417.6 ms), and
mount **59.3 ms**. **Gaskets still miss the separate proposed 200 ms exact target**;
no provisional geometry is counted as exact generation.

All 399 application tests, 20 focused browser regressions, the 256-cycle full-UI
memory soak, and the complete workbench/matrix/outline/pointer/headless-live suite
pass on the final application build. Earlier failures and diagnostic runs remain
recorded below. The final follow-up section contains current evidence; preceding
sections retain the chronology rather than replacing failed measurements.

## Instrumented reference

`app/performance-results/live-generation-before-2026-09-27.json` records
Chromium 153.0.8010.52, viewport 1280×720, CPU and CAD WASM hash. It adds
bounded opt-in body timings, yields, progress counts, obsolete jobs and save
latency. Its five actions per scenario are diagnostic samples, not independent
sessions or a frozen CAD acceptance comparison.

| Measurement | Before |
| --- | ---: |
| Initial numeric case worker request | 477.8 ms |
| Initial gasket case worker request | 813.3 ms |
| Initial gasket timer waits | 100.9 ms / 29 yields |
| Initial gasket progress messages | 127 |
| Numeric release p50 / p95 | 546.2 / 884.6 ms |
| Undo release p50 / p95 | 429.6 / 579.8 ms |
| Gasket release p50 / p95 | 827.3 / 861.3 ms |
| Mount release p50 / p95 | 499.7 / 543.9 ms |

The initial gasket's largest bodies are bottom (246.1 ms), plate foam
(119.3 ms), plate (97.9 ms), and retainer (43 ms). Numeric, Undo and gasket
saves normally take 4–8 ms. Four mount saves take 112–131 ms, with one at 6.9 ms;
this wall time includes delayed main-thread delivery and does not prove slow
IndexedDB work. The rapid numeric sequence supersedes two preparation jobs
(23.9 and 18.5 ms); its final release-to-paint opportunity is 500.3 ms.

Regression evidence for cooperative supersession: four application assertions
fail before implementation (missing abort protocol, already-aborted dispatch,
hook signal, queued cancellation), and the real WASM body-loop test fails
because all bodies finish despite cancellation. Logs are in
`/tmp/boardstudio-g2-{red,cad-red}.log` for this session.

## Implemented changes

- Supersede obsolete work between synchronous body operations. Abort controls
  bypass the worker queue; successful replies racing cancellation still update
  both sides of the delta cache. Keep the kernel and completed caches warm.
- Share immutable final solids between region and body caches with `Rc`.
  Keep deep copies at Boolean and export ownership boundaries, and retain the
  existing topology, solid, mesh and entry budgets. Native ownership tests
  verify retained regions survive eviction and release after the last owner.
- Promote hits in the body cache. The regression previously rebuilt the hot
  body on edit 64; it now survives 70 edits. Keep the existing region/stage
  invalidation rules: the measured gasket edits really change seven bodies,
  including plate tabs and bottom openings. Expanding retention or ignoring
  changed prepared fields would require separate evidence.
- Yield after a 12 ms work slice instead of every small body. An individual
  synchronous CAD operation can exceed 12 ms; it remains non-preemptible.
  Coalesce progress at 32 ms, preserving first-building and terminal events.
- Keep released geometry/handles visible while the edit is acknowledged;
  restore the prior placement on failure. Compare all exact prepared body
  inputs before reusing a draft, then publish the newly prepared committed
  revision. Remove the 60 ms commit debounce behind the bounded scheduler.
- Preserve autosave ordering. The unusually slow saves coincide with main
  thread delivery stalls; the common 4–8 ms writes do not justify weakening
  the existing durability-before-acceptance behavior.
- Reused buffers receive a new exact display acknowledgement through the
  renderer's zero-delta path. Status changes still do not redraw the scene.

## First combined live diagnostic

`app/performance-results/live-generation-after-2026-09-27.json` uses the same
fixtures and action timestamps as the instrumented reference.

| Measurement | Before | After |
| --- | ---: | ---: |
| Numeric release p50 / p95 | 546.2 / 884.6 ms | 405.3 / 426.6 ms |
| Undo release p50 / p95 | 429.6 / 579.8 ms | 212.7 / 228.1 ms |
| Gasket release p50 / p95 | 827.3 / 861.3 ms | 858.3 / 870.2 ms |
| Mount release p50 / p95 | 499.7 / 543.9 ms | 170.7 / 282.3 ms |
| Initial gasket timer waits | 100.9 ms / 29 yields | 5 ms / 6 yields |
| Initial gasket progress messages | 127 | 11 |

Four of five mount commits reuse the completed exact draft without a CAD
request. Numeric worker execution is 216–221 ms; Undo is 1.5–2.2 ms. Gasket
worker execution remains 394–405 ms, dominated by genuine opening cuts and
plate construction; its end-to-end latency has not improved in this sample.
Preview assembly's `cacheSolidCopy` stage is absent, with mutation-stage copies
still measured. Initial numeric/gasket request timings are 509/781 ms, versus
478/813 ms before; a single cold sample does not establish a startup win.

These are five actions in one session. Frame-after-draw remains a paint
opportunity proxy, not physical display timing. The 200 ms p95 exact refinement
target remains unmet.

## Interactive rendering and constrained run

The in-app workbench was checked through its controls: entering Case, editing
thickness, reaching Preview current, and Undo restoring the value. After replacing
build assets, the first Case navigation was blank and a reload recovered it;
the separately tracked F3 recovery issue is still open.

A separate visible Chromium test browser on the local display was profiled with
CPU, compositor and GPU tracing. Its recorded device is ANGLE/OpenGL ES 3.2 on
AMD Radeon 780M, Mesa 26.2.3. This is a headed test browser, not a trace of the
embedded app browser. The trace summary is
`app/performance-results/generation-headed-trace-2026-09-27.json`; raw trace and
CPU samples remain at `/tmp/boardstudio-generation-headed.cpuprofile*`.

| Action | Visible Chromium p50 / p95 | Synthetic 4× CPU p50 / p95 |
| --- | ---: | ---: |
| Numeric | 251.1 / 278.8 ms | 463.6 / 570.8 ms |
| Undo | 93.2 / 93.8 ms | 294.7 / 340 ms |
| Gasket | 386.7 / 419.3 ms | 551.8 / 599.2 ms |
| Mount | 54.8 / 71.8 ms | 195.1 / 232.1 ms |

The normal warm action windows contain no main-thread long tasks. Longer events
in the broader trace occur around profiling setup and page/module/worker startup;
they are not evidence of a repeated 130 ms compositor stall during these edits.
Pointer work/submission p95 are 0.4/2.3 ms normally and 1.5/8.9 ms under synthetic
CPU throttling. Retained handle updates upload zero handles. No additional
rendering-quality reduction is justified by these traces. CPU throttling is not
a complete mobile-device simulation, and these values must not be treated as a
before/after comparison with headless samples.

The final headless gate records ANGLE/SwiftShader (Subzero), confirming a
different rendering path from the visible Radeon run. Its five live actions
yield p50/p95 of 455.1/564.1 ms numeric, 212.3/227.9 ms Undo, 886.4/987.4 ms
gasket, and 172.4/288.9 ms mount. This second diagnostic shows variance and
does not establish a gasket latency improvement. Pointer work/submission p95
remain 0.4/2.1 ms. See `app/performance-results/generation-full-gates-2026-09-27.json`.

Five additional fresh visible-browser sessions (25 actions per scenario) give
pooled p50/p95 of 266/279 ms numeric, 93/93.8 ms Undo, 386.4/417.6 ms gasket,
and 54.6/71.8 ms mount. Each browser process starts fresh. Raw per-session
reports and `live-generation-five-sessions-2026-09-27.json` are preserved in
`app/performance-results`. The hardware-accelerated mount gestures finish
quickly enough that none of these 25 releases uses the completed-draft shortcut;
they still reuse cached CAD geometry. The before reference only has one live
session, so these repeated runs establish current behavior rather than a formal
multi-session live improvement claim.

## Independent CAD comparison

Five fresh browser sessions before and after, 20 samples per warm scenario and
one cold/cancel sample per session, cover all 30 frozen fixture/scenario pairs.
`cad/bench/results/generation-after-2026-09-27/comparison.json` compares against
`generation-before-2026-09-27`, preserving the historical reference and budgets.
CPU, OS, core/renderer WASM identity, fixture hashes and sample coverage match.
All 30 completion, paint and RSS budgets pass.

Holed-plate warm generation p95 improves from 35.0 to 25.4 ms, and its region-edit
p95 from 41.7 to 31.8 ms. Some tails increase within the frozen budgets: boss-tray
preview/export p95 is 117.7 → 145.0 ms, while its mean is 91.7 → 91.2 ms and mean
STEP serialization remains 14.1 ms. Gasket-pair region-edit p95 is 27.5 → 30.8 ms
with an essentially unchanged 23 ms mean. Cold and cancel runs have only five
samples and remain descriptive. These results support the targeted preview-copy
reduction, not a claim that every scenario became faster. Mesh transfer bytes
are unchanged; the removed copies are internal solid topology copies.

## Memory soak

The first soak completed 480 cycles, 240 each on boss-tray and gasket-pair,
with one warm worker per fixture. Each eight-cycle sequence includes fresh
geometry, return to original geometry, another board context, STEP export,
and a superseded draft. All 60 superseded requests settled as errors and the
following exact requests completed. This exercises production CAD/client/cache
ownership; full React gesture semantics are covered separately by browser tests.

The detailed raw RSS/PSS samples and allocated-WASM measurements are in
`cad/bench/results/generation-soak-2026-09-27`. RSS median in the last quarter
was lower than the previous quarter for boss-tray and within 0.3% for gasket-pair.
Allocated WASM still grew as the caches filled (boss 48.4 → 49.6 MB and gasket
17.1 → 18.4 MB across the last two quarter peaks), so a longer run is required
before describing the allocation trend as settled. RSS sums Chromium descendants
and may double-count shared pages; neither RSS nor cache caps proves leak freedom.

The extended run in `cad/bench/results/generation-long-soak-2026-09-27` completed
1,920 cycles, 960 per fixture, including 240 superseded requests followed by
successful exact requests. Boss-tray allocated WASM plateaued at 49,610,752 bytes
across all four 240-cycle quarters. Gasket-pair reached 20,774,912 bytes in quarter
two and stayed there in quarters three and four. Late-quarter RSS medians declined
for both fixtures. This longer sample supports bounded retained memory for these
workloads; it is not proof for arbitrary projects or a full-day React session.

## Validation and remaining work

- Application: 398 tests in 67 files passed.
- CAD: 33 adapter/fixture/benchmark checks passed; 9 native geometry/cache tests
  passed, with 3 pre-existing diagnostic tests ignored. Renderer: 25 tests passed.
- Browser: 23 focused assembly, live generation, mount, gasket, manual update,
  Undo/Escape and library/responsive workflows passed. The in-app test edit was
  undone; Starter keyboard reports saved locally, thickness 3 mm and Preview current.
- Production build, Rust formatting, repository rules, generated contracts,
  runtime imports and native/WASM boundary parity passed. The build still
  reports its existing large-chunk advisory.
- Existing workbench gate: worker p95 medians 3.0/3.1/4.4/4.5 ms, paint-opportunity
  medians 33.5/33.5/33.5/34.1 ms for 100-single/100-row/200-single/200-row;
  all pass unchanged thresholds. Pointer, matrix and outline gates also pass.
- All 30 frozen CAD fixture/scenario budgets pass with comparable current
  before/after references. Five independent live sessions establish the new
  hardware-accelerated reference; earlier live comparisons remain diagnostic.

Phase 1's eight execution items are complete and uncommitted. The proposed
200 ms p95 exact-refinement target is still unmet for numeric and gasket work.
The next performance investigation should target exact gasket bottom opening
cuts and plate construction, using the hardware-accelerated reference above;
provisional geometry would need separate readiness/export safeguards and cannot
count as exact refinement. Gallery scaling and the observed build-reload failure
remain separate follow-ups in PLAN.md. No cache budget, geometry tolerance,
rendering quality or acceptance threshold was relaxed.

## Phase 2 acceptance protocol

Phase 2 uses the five-session Radeon reference above and the comparable Phase 1
CAD candidate as its starting references; their original artifacts remain intact.
`node app/scripts/run-live-performance.mjs <new-directory> <reference-summary>`
runs five fresh headed sessions, five actions per warm scenario per session, and
compares pooled p95 while retaining each session's p95 and raw reports. The live
comparison requires matching CPU/GPU/browser/viewport and unthrottled CPU, rejects
missing/invalid or undersampled data, and allows at most 10% p95 regression with
4 ms minimum frame-proxy slack. It requires at least 10% improvement in numeric
or gasket p95 for live latency acceptance. The existing CAD policy independently
accepts at least 10% targeted latency or 25% targeted copy/transfer improvement
with all frozen budgets passing. No copy reduction implies a live latency win.

Exact 200 ms acceptance is reported separately for numeric, Undo, gaskets, and
mounts. `compare-live-performance.mjs candidate reference output --require-exact`
returns failure if that proposed target remains unmet; ordinary comparison still
reports it without conflating it with incremental improvement. Four comparison
regressions cover numeric target failure, incompatible/undersampled/invalid data,
unchanged results, and CLI failure exits. The initial tests failed before the comparison module existed and now
pass. Functional tests and timing runs execute separately on a quiet host.

Cold workload: fresh page/worker initialization for numeric and gasket fixtures,
one sample each per session, with initialization, construction, and tessellation
reported separately; five samples are descriptive, not a reliable cold p95 gate.
Burst workload: the existing six successive numeric commits, measured from the
last actual blur through the final exact revision. Record superseded requests
and distinguish kernel work from delivery/scheduling time.

Full-UI soak protocol: 256 edit cycles in one browser session, first 64 as warmup,
including numeric changes/Undo, draft cancellation, view/context changes, and
periodic export and project switches. Sample retained JS heap after explicit test
GC and allocated CAD WASM at regular checkpoints. Compare median samples in the
last 64 cycles against the preceding 64: JS growth must be within max(10%, 2 MiB)
and allocated WASM growth within max(5%, 2 MiB), with no failed/stale final previews
or unsettled operations. Report process-memory samples separately if available;
missing process/GPU memory is not zero. Report actual elapsed time and workload
coverage; this bounded workload does not establish full-day or arbitrary-project
leak freedom. Synthetic 4× CPU results remain separate from physical-device
coverage, which depends on available hardware.

### Phase 2 attribution and selected changes

The new headed diagnostic and captured prepared IR are preserved under
`app/performance-results/phase2-attribution-before/`. They confirm that numeric
wall-thickness edits change outer profiles for the plate, bottom and plate foam;
the bottom foam's prepared geometry stays identical. Gasket moves change the
plate outline, bottom/retainer outlines and closure holes, bottom openings, and
four pad positions. Reusing an entire changed body would be incorrect.

Selected investigations: compare exact opening-cut evaluation order for the
bottom's overlapping pocket cutters and disjoint shallow nut pockets; avoid
pre-opening topology retention/copying when no opening can affect the region.
Plate/foam rebuilding and tessellation are genuine costs, so retain tolerances
and validate any further construction strategy geometrically. Initial generation
shares those construction costs; do not introduce speculative warm-up. The burst
already supersedes preparation work and keeps only the newest pending generation;
measure construction changes first rather than changing cancellation boundaries.

### Phase 2 exact construction results

Remove overlapping cutter interiors only when an earlier rectangular opening
fully covers the later opening's depth and is contained by a simple orthogonal
polygon. Partition its remaining area into exact rectangles, without rounding
coordinates or changing tolerances. Unsupported, invalid, or uncovered profiles
retain the original kernel path. The live gasket fixture verifies both directions
of Boolean material difference, volume, bounds, and STEP roundtrips; additional
checks cover reversed winding and partial depth. Thirteen native CAD tests pass
with four diagnostic benchmarks ignored (three existing, one added experiment).

Skip pre-opening topology retention when no opening intersects the region; the
final region cache already owns that result. The new regression failed because
unused upstream copies were retained, then passed after the guard. Existing
mutation-isolation, invalidation, eviction, geometry and export tests still pass.

Five fresh hardware sessions are in `app/performance-results/phase2-live-after/`.
Compared with the preserved five-session reference, median gasket opening cuts
fall from 112.4 to 98.9 ms (12.0%). Numeric pre-opening copy time falls from 21.2 ms
to zero, with all such copy operations eliminated; gasket copy time falls from
10.3 to 0.9 ms. These meet the numerical stage/copy improvement thresholds, but
overall acceptance also requires the regression gates below. No additional cache
capacity, reduced tessellation quality, or altered Boolean material is used.

The end-to-end result is smaller: numeric p95 279 → 267.4 ms, gasket p95
417.6 → 385.8 ms, Undo 93.8 → 94.6 ms, and mounts 71.8 → 72.7 ms. All live
regression limits pass, but the new live runner exits 1 because neither targeted
end-to-end p95 improves by 10%. Its `improvementPassed: false` result is retained;
this failed criterion is not waived by the stage/copy improvement. The proposed
200 ms exact target also remains unmet. The first session contains a slower
numeric sample; it is retained in the pooled result rather than discarded.

### Cold generation and burst decision

`phase2-live-after/cold-burst.json` retains all five before/after samples. Cold
worker medians are 451 → 454.5 ms numeric and 580.6 → 555.3 ms gasket. These small
cold distributions do not establish a general startup win. The six-commit burst's
last-edit-to-paint median is 267 → 253.3 ms. Construction improvements apply to
those paths without changing scheduling; the measurements do not justify worker
pools, speculative initialization, weakened save ordering, or unsafe kernel
preemption. Retain the existing bounded scheduler and cooperative checkpoints.
Application coverage includes pre-aborted, queued, running and completed-reply
races, stale/context-switched results, and caller settlement.

### Phase 2 full-UI memory soak

`app/performance-results/phase2-ui-soak.json` records 256 completed cycles over
186.3 seconds in one browser session: 128 numeric changes and 128 gasket moves,
each followed by Undo, plus eight verified draft cancellations, board switches,
project switches, and mechanical package exports apiece. The run waits for exact
readiness and settled preview requests. It passes the predefined retained-JS and
allocated-WASM growth limits after 64 warmup cycles.

Comparing cycles 129–192 with 193–256, numeric retained-JS medians are
18,450,528 → 18,569,844 bytes; gasket medians are 18,533,980 → 19,725,036 bytes.
Both increases stay within the declared 2 MiB minimum allowance. Allocated CAD
WASM medians remain 68,747,264 bytes for both projects; individual final samples
reach 68,878,336 bytes. This is a bounded workload result, not proof of leak
freedom under arbitrary projects or full-day editing.

The separate `phase2-ui-soak-rss.json` samples Chromium descendants of the
isolated Playwright process. Approximately aligned final-quarter process-RSS
medians rise from 1,047,175,168 to 1,087,049,728 bytes (3.8%); peaks rise from
1,161,117,696 to 1,237,901,312 bytes. RSS sampling began after startup, can
double-count shared pages, and does not isolate GPU allocations. This observational
increase is not a demonstrated process-memory plateau; the declared pass applies
to retained JS and allocated CAD WASM only.

### Phase 2 correctness and interaction gates

| Check | Result |
| --- | --- |
| Application and worker tests | 398 passed across 67 files |
| CAD JavaScript, fixtures, and comparison regressions | 37 passed |
| Native CAD geometry, cache, and STEP checks | 13 passed; 4 diagnostic benchmarks ignored |
| Native renderer checks | 25 passed |
| Focused browser workflows | 20 passed: live case/mount edits, manual generation, mechanical assembly, saved projects, and small-screen navigation |
| Full-UI memory soak | Passed the declared JS/WASM limits; process-RSS observations reported separately above |
| Production CAD/app builds and TypeScript | Passed; existing application chunk-size advisory remains |
| Repository, contracts, runtime imports, boundary parity, formatting | Passed |
| Standard workbench and interaction performance | Five workbench sessions, matrix/outline/pointer checks, and the headless live diagnostic passed |
| Frozen CAD budgets | Failed: 23/30 scenarios pass in the first run, 28/30 in one fresh verification run |
| Comparable headed live regression limits | Passed for all four scenarios |
| At least 10% numeric or gasket end-to-end improvement | Failed: 4.2% numeric and 7.6% gasket |
| Proposed exact 200 ms p95 target | Failed for numeric and gasket on normal hardware |

`app/performance-results/phase2-full-gates.json` preserves the standard performance
run. Median session worker/paint p95 values are 3.1/33.5 ms for 100 single edits,
3.1/33.4 ms for 100 row edits, 4.5/33.5 ms for 200 single edits, and 4.9/34.2 ms
for 200 row edits; all existing limits pass. Headless live results use software
rendering and are not substituted for the five comparable headed sessions.

`app/performance-results/phase2-live-cpu4x.json` records one additional headed
session with synthetic 4× CPU throttling. Exact edit-to-paint p95 values are
571.4 ms numeric, 313.3 ms Undo, 637.2 ms gasket, and 215.5 ms mount; all miss
200 ms. Pointer-handler and pointer-to-render-submission budgets pass. These
five-action diagnostic distributions do not establish a constrained-device p95
baseline. No physical mobile or other constrained device was tested.

### Frozen CAD failures retained for investigation

Both `cad/bench/results/phase2-after/` and
`cad/bench/results/phase2-verification/` contain five fresh sessions, 20 warm
samples per scenario per session, raw compressed reports, and comparisons against
`generation-after-2026-09-27`. Fixture hashes, required sampling, scenario coverage,
CPU/OS/core/renderer identity all match. Neither run passes the unchanged gate.

The first run fails seven scenarios: gasket preview export, direct export and
cancel/retry; split cold, warm uncached, preview export and direct export. The
slower samples cluster in session five. One full verification run with unchanged
code, fixtures and budgets passes 28/30 scenarios, but still fails split exports:

| Scenario | Completion p95 / limit | Paint-opportunity p95 / limit |
| --- | --- | --- |
| Split preview export | 53.1 / 43 ms | 77.1 / 72 ms |
| Split direct export | 54.9 / 42 ms | 81.1 / 71 ms |

All RSS budgets pass. Verification-session four contains the slower export
samples; the other sessions are close to the reference. These fixtures do not
exercise the new opening reduction and retain identical operation/copy counts.
That narrows investigation but does not prove an environmental cause or rule out
a code/build regression. Do not discard either run, relax budgets, or repeatedly
rerun until green. A controlled same-environment before/after binary comparison
with process/CPU-pressure observations was selected for follow-up; its results
are recorded below. The implementation and validation tasks have been
executed; full performance acceptance remains open.

### Conditional feedback decision (F1)

Recommend a separate outline-feedback implementation for edits that still wait
267–386 ms for exact solids on the measured hardware. The synthetic constrained
case reinforces the need for earlier feedback. This recommendation does not
resolve the failed export budgets or replace further exact-generation work.
This phase records the decision only; no provisional geometry is implemented.

Use the latest successfully prepared, validated profiles and body elevations
from the authoritative core result, keyed by project, board, revision, and draft
identity. Render a visibly distinct outline overlay while retaining the last
valid exact solid. Show “Outline preview · exact solid updating” so the overlay
cannot be mistaken for a finished body. Avoid guessed filled solids, cavities,
clearances, or mount geometry whose accuracy has not been established.

Accuracy scope is the prepared profile boundary, not Boolean volume or material
intersections. Proposed implementation acceptance: validate the rendered world
coordinates against those prepared boundaries with at most the existing 0.1 mm
tessellation tolerance, including concave profiles, holes, elevations, and view
transforms. This is an unverified requirement for the follow-up, not a measured
accuracy claim. Unsupported profiles or inability to establish the bound must
fall back to the retained exact body and updating status.

Keep overlay buffers outside exact-body caches, committed generation state,
readiness, and every export path. Preparation failure must preserve the usable
exact preview and expose the error. Cancel, Undo, project/board switches, stale
replies, and successful exact replacement must remove or replace the overlay
only for the matching context. Cover these races and export isolation before
enabling the feature. Measure time to outline separately; only matching exact
solids count toward the existing 200 ms exact-generation target.

### Follow-up: controlled investigation of split export tails

The continuation compared the current release binary with a separately built
control that disables only the two Phase 2 production optimizations: opening
remainder reduction and the no-opening upstream-cache guard. Both keep current
instrumentation. Production source and assets were not replaced. Generated
JavaScript bindings match byte-for-byte; the diagnostic refuses incompatible
bindings or stale production WASM. The control patch, source hashes, binary
hashes, and ten raw sessions are in
`cad/bench/results/phase2-export-paired/`.

Five pairs alternate current/control order, with a fresh browser per variant and
20 samples per scenario per session. The first export and existing warm-up are
retained. This focused diagnostic uses one-second host observations rather than
the full runner's 10 ms RSS sampler; comparisons are within this paired run.

| Split export | Current completion p95 | Control completion p95 | Frozen completion limit |
| --- | --- | --- | --- |
| Preview then export | 38.9 ms | 35.9 ms | 43 ms |
| Direct export | 36.7 ms | 37.8 ms | 42 ms |

Both variants also stay within the paint limits. This does not demonstrate a
consistent export slowdown caused by the two optimizations, nor establish their
absence under every workload. The earlier failed tails did not reproduce.

An additional five-session full-workload diagnostic preserves the original
fixtures, sampling and RSS method while adding CPU/memory-pressure, swap/reclaim,
CPU-frequency, and process-CPU observations. Its results and analysis are in
`cad/bench/results/phase2-export-host-diagnostic/`. Split completion p95 is
41.7 ms preview/export and 34.6 ms direct export; all 30 scenarios fall within the
budget numbers. **This run is explicitly ineligible for acceptance** because of
the added instrumentation. It neither replaces nor clears either failed run.

The host uses `amd-pstate-epp`, `powersave`, and `balance_power`; no host settings
or unrelated processes were changed. Each roughly 47-second session records
356,285–843,035 system-wide swapped-in pages and 448,988–547,252 swapped-out
pages. Memory-pressure `full avg10` ranges from 1.17% to 3.90%. These are
system-wide observations, not memory attributed to Board Studio, and do not
prove the cause of the earlier export failures. They do show that “no competing
build/test process” is insufficient evidence of an idle host. One-second samples
also cannot resolve every short interference event.

No production change is justified by this investigation. Keep the original two
export-budget failures open until a comparable idle-host acceptance run and, if
needed, controlled reproduction can resolve them. Do not tune budgets, introduce
speculative export warm-up, or roll back exact optimizations based on an
unconfirmed cause. The 10% live improvement criterion and 200 ms exact target
remain unmet independently of these export diagnostics.

Validation for this follow-up: both diagnostic runners completed; seven focused
comparison tests pass, including rejection of diagnostic acceptance and
incompatible control bindings. Production code was unchanged, so the earlier
geometry, application, and browser results remain the relevant functional checks.

### Phase 2 completion candidate: exact planar meshing

The remaining numeric bottleneck was meshing flat plates and foam sheets.
`construction/planar_mesh.rs` now generates an exact conforming-grid mesh for
rectangular outer profiles with disjoint simple orthogonal holes. The CAD kernel
still builds and retains the same solids for export. Openings, mounts, other
feature cuts, non-orthogonal/invalid/touching/overlapping profiles, collapsed float
coordinates, or grids above 4,096 cells use the existing kernel mesher. Work is
also bounded by 1,024 hole edges. No CAD tolerance or cache budget changes.

Shared grid vertices prevent T-junctions. Tests verify every triangle's winding,
closed oppositely oriented mesh edges, volume against the kernel, sampled inside
and outside material, both contour windings, negative elevation, and fallback
conditions. The new-path regression failed before implementation; the resulting
native suite passes 16 tests with four diagnostic experiments ignored.
The real WASM suite passes 39 tests, including retained cache and STEP behavior;
all 398 application tests and the production builds also pass.

The first live attempt, `phase2-planar-live/`, stopped after three sessions when
the first mount gesture was blocked and did not create a revision. It is retained
as incomplete. The test had reconstructed the camera from asynchronous scene
bounds. Mount targets now use the actual draw's camera matrix, read during setup,
and each saved mount coordinate is asserted after its timed action. No application
gesture behavior changed. Numeric/gasket inputs and timing boundaries are unchanged.

Five fresh completed sessions are in
`app/performance-results/phase2-planar-live-verified/`:

| Scenario | Phase 1 reference p95 | Final candidate p95 | Improvement | Exact 200 ms |
| --- | --- | --- | --- | --- |
| Numeric | 279.0 ms | 166.1 ms | 40.5% | Pass |
| Undo | 93.8 ms | 89.5 ms | 4.6% | Pass |
| Gasket | 417.6 ms | 337.6 ms | 19.2% | Miss |
| Mount | 71.8 ms | 54.0 ms | 24.8% | Pass |

All live regression limits and the 10% incremental-improvement criterion pass.
The proposed exact target remains unmet for gaskets; this is not an all-scenario
200 ms result. Numeric median kernel meshing drops from 72.9 to 1.8 ms compared
with the first Phase 2 candidate, plus 1.2 ms in the new planar mesher. The
numeric fixture emits 5,544 rather than 4,024 newly tessellated triangles, and
copies 902,592 rather than 683,712 mesh bytes (32.0% more). This is an explicit
tradeoff to validate against rendering and retained-memory gates. Across the five
sessions, pointer-work p95 is 0.4–0.5 ms and render-submission p95 is 2.5–2.7 ms.

The unchanged frozen CAD harness used five independent sessions and 20 warm
samples per scenario per session. It passes 29/30 scenarios, all completion
limits, and all sampled RSS limits. The cached gasket fixture has the same
triangle count as before and does not enter the planar mesh path. Its cached
CAD p95 remains 0.9 ms, while paint-opportunity p95 rises from 33.3 to 42.3 ms
against 38 ms. Slower samples include an extra delay between render submission
and the next animation frame. The harness still prepares/uploads a full scene
for cached results. These observations locate the remaining delay after CAD;
they do not establish its cause or justify changing the timing method, renderer,
or budget. This is a failed acceptance gate, not a waived measurement.

The final full-UI soak (`app/performance-results/phase2-planar-ui-soak.json`)
passes all predefined growth checks after 256 cycles in 154.9 seconds, with
8 exports, 8 cancelled drafts, 8 board switches and 8 project switches. Numeric
post-warmup JS medians are 18,516,592 → 18,668,280 bytes; gasket medians are
19,431,120 → 19,750,152 bytes, each within the 2 MiB allowance. Observed CAD WASM
capacity remains 45,744,128 bytes in both comparison windows. Process memory
was not sampled in this UI soak; the frozen CAD harness's RSS check is separate.
This bounded run does not establish that memory cannot grow in longer sessions.
The final focused browser run also passes all 20 live case/mount, manual
regeneration, mechanical assembly, and project-library regressions.

The unchanged workbench/interaction runner initially exited 1. Matrix, outline
and pointer checks passed. Its `100-row` worker median was 4.400000035762787 ms
against 4.4 ms: a timestamp-subtraction residue of 0.000000035762787 ms. A new
comparison regression failed first; the comparator now rounds to nanoseconds
before comparing finite values, with tests retaining real over-budget failures.
Re-evaluating the preserved five-session samples passes all four workbench
scenarios (`phase2-planar-workbench-comparison.json`); this is explicitly a
comparison replay, not a fresh measurement. No baseline or budget was changed.

The runner's headless live test also exposed mouse-coordinate quantization in
the new saved-mount assertion (5 mm intended, 4.7865 mm saved). The assertion now
projects the saved position back through the captured camera and requires each
axis to match the requested pointer within one CSS pixel, rather than imposing
zoom-dependent 0.05 mm precision. It still rejects a blocked or wrong move and
requires the saved revision. The focused headless rerun passes, with its own
artifact `phase2-planar-headless-verified.json`; the initial failed report remains
preserved. The five comparable headed sessions already passed the stricter
world-coordinate assertion; their timing boundaries and samples are unchanged.

The final 4× CPU diagnostic (`phase2-planar-cpu4x.json`) passes the functional
and interaction assertions. Exact release-to-paint p95 is numeric 403.2 ms,
Undo 271.8 ms, gasket 510.8 ms and mount 159.5 ms. Pointer work stays within
4 ms and render submission within 33 ms. This is one synthetic throttled
session on the desktop GPU, not physical mobile-device coverage; no physical
constrained device was available.

Final repository verification passes: 447 authored modules checked, generated
contracts, runtime-import contracts, Rust boundaries, application/CAD TypeScript,
and whitespace checks. The two new comparison regressions pass. Production CAD
and application builds, 16 native CAD tests (4 diagnostics ignored), 39 real-WASM
CAD checks, 398 application tests, and the 20 focused browser checks are green.

At the end of the planar-mesh iteration, **Phase 2 acceptance was not complete**.
TODO retained a controlled frame-pipeline investigation and a comparable full
acceptance run for the cached-gasket paint failure. The following continuation
resolves that gate. The conditional-feedback decision remains applicable to
gasket editing, which still exceeds the proposed 200 ms exact target.

### Final Phase 2 follow-up: reuse identical displayed scenes

A fresh focused five-session run (`phase2-cache-paint-before/`) reproduced the
cached-gasket failure: 100 samples, CAD completion p95 1.0 ms and paint-opportunity
p95 43.3 ms. The slowest samples spent approximately 28–29 ms between draw
submission and the next frame. Every cache hit re-prepared and redrew the same
full scene despite already retained mesh buffers.

The renderer client now compares an immutable incoming scene with its accepted
base metadata and currently applied bodies. Identical bodies in the same order,
matching metadata, and `keepCamera: true` can reuse the displayed geometry.
An empty prepared patch advances the renderer's existing revision guard; the
unchanged frame-after-draw acknowledgement still runs. No-op acknowledgement
preserves an already scheduled camera/state redraw. Explicit Fit requests,
changed metadata or geometry, unfamiliar metadata values and comparisons above
the bounded work/depth limit fall back to normal preparation. Superseded worker
replies remain rejected. No renderer API, geometry, timing boundary or budget
was changed.

The new regression failed before implementation. It covers no redundant worker
or draw work, retained camera redraws, geometry/theme/board changes, explicit
camera resets, obsolete replies, full scenes after body patches and renderer
rejection. The existing body-delta and paint-attribution checks remain in place.

`app/scripts/diagnose-cached-scene.mjs` alternates complete control/current app
builds over five fresh-browser pairs, with the frozen gasket fixture, identical
WASM hashes, 20 warm samples per variant/session and diagnostic-only counters.
All asset hashes and raw samples are retained in `phase2-cache-paint-paired/`.
The instrumented run is explicitly ineligible for acceptance:

| Diagnostic | Control | Identical-scene reuse |
| --- | ---: | ---: |
| Pooled paint-opportunity p95 | 43.5 ms | 31.1 ms |
| Scene-worker requests over 100 hits | 100 | 0 |
| WebGL draw calls over 100 hits | 400 | 0 |

The paired result supports eliminating redundant scene work as a fix for this
cached presentation workload. It does not replace the full unchanged acceptance
run or imply that exact gasket construction meets 200 ms.

The subsequent full frozen acceptance run is
`cad/bench/results/phase2-scene-final/`: five sessions, 20 warm samples per
scenario/session, unchanged fixtures, profiling, RSS sampling and budgets.
**All 30 scenarios pass completion, paint and RSS limits.** Cached-gasket
paint-opportunity p95 is 31.4 ms (38 ms limit); CAD completion p95 is 1.5 ms
(4 ms limit). The latter is higher than the earlier 0.9 ms reference and within
its frozen budget; no claim is made that renderer reuse speeds CAD construction.
Previously failing split export scenarios also pass. The earlier failed runs
remain available and are not overwritten.

Final renderer application checks pass 399 tests, including the new regression;
20 focused browser regressions cover live edits, mount/gasket gestures, Undo,
cancel, manual generation, assembly and saved-project switching. Repository
checks cover 448 authored modules; contract checks, boundary parity, whitespace
and timing-comparator regressions pass. CAD and renderer WASM are unchanged from
the prior validated planar candidate; this follow-up rebuilt the application.

Five fresh comparable hardware-accelerated live sessions are retained in
`app/performance-results/phase2-scene-live/`. The final candidate passes every
live regression limit and the incremental improvement criterion:

| Scenario | Phase 1 reference p95 | Final p95 | Improvement |
| --- | ---: | ---: | ---: |
| Numeric | 279.0 ms | 171.0 ms | 38.7% |
| Undo | 93.8 ms | 91.3 ms | 2.7% |
| Gasket | 417.6 ms | 370.0 ms | 11.4% |
| Mount | 71.8 ms | 59.3 ms | 17.4% |

Gasket p95 is higher than the previous planar candidate's 337.6 ms. The retained
`gasket-attribution-comparison.json` checks this tail: median CAD execution is
239.6 → 239.5 ms, renderer preparation 3.1 → 3.1 ms, and renderer upload
1.5 → 1.7 ms. The slowest sample's CAD execution increases 249.4 → 263.6 ms;
its body-patch completion increases 6.5 → 9.3 ms, with no recorded main-thread
long tasks. These observations do not establish the cause of cross-run tail
variation or a regression from scene reuse. Retain the final 370 ms result;
do not select the faster prior run as the final result. Gaskets still miss the
separate 200 ms exact target, and the earlier provisional-feedback decision
remains applicable.

The final UI soak (`phase2-scene-ui-soak.json`) passes 256 cycles in 160.1 s,
including 8 each of exports, cancellations, board switches and project switches.
Post-warmup numeric JS medians are 19,073,516 → 18,639,252 bytes; gasket medians
are 18,559,616 → 18,700,364 bytes. Observed CAD WASM capacity is stable at
45,678,592 bytes in both projects' comparison windows. Process RSS was not
sampled in this UI soak; the frozen CAD run's sampled RSS results are separate.
These bounded-session results retain the prior soak's limitations.

The final standard performance runner exits successfully and writes
`phase2-scene-full-gates.json`: all four workbench scenarios pass across five
fresh sessions, followed by passing matrix, outline, pointer and headless-live
checks. Unlike the earlier comparison replay, this is a complete fresh run.
Application TypeScript checks also pass after the final regression-test edits.

The final synthetic 4× CPU session (`phase2-scene-cpu4x.json`) passes functional
and interaction assertions. Exact p95 is numeric 464.9 ms, Undo 278.6 ms,
gasket 545.6 ms, and mount 183.3 ms; only mount meets 200 ms under throttling.
Pointer work remains within 4 ms and render submission within 33 ms. This single
synthetic desktop session is diagnostic evidence, not a physical mobile-device
result or a substitute for the five unthrottled acceptance sessions.

All seven main TODOs and three follow-up items are complete, with no blocked or
skipped tasks. Phase 2's frozen regression and incremental-improvement acceptance
is satisfied; the separate 200 ms gasket refinement target remains unmet.
TODO.md and CHANGELOG.md record completion. PLAN.md remains read-only under the
invoked TODO skill and can now have its Phase 2 status aligned in a separate
planning update.
