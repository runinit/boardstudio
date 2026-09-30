# Generation and edit performance plan

## Immediate prerequisite: recover preserved outline work

Before resuming pending Phase 3 performance work, recover the outline and related
mechanical changes preserved in `codex/outline-mechanical-wip`. Snapshot modified
and untracked source files with hash verification, create an isolated integration
worktree from current `dev`, and capture the preserved edits as a recovery commit
in an isolated checkout of their original base. Reconcile them with newer `dev`
changes using three-way integration, regenerate contracts, and verify affected
Rust, app, build, export, and browser workflows before integrating into `dev`.
Preserve source worktrees, unrelated edits, existing task states, and baseline
failures; do not reset, discard, or overwrite newer work. These prerequisites do
not complete any existing performance task.

## Objective and scope

Make case generation, regeneration, and repeated edits finish sooner while the
editor remains responsive. Prioritize actual computation and obsolete work, then
the handoff from edited geometry to the displayed result. Preserve exact geometry,
manufacturing readiness, one-step Undo, and recoverable cancellation.

Updated 2026-09-28 after reviewing the completed implementation and the gasket,
parametric construction, Cadrum/OCCT, and alternative-kernel discussions. Phases 1
and 2 are committed in `49d33a91` (`Improve workspace workflows and live CAD
performance`). Preserve that implementation and its retained benchmark evidence.

[TODO.md](TODO.md) records Phase 2 as complete: seven main items and three
follow-ups, with none pending, blocked, or skipped. Phase 3's selected bounded
comparisons are now complete; the remaining optimization experiments and
production integration are unexecuted. Its detailed protocol is in
[Generation optimization experiments](docs/generation-optimization-experiments.md).
Measured outcomes and revised priorities are in
[Gasket comparison results](docs/gasket-comparison-results.md).
The **full E0–E9 program remains open**. The first comparison was a bounded
subset, not a replacement for the other experiments. Follow the
[coverage table](docs/generation-optimization-status.md) for pending work and the
[second OCCT screening](docs/occt-optimization-round2.md) for continued testing.
The separate proposed 200 ms p95 exact-generation target remains unmet for gaskets.

The current planning constraint is to preserve the case design. Optimize the
generation algorithm and Cadrum/OCCT implementation without moving closure
hardware, replacing tabs with ledges, changing gasket placement/linkage, or
altering material boundaries and tolerances. Kernel migration is deferred.

## Evidence and current baseline

Primary evidence:

