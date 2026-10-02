# Generation and edit performance plan

## Current priority: frontend Dioxus migration

The latest user clarification makes this run 100% frontend: port all existing TSX, theming and interaction/responsive behavior. The [frontend v1 roadmap](docs/migration/DIOXUS-FRONTEND-V1.md), [spec](.scratch/dioxus-frontend-v1/spec.md) and [task graph](.scratch/dioxus-frontend-v1/PLAN.md) govern the continuation. F1 is implemented and verified as the first workbench shell/theme increment ([handoff](.scratch/dioxus-frontend-v1/evidence/handoff.md)); F2 is next, and remaining phases complete projects and every workspace. Backend engine/generator/CAD rewrite work is outside this active plan. Earlier session restrictions and plans below remain historical; current user authority and applicable constraints govern.

## Current run: finish milestone specifications and bound the first probe

Continue the approved six-capability map and accepted ADR 0003; do not reopen
settled architecture or replace this direction with a competing migration plan.
The 2026-10-01 request authorizes documentation, technical validation and a
read-only subagent preflight. It does not approve an undefined implementation
task set. The existing editor-session spec remains a review input.

Complete the other five module specs for the same representative milestone,
validate all six together, and retain runtime/compatibility questions as gates.
Use a fresh, task-owned review worktree at `96dd51d3`, with verified copies of
accepted inputs. Leave every pre-existing protected worktree untouched.

Extend this plan with only P1 worker/CAD packaging in [tasks/plan.md](tasks/plan.md),
revision `P1-r1`, and its bounded task set in [TODO.md](TODO.md). Present that
candidate as authorized in advance by the later user instruction. DOC-1
technical validation remains required before prototype execution. P2/P3 and
production adoption remain outside this bounded run. Record
preflight, exact document versions, candidate review and resumable handoff in
[the migration run record](docs/migration/RUN.md).

## Current session: editor-session specification

The user approved the [capability map](CAPABILITY-MAP.md) and
[ADR 0003](docs/adr/0003-rust-application-ownership.md) on 2026-10-01. Record
that Phase 0 decision without treating runtime probes or trial tickets as
complete. Retain the accepted Wayfinder inputs and assessment baseline.

Run spec-driven-development Phase 1 for only `editor-session`, bounded to the
representative layout/case milestone. Existing public engine and service
contracts are its providers; they do not need replacement specifications to
describe this consumer. Specify command and gesture ordering, accepted versus
preview state, durable-save recovery, job scope and export coordination in
[SPEC-editor-session.md](SPEC-editor-session.md). Keep CAD, scene semantics,
host adapters and Dioxus implementation with their approved owners.

Assume a headless Rust session whose transitions produce host effects, using
existing request/reply types. Surface any new interface or behavior choices
as specification proposals. Reuse the exact-version official-source evidence;
dependency resolution still does not establish packaging or runtime behavior.

This session writes documentation only. Do not create an application crate,
implement migration code, generate a migration backlog, activate gates,
change dependencies/visibility or promote prototype code. Track one scoped
specification deliverable, validate its sources and links, and stop for review
of this module's specification before implementation planning or tasks.

The bounded specification is prepared for Phase 1 review. Repository and
whitespace checks pass; the [documentation record](.scratch/dioxus-session-spec/scoped-document-checks.json)
validates the capability/spec/ADR links and retains the five inherited missing
performance reports in the full PLAN inventory. Production sources, constraints
and accepted Wayfinder artifacts remain unchanged. No prototype or application
implementation was added; integration probes and trial acceptance remain open.

## Completed scope review: architecture boundaries

Use spec-driven-development Phase 0 to propose a small capability map for
document/engine, editor session and interactions, rendering, host integration,
generators/CAD/export and Dioxus presentation. These are ownership boundaries;
justify packaging separately rather than assigning a crate to every row.
Reuse the accepted [Wayfinder decisions](.scratch/dioxus-browser-trial/map.md)
and [context assessment](docs/dioxus-context-baseline.md), including their
recorded failures and unverified integration questions.

Present the map and proposed resolutions of the first milestone's blocking
decisions before detailed rationale: full-Rust scope, platforms, state owners,
execution/transport, preview/cancellation, scenes, persistence/export,
remaining dependencies and transition. Apply API/interface design, versioned
official-source verification and the existing ADR convention. Give each
change its observed problem, alternatives, preserved/changed contracts and
validation evidence. Scope uncertain integration prototypes explicitly and
isolate any later execution; prototype results never authorize production use.

One throwaway dependency-resolution probe is in scope now: Dioxus/CLI candidate
0.7.10 with existing core and pinned browser bindings, in a standalone scratch
package. It contains no application implementation; its lockfile and verdict
stay isolated. Runtime worker/CAD/canvas/offline probes remain proposed charters
for review, not completed evidence.

