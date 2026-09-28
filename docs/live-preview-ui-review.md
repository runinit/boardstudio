# Live case preview and performance reconciliation

Updated 2026-09-27. Implementation is in the UI worktree at `0ca5e3f1`
plus uncommitted changes. Functional and performance verification are recorded
separately below; historical CAD results are not results for this build.

## Sources and decisions

The original review compared this worktree with the live-case architecture
proposal in the development checkout (`30d2eb25`) and the completed CAD review
in worktree `3e8a` (`0ca5e3f1` plus uncommitted changes). The accepted CAD
foundation is now integrated: safe Boolean batching, bounded pre-opening stage
reuse, body previews, and stage/copy/transfer metrics. The public combined-mesh
adapter remains available and computes the combined mesh only when needed.

STEP-only export, worker pools, indexed transport, progressive OCCT tessellation,
and speculative warm-up remain unselected. STEP **model import** remains covered
by the benchmark and production API. Pre-opening reuse includes mounts; it must
not be presented as proof of a pre-mount cache benefit.

The existing 44px header, project gallery, navigation, and editing returns are
preserved. Gallery scaling is a separate open question: listing every complete
saved document and rendering every preview needs measurement with 10, 50, and
200 projects before introducing summary records or virtualized rendering.

## Implemented behavior

- Live preview starts enabled in the active, eligible Case workspace. Numeric
  changes and gestures refine automatically. Pause retains the last usable
  geometry; one Update preview action supports manual work and recovery.
- One active request and one replaceable pending request bound CAD work.
  Superseding an edit preserves the warm worker. Explicit Cancel pauses live
  updates and terminates the generation worker. Export uses a separate client;
  model imports retain their own existing worker ownership.
- Every request captures document, session, board, instance, revision, scene,
  and mechanical context. Drafts also have a separate sequence. Results from a
  replaced request or old context cannot become current or paint as a new result.
- Draft meshes remain presentation-only. They never enter the committed preview
  cache or become export-ready. Last usable committed geometry can stay visible
  while updating, paused, blocked, or failed; the UI identifies retained geometry.
- Gasket and case mount handles update retained transforms, with picking in the
  transformed coordinate space. Pointer work is coalesced to animation frames.
  Valid movement submits drafts; release commits once; Escape cancels. Gasket
  links move together. Case mount edits do not alter PCB mounting holes.
- CAD preview replies send changed bodies and ordered body IDs. The client keeps
  unchanged arrays. Scene preparation receives changed case bodies and removals;
  unchanged PCB/model data stays out of that path. Renderer patches preserve
  camera, section, visibility groups, theme, and exploded transforms.
- Preview controls remain above the canvas when the inspector is closed,
  including mobile. Progress stays outside layout flow so refinement cannot
  shift the canvas during a drag. Fit uses visible retained geometry, and mount
  handles remain reachable through the PCB. Manufacturing export requires current committed
  geometry and successful mechanical validation.

Key implementation: [scheduler](../app/src/livePreviewScheduler.ts),
[generation](../app/src/useCaseGeneration.ts), [CAD client](../app/src/CaseClient.ts),
[gestures](../app/src/ui/AssemblyScene.tsx), [scene client](../app/src/renderClient.ts),
[renderer](../renderer/src/wasm.rs), [export readiness](../app/src/ui/caseReadiness.ts).

## Measurement policy

No specific Chrome version is pinned. Runs record the actual Chromium version,
Node, CPU, viewport, sources, and built artifacts where applicable. Existing
workbench CPU and latency limits remain unchanged. CAD comparison retains its
CPU/OS/core/renderer identity requirements, and reports browser-version differences
as provenance. A changed renderer needs a reconciled reference before a historical
CAD comparison can establish acceptance; this is not permission to relax budgets.

`pnpm test:perf` builds release artifacts, runs five workbench sessions, then
pointer, matrix, outline, and live-preview tests. The live report is a diagnostic
with numeric-edit, Undo, gasket-release, and mount-release distributions, retained-handle upload
counts, and body-patch metrics. Five actions per scenario are not a frozen baseline.
`BOARDSTUDIO_PERF_REPORT` selects the combined JSON report destination.

The separate [CAD harness](../cad/bench/README.md) uses the real core, CAD worker,
scene worker, and renderer. It excludes React/app-shell overhead, so it cannot
substitute for a real gesture trace. Its historical baseline retains original
provenance. New measurements use fresh output directories and a quiet host.

| Proposed target | Interpretation |
| --- | --- |
| Pointer to hardware pose p95 ≤33 ms | Current instrumentation ends at render submission; not physical display latency |
| Main-thread incremental work p95 ≤4 ms | Pointer callback work, reported separately from CAD and frame pacing |
| Release to exact geometry p95 ≤200 ms | Measure real mount/gasket releases, not unrelated opening edits |
| Queue: one active + one pending | Functional invariant, independently tested |
| Unchanged geometry: zero rebuilt/transferred/uploaded | Verify CAD deltas and scene patches independently |
| Bounded long-session memory | Needs a soak measurement; cache bounds alone do not prove it |