- [Latest implementation, validation, and remaining limits](docs/generation-performance.md#latest-phase-2-status).
- [Final Phase 2 hardware-accelerated live summary](app/performance-results/phase2-scene-live/summary.json).
- [Final Phase 2 CAD comparison](cad/bench/results/phase2-scene-final/comparison.json).
- [Phase 1 live reference, retained for history](app/performance-results/live-generation-five-sessions-2026-09-27.json).
- [Extended CAD editing soak](cad/bench/results/generation-long-soak-2026-09-27/memory-trend.json).
- [Live-preview architecture, review, and validation](docs/live-preview-ui-review.md).
- [CAD benchmark protocol](cad/bench/README.md), [frozen budgets](cad/bench/budgets.json), and [workbench limits](app/performance-baseline.json).

The current live reference contains five fresh visible Chromium sessions and
25 actions per scenario, using Chromium 153.0.8010.52, Ryzen 9 8945HS, Radeon
780M through ANGLE/OpenGL ES 3.2 and Mesa 26.2.3, and a 1280 × 720 viewport.
The values below are pooled action percentiles. Inputs are timestamped at DOM
blur, Undo click, or pointer release; completion is a frame-after-draw paint
opportunity for the exact revision, not physical display time.

| Scenario | Phase 1 p95 | Final Phase 2 p95 | Implication |
| --- | ---: | ---: | --- |
| Numeric case edit | 279 ms | 171 ms | Preserve the accepted construction/meshing gain |
| Undo | 93.8 ms | 91.3 ms | Preserve the existing cached-generation and display path |
| Gasket release | 417.6 ms | 370 ms | Still above 200 ms; prioritize exact construction and meshing |
| Authored mount release | 71.8 ms | 59.3 ms | Preserve exact reuse, warm caches, and commit acknowledgement |

The earlier live before reference contains only one headless session. It cannot
establish a multi-session improvement against this hardware-accelerated reference.
Historical [pre-Phase 1 timings](app/performance-results/live-preview-perf-final-2026-09-27.json)
and [instrumented before/after results](docs/generation-performance.md) remain
available; they are not the current baseline. Keep software-rendered headless
runs and synthetic CPU throttling separate from hardware and real-device results.

The Phase 1 hardware-accelerated trace found no main-thread long tasks during normal
warm actions; pointer work/submission p95 were 0.4/2.3 ms. It did not reproduce
the large compositor waits seen in earlier automated headless diagnostics.
Additional rendering-quality reductions are not supported by this evidence.

Gasket edits genuinely change seven bodies, including plate tabs and bottom
openings. Preview assembly topology copies have been removed; copies needed
before mutating Boolean operations remain. Exact construction and safe stage
reuse are the next investigation. Do not add stage medians to infer an
end-to-end percentile or assume changed geometry can be reused unchanged.

Initial generation has only descriptive cold samples, so no startup improvement
is established. The 1,920-cycle CAD soak supports bounded retained memory for
its two fixtures. Phase 2 also passed the 256-cycle full-UI JS/WASM soak; process
RSS was not sampled in that UI soak. Neither establishes arbitrary-project or
full-day memory behavior.

## Completed foundation to preserve

- The 44 px header, saved-keyboard gallery with separate demos, contextual
  toolbars, and navigation/return controls are implemented in this worktree.
- Automatic case preview, pause/manual update, one active plus one replaceable
  pending job, warm-worker supersession, and context/revision guards are present.
- Draft geometry stays outside committed readiness and export. Mount validation
  uses authoritative prepared case regions, holes, and intersecting openings.
- Body deltas and scene patches retain unchanged mesh buffers, PCB/models, camera
  state, and handle meshes. The combined-mesh compatibility adapter stays lazy.
- Redundant status-driven redraws, identical-scene draws, unchanged-handle draws,
  and repeated bundled-catalogue generation have been removed.
- Profiling is opt-in and bounded. Actual input timestamps and core/kernel/scene
  attribution distinguish waiting, computation, and rendering.
- Phase 1 verified 398 application tests, 33 CAD adapter/fixture checks, 9 native
  CAD tests (3 existing diagnostic tests ignored), 25 renderer tests, and 23
  focused browser tests. Production build and repository/contracts/boundary
  checks passed, as did existing workbench, matrix, outline, and pointer gates.
- All 30 frozen CAD fixture/scenario budgets passed using comparable five-session
  before/after references with required sample coverage. Some tail latencies
  increased within those budgets; this is not a claim that every case improved.
- The extended CAD soak completed 1,920 cycles including 240 superseded requests;
  allocated WASM memory plateaued for both tested fixtures. This does not prove
  bounded memory for arbitrary projects or a full-day React session.
- Phase 2 added exact opening-cutter reduction, removed unused upstream cache
  copies, and added guarded planar meshes while retaining CAD solids. Identical
  displayed scenes now skip preparation/drawing while advancing revision guards.
- Final Phase 2 validation passes all 30 frozen CAD scenarios, the live regression
  and incremental-improvement checks, 399 app tests, 20 focused browser tests,
  full workbench/interaction gates, and the full-UI soak. Earlier failed reports
  remain preserved; the final gasket p95 is 370 ms, not the earlier 337.6 ms run.

## Phase 1 — Faster exact generation and changes (complete)

The eight execution items below are completed in this worktree. Their evidence
and limitations are retained in the implementation report and changelog.

| Item | Completed outcome | Boundary carried forward |
| --- | --- | --- |
| G1 | Added bounded per-body, queue, progress, cancellation, persistence, initial-generation, and rapid-edit diagnostics. | Cold samples are descriptive; detailed remaining-stage attribution belongs to Phase 2. |
| G2 | Added cooperative supersession between bodies, caller settlement, and client/worker delta-cache race handling. | An individual synchronous CAD operation remains non-preemptible. |
| G3 | Shared immutable final solids across preview caches and verified ownership/eviction and geometry/export equivalence. | Keep deep copies before mutation/export operations that require isolation. |
| G4 | Replaced unconditional per-body delays with 12 ms work slices and coalesced progress at 32 ms. | Preserve cancellation checkpoints and first/terminal progress; slices cannot preempt a synchronous body operation. |
| G5 | Promoted body-cache hits and measured stage invalidation/evictions without raising budgets. | Seven gasket-affected bodies genuinely change; further exact reuse requires complete input equivalence. |
| G6/G7 | Retained released geometry through commit acknowledgement, reused matching exact drafts, and removed the 60 ms commit debounce. | Keep save ordering: normal 4–8 ms writes did not justify changing durability-before-acceptance. |
| G8 | Profiled visible hardware-accelerated Chromium and synthetic 4× CPU throttling; verified normal warm actions had no main-thread long tasks. | Synthetic throttling is not a real mobile device, and the headed test browser is not the embedded app browser. |
| V1 | Passed correctness/build checks, all 30 frozen CAD budgets, existing interaction gates, five live sessions, and the 1,920-cycle CAD soak. | The 200 ms exact target remains unmet; full-UI memory and cold-generation conclusions need separate evidence. |

Primary implementation areas:

| Area | Current files |
| --- | --- |
| Supersession and progress | [generation hook](app/src/useCaseGeneration.ts), [scheduler](app/src/livePreviewScheduler.ts), [CAD client](app/src/CaseClient.ts), [worker](app/src/case.worker.ts), [body loop](cad/src/preview.ts) |
| Exact generation and reuse | [construction](cad/wasm/src/model/construction.rs), [caches](cad/wasm/src/model/construction/cache.rs), [metrics](cad/wasm/src/model/metrics.rs) |
| Commit and display handoff | [gestures](app/src/ui/AssemblyScene.tsx), [project acceptance](app/src/useProjectSession.ts), [renderer client](app/src/renderClient.ts) |
| Evidence | [live diagnostic](app/e2e/live-preview-performance.spec.ts), [CAD runner](app/scripts/run-cad-benchmark.mjs), [CAD fixtures and budgets](cad/bench/README.md) |

## Phase 2 — Remaining exact-generation latency (complete)

Sections 2.1–2.7 retain the original execution requirements corresponding to the
seven completed [TODO.md](TODO.md) items. They are historical specifications, not
pending work. Final evidence is in the
[performance report](docs/generation-performance.md#final-phase-2-follow-up-reuse-identical-displayed-scenes).
Frozen regression and incremental-improvement acceptance passed; the separate
200 ms gasket target did not. The provisional-feedback item produced a decision
only, with no provisional rendering implementation.

### 2.1 Acceptance and comparable reference

The current live diagnostic's `exactReleaseP95` result checks only gaskets and
mounts and records a boolean without enforcing acceptance. Include numeric edits,
report every scenario separately, and add executable comparison checks with
regressions that demonstrate failures are detected. Keep the proposed 200 ms p95
exact target distinct from the frozen regression budgets and incremental-gain
criteria; a passing incremental optimization must not imply that target is met.

Preserve the existing five-session hardware reference. Record source/WASM hashes,
fixture identity, CPU/GPU, browser, viewport, and sample coverage for comparisons;
the base commit alone cannot identify this uncommitted implementation. Use a
fresh comparable reference only if provenance or the selected protocol requires
it, without replacing historical artifacts or lowering sampling requirements.
Retain the existing policy: at least 10% targeted latency improvement or 25%
targeted copy/transfer reduction, with all frozen regression budgets passing.

Before optimization, define repeatable cold-start and latest-edit burst workloads,
the live comparison aggregation and sample coverage, and full-UI soak duration,
warmup, workload, memory metrics, and post-warmup growth criteria. Keep small cold
and cancellation samples descriptive. Record worker completion and exact-revision
paint opportunity separately; neither measures physical display latency.

### 2.2 Targeted attribution

Depends on 2.1. Identify changed prepared inputs and construction stages for
gasket bottom cuts, plate construction, and numeric edits. Measure cold startup
and latest-edit completion during bursts separately. Rank candidate changes by
measured cost, record the proposed optimization or evidence-based no-change
decision, and avoid repeating completed copying, scheduling, or rendering work.

### 2.3 Gasket bottom opening cuts

Depends on 2.2. Optimize the measured exact opening-cut workload. Before changing
behavior, establish geometry and cache-invalidation regression coverage. Verify
bounds, material, volume, and STEP equivalence, then measure the focused result
against 2.1. Preserve Boolean semantics, mutation isolation, tessellation
tolerances, cache budgets, and warm-worker cancellation behavior.

### 2.4 Plate construction and numeric edits

Depends on 2.2. Implement changes in measured impact order, reusing a stage only
when all relevant region, opening, elevation, mount, geometry, and tessellation
inputs match. Plate tabs can change the outer profile, so unchanged-body reuse
must not be assumed. Validate each change independently with pre-change
regressions, geometry/cache/export equivalence, and focused timing before moving
on. Preserve one-step Undo, unchanged-body deltas, and rendering quality.

### 2.5 Cold generation and rapid edits

Depends on 2.2. Address these costs only if attribution justifies a change;
otherwise record a measured no-change decision. Verify latest-edit completion,
bounded active/pending work, settlement of every superseded caller, and rejection
of stale or old-context results. Retain safe cancellation boundaries and test
before-start, queued, in-progress, and completed-reply races for any affected
behavior. Validate each implementation independently against 2.1.

### 2.6 Combined acceptance and full-UI soak

Depends on 2.3, 2.4, and 2.5. Run five comparable fresh hardware-accelerated live
sessions and the CAD harness's required independent sessions/sample counts. Run
relevant native CAD/renderer, application/worker, browser, production build, and
repository checks, plus unchanged CAD/workbench/interaction performance gates.
Each optimization must already have passed its focused checks; this combined
run checks interactions and overall regressions.

Run the full-UI soak defined in 2.1 across drafts, Undo, cancellation,
project/board changes, and export. Distinguish JS, WASM, and process-memory
observations where available; do not present the earlier CAD-only soak as full-UI
coverage. Record constrained-device testing and identify unavailable real-device
checks explicitly; synthetic CPU throttling is separate evidence.

Publish exact scenario latency, cold/burst results, memory trends, regressions,
artifact provenance, and unmet targets in [the report](docs/generation-performance.md).
Retain existing budgets and investigate worsening tails even when they remain
within those budgets. F2 and F3 are not prerequisites for this item.

### 2.7 Conditional feedback decision (F1)

Depends on 2.6. If exact generation still misses the proposed 200 ms p95 target,
document whether a separate provisional-feedback implementation is justified.
Define authoritative inputs, accuracy limits, visible provisional status,
fallback behavior, and strict exclusion from readiness/export. If unnecessary,
record that decision. This item produces a decision, not automatic implementation;
provisional timing cannot satisfy exact-generation acceptance.

## Phase 3 — Test further optimizations without changing the case design

The [experiment plan](docs/generation-optimization-experiments.md) defines the
hypothesis, eligibility, risks, correctness checks, and decision rules for each
experiment. Isolated E0/E1/E7 screening and the selected Manifold/Monstertruck
comparisons are complete; production code is unchanged. Across five browser
sessions, unused mesh output made no meaningful difference, Cadrum height-band
profiles were about 40% slower, and Manifold preview was about 4.5× faster for
the measured bottom. Monstertruck did not complete the first hole operation.
See the [full results and limitations](docs/gasket-comparison-results.md).

The second screening tests history suppression, OBB filtering, specialized cuts,
combined cutters, analytic holes in extrusion profiles, non-destructive mode,
and final cleanup. E4 tabbed meshing and E6 bounded reuse remain distinct,
unexecuted experiments with confirmed eligible inputs. Retain Cadrum for exact
validation/export and keep Manifold integration separate. The table below retains
the broader inventory; the coverage table distinguishes partial tests from full
acceptance and records the remaining build/ownership/whole-app work.

| Order | Experiments | Deliverable |
| --- | --- | --- |
| 1 — P0 | E0: reproducible controls, geometry fixtures, detailed Cadrum/OCCT attribution | A verified patch/build path and ranked cost breakdown |
| 2 — P1 | E1/E2: omit unused mesh edges/face IDs and unused Boolean provenance | Independently measured variants retaining ownership protections |
| 3 — P1 | E3/E4: OCCT OBB/operation selection and exact meshing for eligible tabbed plates | Equivalent geometry and measured end-to-end effects |
| 4 — P2 | E5/E6: better batching/cutter preparation and narrowly scoped parametric reuse | Less repeated work within existing cache budgets |
| 5 — P2 | E7: construct the identical stepped bottom from profiles | A bounded alternative construction prototype with solid/STEP equivalence |
| 6 — P3 | E8/E9: safe copy reduction, cleanup/build tuning, threading feasibility | Evidence-based adoption or deferral only where remaining cost justifies it |
| 7 — Acceptance | Combine successful variants and run correctness, CAD/live, interaction, and memory gates | Accepted changes, retained raw evidence, and explicit unmet targets |

Measure the final Phase 2 implementation as the incremental control. Do not
repeat completed batching, caching, or renderer work, reuse changed geometry,
skip ownership copies without proof, or treat native-threading results as browser
performance. The published OCCT options need version/build verification and
per-workload measurement; none is an assumed speedup.

Parametric principles here mean stable feature dependencies and equivalent
construction. Fixed closure hardware, continuous support ledges, and different
mounting mechanics remain separate design decisions. The bounded Manifold preview
and Monstertruck experiments do not count toward this phase's exact-generation
claim. Production kernel replacement and deferring solid validation until export
remain unselected.

## Separately tracked follow-ups

| ID | Priority / status | Next action and acceptance |
| --- | --- | --- |
| F3 | P2 resilience; observed, cause unconfirmed | Reproduce the blank first Case navigation after replacing build assets across build activation and offline/online transitions. Stale service-worker/lazy-chunk mismatch remains a hypothesis. Fix the confirmed cause with recovery/asset consistency tests, preserving saved work and drafts and avoiding destructive forced reloads. |
| F2 | P3 gallery; scaling unmeasured | Benchmark opening, search, and scrolling with 10, 50, and 200 saved projects. Choose summary records, lazy previews, or virtualization only if measurements justify them. |

Neither follow-up is evidence of a CAD bottleneck or part of the current TODO.
Worker pools, kernel replacement, speculative warm-up, indexed transport, and
STEP-only export remain deferred. Phase 3 includes only a conditional OCCT
threading feasibility investigation, not a selected worker-pool implementation.

## Success criteria and boundaries

- Preserve one active plus one replaceable pending job, warm-worker reuse on
  supersession, and independent export/model-import ownership.
- Keep pointer callback p95 at or below the proposed 4 ms target and submission
  p95 at or below 33 ms. These do not establish physical display latency.
- Improve initial generation and newest-edit completion with comparable evidence.
  Retain the proposed 200 ms release-to-exact-geometry target as **unmet** until
  demonstrated; a provisional mesh or faster status update cannot satisfy it.
- Unchanged geometry must not be rebuilt, transferred, or uploaded unnecessarily.
  Superseded and old-context results must never become current or export-ready.
- Preserve tessellation tolerances and Boolean semantics. Verify exact geometry
  and export equivalence for CAD changes rather than relying only on screenshots.
- Verify Escape, invalid moves, commit failure, exactly one Undo step, linked
  gaskets, authoritative mount boundaries, project/board/instance switches,
  pause/manual update, worker errors, and retained view state.
- Keep existing cache budgets and performance thresholds. Measure memory/RSS
  after repeated edits and cancellations; do not infer absence of leaks from
  bounded entry counts alone.
- Use release artifacts for performance runs on a quiet host, preserve original
  reports, record provenance, and distinguish diagnostic samples from accepted
  comparisons. Run functional checks separately from timing measurements.

## Tracking

TODO.md remains the completed Phase 2 checklist, with sections 2.1–2.7 mapping
to its seven main items and three completed follow-ups. The selected Phase 3
screening is recorded in the comparison report, CHANGELOG, and retained raw
artifacts. The rest of the linked experiment plan remains unexecuted; no production
integration or full-application acceptance is claimed. Generate a separate
execution checklist when a follow-up implementation is selected. Preserve
Phase 1/2 history and all failed benchmark artifacts.