Keep this session to a review proposal in [CAPABILITY-MAP.md](CAPABILITY-MAP.md)
and [proposed ADR 0003](docs/adr/0003-rust-application-ownership.md). Do not write
module specifications, implement migration code, generate a migration backlog,
activate gates, change dependencies/visibility or rewrite accepted decisions.
Track only this scope-review deliverable in TODO; leave trials and cutover
pending. Stop for human review of boundaries and blocking decisions, as required
by spec-driven-development's scope gate and the user's instruction.

The map and ADR were prepared, with official 0.7.10 source verification
and an isolated dependency-resolution verdict. Online resolution passes with
the existing binding pins and required mounted feature; offline resolution
fails against the incomplete cache. The user approved the architecture scope
on 2026-10-01. Runtime prototypes and trial acceptance remain pending; the next
bounded specification is the current session above. No accepted Wayfinder
ticket or production gate is marked complete by scope approval.

## Current session: migration constraint proposal

Update the existing [CONSTRAINTS.md](CONSTRAINTS.md), using the installed Addy
constraint-driven-development skill and the
[context assessment](docs/dioxus-context-baseline.md). The migration permits
architectural redesign of the full application, including existing Rust
internals, while preserving validated capabilities and accepted contracts.
Keep the Wayfinder decisions and the completed assessment as linked evidence.

Require each task to identify whether it preserves behavior, intentionally
changes behavior, or corrects a defect. Define characterization and approval
requirements, preserve the quality floor and protected work, and make temporary
adapters accountable for retirement. Map native, WASM/browser, boundary,
interaction, persistence/export and resource checks to existing executable
commands, separating development, completion and integration tiers. Reuse
measured budgets; record missing measurements and missing checks explicitly.

This session produces a proposed contract for review. Do not install or activate
new gates, change production code or dependencies, repair baseline failures,
update measurement baselines, or merge, push or deploy. Present the proposal
before any later gate activation; no architecture implementation is authorized.

The [proposal](CONSTRAINTS.md) is prepared and verified; [TODO.md](TODO.md) and
[CHANGELOG.md](CHANGELOG.md) record completion. Additional gate activation and
unresolved measurement/ownership decisions remain future work.

## Completed assessment: Rust architecture reconciliation

Assess the committed `dev` baseline `96dd51d3e790c28f5554a8c9888a147c8814e2a7`
on branch `docs/dioxus-context-baseline` in the new isolated worktree. Use the
installed Addy Osmani agent-skills package, starting with context-engineering,
for the full-Rust/Dioxus architecture effort. Preserve the
[Wayfinder map](.scratch/dioxus-browser-trial/map.md), tickets and research
unchanged as evidence; the user's current skill choice governs this effort.
Reuse [CONSTRAINTS.md](CONSTRAINTS.md). Record current ownership, reusable
decisions, validation provenance and the next recommended decision in
[the context baseline](docs/dioxus-context-baseline.md).

This session permits documentation and baseline checks only. Do not implement
the migration, change production code or dependencies, commit or rewrite Git
history, repair unrelated failures, update test/performance baselines, or
reset, clean, delete or recreate existing worktrees. Keep diagnostics from the
dirty source checkout separate from this committed baseline. Stop after
reporting risks and the next decision; the existing
[architecture/acceptance ticket](.scratch/dioxus-browser-trial/issues/03-trial-architecture-acceptance.md)
remains unresolved.

## Immediate feature: keymap workspace and located findings

Separate physical Keycaps settings from the Keymap workflow. Rebuild Keymap
inside the existing workbench design system with layer selection, key selection,
a searchable behavior/keycode editor, and dedicated macro and encoder controls.
Use clean-room observations of ZMK Studio and DYA Studio, preserving local
projects, stable key identities, Undo/Redo, and export snapshot ownership.

Rust owns a typed persisted keymap, validation, behavior semantics, and ZMK source
generation. Support key press, mod-tap, layer-tap, layer actions, transparent/none,
macros, and per-layer encoder rotation. Retain legacy bindings and safe modifier
expressions; reject unknown references and invalid source tokens. Hardware
encoder wiring remains owned by the existing electrical handoff. Local editing
and ZMK source export are the agreed scope; live keyboard connectivity is future
work. Keep an explicit documented path for remaining ZMK behavior families.

Finding locations must originate with their geometry diagnostics, survive board
and part wrappers, and drive all findings surfaces. Show affected corners/edges,
component envelopes, or mechanical features where known; retain a truthful
whole-object fallback for diagnostics without a meaningful geometric location.
Test through the agreed public Rust edit/firmware-export APIs and browser
workflows. Record regressions before fixes, regenerate contracts, run affected
build/checks and the final full suite, review standards and spec, and commit.


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

### Tracking scope
Track this request as three deliverables: keymap and Keycaps, located findings, and verification/review. Keep the existing backlog separate and do not expand this request into additional tasks.
