# Generation and edit performance plan

## Objective and scope

Make case generation, regeneration, and repeated edits finish sooner while the
editor remains responsive. Prioritize actual computation and obsolete work, then
the handoff from edited geometry to the displayed result. Preserve exact geometry,
manufacturing readiness, one-step Undo, and recoverable cancellation.

This plan records the findings from the UI overhaul, live-preview review, and
performance investigation through 2026-09-27. The current checkout is detached at
`0ca5e3f1f71c376135286345f9974066ed8cac45` with substantial uncommitted work.
Completed work below is present in this worktree, not a claim about the committed
revision or another checkout. Preserve these changes when implementing this plan.

[TODO.md](TODO.md) is the ordered execution checklist for Phase 2: seven pending
items, with implementation not yet started. Phase 1's eight execution items are
complete and recorded in [CHANGELOG.md](CHANGELOG.md) and the
[generation performance report](docs/generation-performance.md). Completing
Phase 1 did not meet the proposed 200 ms p95 exact-generation target.

## Evidence and current baseline

Primary evidence:

- [Phase 1 implementation, validation, and remaining limits](docs/generation-performance.md).
- [Five-session hardware-accelerated live reference](app/performance-results/live-generation-five-sessions-2026-09-27.json).
- [Current CAD comparison](cad/bench/results/generation-after-2026-09-27/comparison.json).
- [Extended CAD editing soak](cad/bench/results/generation-long-soak-2026-09-27/memory-trend.json).
- [Live-preview architecture, review, and validation](docs/live-preview-ui-review.md).
- [CAD benchmark protocol](cad/bench/README.md), [frozen budgets](cad/bench/budgets.json), and [workbench limits](app/performance-baseline.json).

The current live reference contains five fresh visible Chromium sessions and
25 actions per scenario, using Chromium 153.0.8010.52, Ryzen 9 8945HS, Radeon
780M through ANGLE/OpenGL ES 3.2 and Mesa 26.2.3, and a 1280 × 720 viewport.
The values below are pooled action percentiles. Inputs are timestamped at DOM
blur, Undo click, or pointer release; completion is a frame-after-draw paint
opportunity for the exact revision, not physical display time.

| Scenario | Current p50 | Current p95 | Implication |
| --- | ---: | ---: | --- |
| Numeric case edit | 266 ms | 279 ms | Still above the proposed 200 ms target; attribute remaining construction costs |
| Undo | 93 ms | 93.8 ms | Preserve the existing cached-generation and display path |
| Gasket release | 386.4 ms | 417.6 ms | Prioritize exact bottom opening cuts and plate construction |
| Authored mount release | 54.6 ms | 71.8 ms | Preserve exact reuse, warm caches, and commit acknowledgement |

The earlier live before reference contains only one headless session. It cannot
establish a multi-session improvement against this hardware-accelerated reference.
Historical [pre-Phase 1 timings](app/performance-results/live-preview-perf-final-2026-09-27.json)
and [instrumented before/after results](docs/generation-performance.md) remain
available; they are not the current baseline. Keep software-rendered headless
runs and synthetic CPU throttling separate from hardware and real-device results.

The hardware-accelerated trace found no main-thread long tasks during normal
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
its two fixtures; full-UI session memory is a separate Phase 2 validation task.

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

## Phase 2 — Remaining exact-generation latency (pending)

The seven sections below correspond one-to-one with [TODO.md](TODO.md). Acceptance
and attribution come first, then exact construction improvements in measured
impact order, followed by combined validation and a conditional feedback decision.
Expected speedups are hypotheses until measured. Recovery and gallery work remain
separate follow-ups and do not block performance acceptance.

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

## Separately tracked follow-ups

| ID | Priority / status | Next action and acceptance |
| --- | --- | --- |
| F3 | P2 resilience; observed, cause unconfirmed | Reproduce the blank first Case navigation after replacing build assets across build activation and offline/online transitions. Stale service-worker/lazy-chunk mismatch remains a hypothesis. Fix the confirmed cause with recovery/asset consistency tests, preserving saved work and drafts and avoiding destructive forced reloads. |
| F2 | P3 gallery; scaling unmeasured | Benchmark opening, search, and scrolling with 10, 50, and 200 saved projects. Choose summary records, lazy previews, or virtualization only if measurements justify them. |

Neither follow-up is evidence of a CAD bottleneck or part of the current TODO.
Worker pools, kernel replacement, speculative warm-up, indexed transport, and
STEP-only export remain unselected options.

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

Use TODO.md as the durable Phase 2 checklist, with sections 2.1–2.7 mapping to
items 1–7. Its current state is 0 done, 0 blocked, 0 skipped, and 7 pending.
Record completed execution items in CHANGELOG.md when implemented and verified;
do not mark Phase 2 items done from similar-looking Phase 1 changes. Phase 1
history remains in this plan, the changelog, and the implementation report.
Documentation alignment does not count as performance implementation or new
benchmark evidence.