The frame after a successful draw is a paint-opportunity proxy. It does not measure
GPU completion or physical display latency. If exact CAD refinement misses its
target, retained handles remain responsive. A display-only polygon mesh is a
possible follow-up, requiring authoritative inputs, error bounds, and fallback
coverage before adoption; it must remain outside export.

The current live diagnostic timestamps the actual DOM blur, Undo click, and
pointer release. Automation dispatch time is excluded. Per-action attribution
includes core request/reply timing, kernel stages, scene preparation/upload, and
long tasks. `BOARDSTUDIO_LIVE_CPU_PROFILE=/tmp/live-preview.cpuprofile` runs one
numeric edit and Undo with a Chrome CPU profile and accompanying `.timeline.json`;
that profiling mode is separate from the normal five-action diagnostic.

## Remaining performance work

1. Capture a full five-session CAD reference with the current core/renderer
   identity and frozen fixture hashes, then compare candidates using the existing
   budgets and minimum sample counts. Preserve the historical reference.
2. Reduce the remaining exact-refinement delay against the proposed 200 ms
   target. The real release trace separates compositor waits from kernel work;
   gasket edits still rebuild several affected bodies, and draft-to-commit
   transitions can restore committed geometry before displaying the new result.
   Verify exact cache-key reuse and commit/cancel behavior before changing that
   handoff. Pre-opening cache results do not establish reusable pre-mount geometry.
3. Run a long editing soak covering project/board switches, drafts, cancellation,
   and export. Measure worker lifetime and memory growth before claiming bounded
   session memory. Measure large saved-project galleries separately.

## Verification

The application unit suite passed 375 tests across 64 files. Focused tests verify latest-request settlement,
failed/superseded body patches, stale contexts, draft export exclusion, warm-worker
supersession, and mount edge clearance. Final separate checks passed: 25 native
renderer tests, 8 native CAD tests (3 diagnostic tests remain ignored), and 30 CAD
adapter/fixture/benchmark tests. Repository hygiene, generated contracts, runtime
imports, and native/WASM boundary parity (13 core and 9 archive requests) passed.
The CAD rebuild produced the same WASM hash used by the browser measurements.

The final focused browser run passed all 15 tests across assembly preview, live
preview, authored mounts, and manual generation. It covers real gasket/mount
drafts, one commit on release, Escape, Undo, pause/cancel, retained scene updates,
mobile controls, stable canvas bounds during refinement, and PCB pixels with
hidden case geometry. Other targeted case-workbench suites passed earlier in
the task; the complete repository browser/pages suites were not rerun.

The release build passed. The five-session workbench gate and pointer, matrix,
and outline checks passed on Ryzen 9 8945HS, Chromium 153.0.8010.52, Node 26.10.0,
1280 × 720. Existing budgets were unchanged. The executed command was
`BOARDSTUDIO_CHROMIUM=/usr/bin/chromium pnpm --dir app test:perf`, following the
release renderer and application builds.

| Workbench scenario | Median session request-to-reply p95 | Median session paint-opportunity p95 |
| --- | ---: | ---: |
| 100 keys, single | 3.0 ms | 33.5 ms |
| 100 keys, row | 3.1 ms | 33.5 ms |
| 200 keys, single | 4.5 ms | 33.5 ms |
| 200 keys, row | 4.6 ms | 34.0 ms |

The real live-preview diagnostic used five actions per scenario:

| Action | Action/release to paint opportunity p95 |
| --- | ---: |
| Numeric edit | 1112.4 ms |
| Undo | 816.7 ms |
| Gasket release | 1265.7 ms |
| Authored mount release | 699.4 ms |

The raw report names the distributions `actionToWorkerReply` and
`previewStartToPaint`: the former starts at the action timestamp and ends when
the preview worker replies; the latter is the paint-opportunity measure's own
duration from preview start to paint. `releaseToPaint` remains the action/release
to paint-opportunity timing.

**Exact refinement did not meet the proposed 200 ms target.** During the gasket
sequence, pointer callback work was 0.4 ms p95 and render submission was 2.2 ms
p95. All 75 retained-handle updates uploaded zero handle meshes; five body patches
excluded unchanged board/model geometry. These timings do not measure physical
display latency. The performance runner's success means the existing gates passed;
the new exact-refinement target remains diagnostic and failed.

The [raw workbench/live report](../app/performance-results/live-preview-2026-09-27.json)
preserves all sessions, action samples, and provenance. Full historical CAD
acceptance and long-session memory validation remain separate gates.

The CAD smoke run completed all 30 fixture/scenario combinations with one sample
each, including cold and warm STEP model import. Every sample had request metrics
and a paint opportunity. Its [summary](../cad/bench/results/live-preview-smoke-2026-09-27/summary.json)
and [comparison](../cad/bench/results/live-preview-smoke-2026-09-27/comparison.json)
are retained with compressed raw samples. Comparison exited nonzero as expected:
fixture hashes and scenario coverage match, but sampling is insufficient and the
renderer identity differs from the historical reference. This verifies the harness,
not CAD latency/RSS acceptance.


## Review fixes verified 2026-09-27

The three follow-up review findings are addressed in the uncommitted UI worktree:

- Mount gestures capture the current prepared case body's offset regions, holes,
  and intersecting openings. PCB contours and drilled holes no longer stand in
  for case material. Prepared metadata travels with the completed preview without
  evaluating the lazy combined mesh. Drafts cannot replace gesture boundaries;
  changing physical inputs requires a current preview before starting another
  mount gesture. Tray cavities retain their supporting base material.
- CAD and renderer measures are opt-in through `?cadMetrics=1`. Each fixed metric
  name retains at most 512 entries, clearing its previous batch when full while
  continuing to record new samples. External `performance.clearMeasures()` calls
  remain supported; a permanent lifetime counter would break the CAD harness.
- Live diagnostic fields now name their measured intervals:
  `actionToWorkerReplyMs` and `previewStartToPaintMs`. Archived field names were
  corrected without changing measurements; neither interval claims exclusive
  worker/CAD execution time.

Regression tests first reproduced rejected valid offset positions and accepted
invalid hole/opening positions. After the fixes, 383 app tests across 65 files,
16 focused browser tests, and the application production build passed. Browser
coverage includes dragging a mount beyond the PCB into a 5 mm expanded case,
draft refinement, Escape, exactly one commit, and Undo without PCB changes.
Repository hygiene, contracts, runtime imports, and boundary parity passed.
Native renderer/CAD suites and the full repository browser suite were not rerun
for these application-only fixes.

The isolated live diagnostic passed its collection and retained-geometry checks;
its [fresh report](../app/performance-results/live-preview-review-fixes-2026-09-27.json)
keeps the earlier report intact. On the recorded Chromium 153.0.8010.52 / Ryzen 9
8945HS environment, pointer callback work was 0.4 ms p95 and render submission
was 2.9 ms p95. All 75 handle updates uploaded zero meshes; five body patches
retained unchanged board/model geometry. Mount and gasket release-to-paint
opportunity p95 were 904.8 ms and 1275.5 ms respectively. The proposed 200 ms
exact-refinement target remains unmet; these five-action diagnostics do not
establish a statistically significant performance change or long-session memory
acceptance. The five-session workbench gate was not repeated for this follow-up.

## Performance follow-up — 2026-09-27

The release trace exposed repeated WebGL compositor work. Chrome recorded
roughly 150 ms `Commit`/`UpdateLayer` waits with only a few milliseconds of thread
CPU time; those long tasks must not be presented as equivalent React or CAD CPU
cost. The CPU profile also showed repeated Ergogen parsing during committed
document updates.

The application now avoids renderer state updates for generation-status-only
changes, skips draws for identical scene geometry, and skips unchanged handle
submissions. A pending camera or geometry draw still runs when an identical
scene update follows it. The bundled parts catalogue is generated once per
workbench mount; project definitions continue to override it on each update.
Geometry tolerances, export readiness, cache budgets, and interaction limits
are unchanged.

Regression tests reproduced redundant renderer updates and catalogue generation
before the fixes. The application suite passed 386 tests across 66 files, and
23 focused browser tests passed for assembly views, live numeric/gasket/mount
editing, cancellation, Undo, manual generation, and responsive Parts workflows.
The production build, repository checks, contracts, runtime imports, and boundary
parity passed. Impeccable's detector reported no findings on the edited UI
components. Native renderer and CAD code did not change in this follow-up.

The full application performance runner passed its five-session workbench gate
and matrix, outline, and pointer checks without budget changes. Median session
p95 painted times were 33.5/33.5/33.4/34.1 ms for 100-key single/row and 200-key
single/row edits. The final live diagnostic also passed collection and retained
geometry assertions. Comparison with the fresh, equivalently instrumented
[before report](../app/performance-results/live-preview-perf-before-2026-09-27.json):

| Action | Before p50 / p95 | After p50 / p95 |
| --- | --- | --- |
| Numeric edit | 1011.5 / 1098.9 ms | 589.4 / 736.8 ms |
| Undo | 780.4 / 930.3 ms | 427.5 / 463.2 ms |
| Gasket release | 1140.3 / 1256.3 ms | 831.1 / 874.8 ms |
| Mount release | 576.8 / 618.0 ms | 524.9 / 530.0 ms |

The [final raw report](../app/performance-results/live-preview-perf-final-2026-09-27.json)
preserves five workbench sessions and the live samples on Chromium 153.0.8010.52,
Node 26.10.0, Ryzen 9 8945HS, and a 1280×720 viewport. These live comparisons
contain five actions each, not five independent sessions or statistical acceptance
of a new CAD baseline. Earlier reports include automation-dispatch overhead and
must not be compared directly with the corrected DOM-event timestamps.

Pointer callback work remained 0.4 ms p95; render submission was 2.1 ms p95.
All 70 recorded handle updates uploaded zero meshes, and five body patches
retained unchanged board/model geometry. Exact refinement still misses 200 ms:
final gasket kernel execution took 416–455 ms, while mount kernels hit their
cache and the remaining delay was in the application/scene handoff and browser
compositing. Long-session memory and full historical CAD acceptance remain
unverified; passing the existing performance runner does not waive either gate.
