# Generation optimization experiments

> Historical research: measurements, code paths and integration status below
> describe the recorded revision. For current ownership and commands, use
> [architecture](../../architecture.md) and the [development guide](../../../README.md).

Historical raw artifacts and experimental sources now live in the performance
worktree. See Development worktrees (historical record in Git at `323967ff`) for their location.

Planned 2026-09-28 against `49d33a91`. This is Phase 3 of PLAN.md (historical record in Git at `323967ff`).
It consolidates the gasket, parametric construction, Cadrum/OCCT, and alternative
kernel discussions. The selected bounded E0/E1/E7, Manifold-preview, and
Monstertruck screening has now been executed; see [results and revised
priorities](gasket-comparison-results.md). Other experiments below remain
proposals. No candidate has been integrated into production.

## Objective and design boundary

Reduce actual generation and edit latency with the existing case design and
production Cadrum/OCCT backend. Optimize how the same parts are computed.

Preserve support count and placement, explicit mirror linkage, plate tabs,
bottom/retainer lobes, stepped cavities, closure screw positions, captive-nut
pockets, foam pads, wall/floor thicknesses, clearances, and material boundaries.
Prepared geometry and saved project semantics remain the source of truth.
Alternative construction may change internal face partitioning or triangle
ordering, but must preserve the represented solid, part identity, and placement.

Do not introduce fixed closure hardware, continuous support ledges, new tab
arrangements, simplified parts, or altered gasket mechanics. Those were design
alternatives in the discussion, not selections for this phase. Do not loosen
Boolean, profile, or mesh tolerances to obtain a faster result. Existing mesh
deflection remains absolute 0.1 mm linear and 0.5 rad angular.

Keep exact solids and export validation in the production generation path.
Deferring solid construction until export or showing provisional geometry may
improve perceived responsiveness, but cannot count as faster exact generation.
The user selected bounded Manifold-preview and Monstertruck comparisons in
addition to the Cadrum experiments. These are isolated feasibility measurements;
production kernel migrations, Truck, and Brepkit remain deferred.

## Completed bounded screening

**The E0–E9 plan is not complete.** The comparisons below are one subset.
The coverage table (historical record in Git at `323967ff`) tracks every remaining
item, and [round-two OCCT screening](occt-optimization-round2.md) continues
history, Boolean selection/planning, non-destructive, and cleanup experiments.

The [comparison report](gasket-comparison-results.md) records five browser
sessions across four unchanged bottom fixtures, native and independent STEP
validation, stage timings, failed attempts, and reproducible isolated builds.
Unused mesh output had no useful speed effect. The height-band Cadrum prototype
was about 40% slower. Manifold preview was about 4.5× faster for this bottom;
Monstertruck failed or timed out before completing its first circular hole.

Prioritize equivalent Boolean operation selection and eligible analytic holes
in extrusion profiles for the exact path. Separately test the Manifold preview
integration boundary before considering adoption. E2 history suppression and E8
copy removal move below Boolean work; the small measured bridge costs do not
establish that internal OCCT history is free. The original E0–E9 ordering below
is retained as the broader experiment inventory, not a record of completed work.

## Starting evidence

The final Phase 2 build passes all 30 frozen CAD scenarios, live regression and
incremental-improvement checks, interaction gates, and the full-UI memory soak.
The separate gasket target of 200 ms p95 remains unmet.

| Measurement | Final Phase 2 result | Interpretation |
| --- | ---: | --- |
| Gasket release to exact paint opportunity, p95 | 370.0 ms | Primary end-to-end target |
| Numeric edit, p95 | 171.0 ms | Preserve the gain while improving generation |
| Undo / authored mount release, p95 | 91.3 / 59.3 ms | Cached paths must not regress |
| Gasket CAD execution, median | 239.5 ms | Most remaining work is in construction/meshing |
| Opening cuts / base extrusion, median | 88.3 / 54.1 ms | Prioritize bottom operations and repeated construction |
| Tessellation / mount-hole cuts, median | 54.7 / 35.4 ms | Split hidden costs before selecting changes |
| Renderer preparation / upload, median | 3.1 / 1.7 ms | No evidence for another rendering-quality reduction |

Stage medians are not additive end-to-end percentiles. `tessellation` currently
includes Cadrum mesh extraction and edge discretization, not just OCCT meshing.
Seven bodies genuinely change during the measured linked gasket move; whole-body
reuse cannot pretend their inputs are unchanged. The earlier 337.6 ms gasket run
is retained history, not a replacement for the final 370 ms result.

Evidence:

- [Latest report and validation](generation-performance.md#latest-phase-2-status).
- Final live summary (`app/performance-results/phase2-scene-live/summary.json` in the performance worktree)
  and comparison (`app/performance-results/phase2-scene-live/comparison.json` in the performance worktree).
- Retained stage attribution (`app/performance-results/phase2-scene-live/gasket-attribution-comparison.json` in the performance worktree).
- Final CAD summary (`cad/bench/results/phase2-scene-final/summary.json` in the performance worktree)
  and comparison (`cad/bench/results/phase2-scene-final/comparison.json` in the performance worktree).
- Frozen CAD protocol (retired React benchmark; retained in Git at `323967ff`) and budgets (retired React benchmark; retained in Git at `323967ff`).

Already completed: batched deferred Boolean expressions, exact reduction of
overlapping rectangular opening cutters, immutable final-solid sharing, bounded
cache promotion, removal of unnecessary pre-opening copies, a guarded rectangular
plate mesher, latest-only scheduling, and identical-scene reuse. New experiments
must demonstrate additional benefit beyond those changes.

## Experiment order

Run one variant at a time against its control. E0 is required before optimization.
The table orders the remaining experiments; later rows are conditional on the
remaining measured cost, rather than a commitment to implement every idea.

| ID | Priority | Experiment | Main cost / risk |
| --- | --- | --- | --- |
| E0 | P0 | Freeze reproducible controls, equivalence fixtures, and detailed timings | Prevent invalid comparisons and locate real costs |
| E1 | P1 | Omit unused mesh edges and face IDs | Removes known discarded work; savings unmeasured |
| E2 | P1 | Omit unused Boolean provenance work | Separate application history from kernel bookkeeping |
| E3 | P1 | Test OCCT OBB and operation selection | Intersection cost; option overhead may outweigh savings |
| E4 | P1 | Extend the exact plate-meshing fast path to eligible tabbed outlines | Avoid kernel meshing while retaining exact solids |
| E5 | P2 | Improve Boolean batching, grouping, and cutter preparation | Reduce intersections without changing subtraction semantics |
| E6 | P2 | Add narrowly scoped parametric feature reuse | Avoid repeated construction; correctness and memory risk |
| E7 | P2 | Build the identical stepped bottom from profiles | Potentially larger gain; highest geometry-validation effort |
| E8 | P3 | Test non-destructive processing and safe copy reduction | Ownership risk; proceed only if copies remain material |
| E9 | P3 | Conditional topology cleanup, build tuning, and threading feasibility | Secondary costs and deployment complexity |

### E0 — Reproducible controls and attribution

Use committed `49d33a91` as the source control and the final Phase 2 artifacts as
historical acceptance evidence. Pin Cadrum `=0.8.20`, OCCT `8.0.1 rev2`, the
container/toolchain, and generated artifacts. The actual builder uses
wasm-bindgen 0.2.129; do not infer its version from an older assessment paragraph.

Before patching Cadrum, establish which source the container compiles. The local
Cargo registry is useful for inspection but is not proof of the container's
contents. Use a reproducible pinned source patch/fork and locked dependency path;
do not modify the global Cargo registry. Verify an unmodified build through that
path first. Record source, patch, OCCT archive, container, JS glue, core, CAD, and
renderer hashes. Keep runtime contracts and existing API visibility unchanged;
use private diagnostic/build switches for experiments.

Capture representative prepared inputs for unchanged/cached generation, a linked
gasket move, an unlinked move, numeric dimensions, cold generation, the existing
six-edit burst, Undo, and export. Include actual rotated/concave/disconnected
cases and contact/near-contact pockets. Add stress fixtures separately without
altering frozen fixtures or calling them representative of normal usage.

Split instrumentation into non-overlapping sub-stages:

- Primitive/profile creation and tool construction, by body and operation.
- OCCT intersection preparation/evaluation, cell selection, and internal-boundary
  removal; distinguish wrapper `Perform()` timing from a deeper intersection-only
  timer if the latter needs an OCCT diagnostic build.
- History collection/relay, result copy, and cache-boundary copies.
- Surface triangulation, normals, vertex/index/face-ID extraction, edge sampling,
  Rust conversion/expansion, transfer, and renderer preparation/upload.

Keep parent timings for continuity but do not double-count them. Record operand,
face/edge, Boolean, copy, triangle, cache hit/miss/eviction, and warning counts.
Measure opt-in instrumentation overhead against an uninstrumented control;
acceptance timings must use comparable instrumentation settings. Diagnostics
must remain bounded and must not flood the UI with per-face messages.

Deliver a ranked cost report and runnable control/candidate harness before E1.

### E1 — Mesh only what the bridge consumes

Inspection of Cadrum's `src/occt/io.rs` shows unconditional topological-edge
discretization. Its C++ `mesh_shape` also emits face IDs. Our
[`mesh_to_data`](../../../cad/wasm/src/model.rs) consumes positions, normals, and indices
and discards both outputs.

Test edge sampling off first, then face-ID collection off, then both together.
Keep triangulation, normal calculation, winding, transforms, and display quality
unchanged. Cover imported STEP models as well as generated cases; retain the
existing full-output path wherever a caller needs those fields. Verify no
hidden picking/export dependency before adoption. If conversion/expansion still
costs materially, test fewer intermediate buffers separately while preserving
the existing non-indexed transport contract. Indexed renderer transport is deferred.

### E2 — Avoid unused face provenance

Our bridge does not call `iter_history`. Cadrum's `builder_cells` currently
collects builder history, relays it across a deep copy, and returns a face map.

First skip only the unused wrapper relay maps, preserving the solid copy and
kernel behavior. Then separately investigate the inherited
`SetToFillHistory(false)` option. Verify how CellsBuilder boundary unification
uses its internal modification maps; an unused application history API does not
mean all kernel history bookkeeping is removable. Preserve color/history behavior
for any other path that needs it. Reject any variant that compromises topology,
subsequent operations, or export.

### E3 — Tune OCCT with equivalent operations

Test `SetUseOBB(true)` against the current setting on axis-aligned and rotated
fixtures. Record bounding-volume overhead as well as total Boolean time. Retain
the default where a proven workload class receives no benefit.

For pure subtraction groups, compare the current CellsBuilder expression with
a dedicated OCCT multi-tool cut, if the pinned API supports the equivalent
operation. Preserve batching: do not replace one batch with a loop of binary
cuts. Keep unchanged result-copy policy during this comparison. The generalized
expression path remains available for mixed operations.

Do not enable gluing globally. OCCT requires genuinely coincident configurations
without the real intersections its glue mode skips; ordinary pocket cutters do
not meet that assumption. Test a glue variant only if a specific operation has
a cheap, demonstrated eligibility condition. Keep fuzzy tolerance unchanged and
retain validity/error checks. A failed or inapplicable experiment is a valid result.

### E4 — Exact meshes for eligible tabbed plates

The current bounded grid mesher rejects non-rectangular outer contours. First
measure which real gasket plates are eligible for an extension to simple
orthogonal concave/tabbed outlines; do not reshape an outline to make it eligible.

Extend material classification to the actual outer boundary and holes, retaining
the existing size bounds and automatic kernel fallback for unsupported geometry.
Test tabs, notches, winding, hole contact, collapsed coordinates, rotated edges,
and disconnected regions. Preserve exact CAD-solid construction and export.
If most costly plates are ineligible, record that result before proposing a
larger triangulator. A curved/general-profile extension needs its own error bound
and must not silently approximate analytic geometry for export.

### E5 — Equivalent Boolean planning and cutter preparation

Starting from the already reduced opening cutters, compare:

- Existing mount-hole and opening groups versus one combined subtraction where
  their dependency/order permits it. Fewer Boolean calls can still create a
  larger and slower intersection problem; measure both effects.
- Conservative spatial grouping/culling and stable operation ordering. Never
  cull a touching or near-touching tool with an unsafe bounds test.
- Building through-holes into an extruded profile instead of subtracting them,
  only where exact geometry and supported profile primitives allow it.
- Reusing primitive tool definitions/transforms instead of rebuilding identical
  cylinders and prisms; include any isolation copies in the measured cost.

Keep boss unions, openings, and overlapping-depth semantics correct. End with
closed solids and the same material boundaries. Internal-boundary removal cannot
simply be skipped because it is expensive. Unsupported cases use the current path.

### E6 — Parametric reuse without changing mechanics

Use stable feature IDs and complete geometry dependencies to isolate fixed work
from changed work. Start with a measured repeated feature, not a general CAD
feature-tree rewrite. Candidates include the unchanged central plate/key pattern,
base cavity components, tool definitions, and pad-local geometry with placement.
Pads are already cheap; prioritize them only if new evidence changes that ranking.

Support movement still changes tabs, lobes, closure holes, and pocket geometry.
Reusing their fixed inputs does not avoid the final integration cost automatically.
Measure fixed construction, integration, meshing, cache lookup, and copies together.
Prefer reusing immutable input data before sharing mutable kernel topology.

Keys must include every relevant contour, hole, depth, height, transform, tolerance,
and algorithm version. Test each dependency independently, Undo/return-to-original,
eviction, export, and context changes. Any new cache must fit an explicit allocation
within existing retention budgets, not silently add an unbounded cache.

Reuse a prepared `PaveFiller`/CellsBuilder only for identical arguments, options,
and valid lifetimes when selecting several results from the same intersection.
It is not an incremental solver for a moved cutter. First demonstrate a repeated
eligible workload that the existing final-result cache does not already handle.
Otherwise defer this sub-experiment.

### E7 — Construct the same stepped bottom from profiles

Prototype one current gasket-bottom fixture using its existing profiles and
feature depths. Represent constant-section height bands, create the exposed
faces, and construct/sew the equivalent Cadrum/OCCT solid, or compare equivalent
profile extrusions if face assembly proves unsuitable. Do not replace the design
with a continuous ledge or move its fasteners.

Start with the exact planar subset; preserve circular holes as analytic geometry
and use the current path for unsupported surfaces/openings. Reject a method that
only produces a visually similar mesh. Shared boundaries, internal faces, sewing
tolerance, closed-shell validity, dimensions, and independent STEP round trips
are explicit gates. Measure profile processing, construction/sewing, validation,
and tessellation together; do not move expensive work outside the measured path.

This is an alternative construction algorithm, not a new case design. Stop if
equivalence is not established or construction complexity consumes the savings.

### E8 — Non-destructive operations and ownership

Proceed only if E0 still finds meaningful copy cost. Compare
`SetNonDestructive(true)` while retaining existing copies first. It may add cost;
it is not a speed flag. With an external filler, its non-destructive setting is
the one that applies.

Consider removing a particular input/cache copy only after its ownership and
mutation contract is demonstrated. Cadrum's result-copy comments explicitly cite
a previous heap-corruption issue from shared geometry lifetimes. Non-destructive
mode alone does not prove that removing that result copy is safe. Keep the result
copy unless separate lifetime, drop-order, cache-eviction, repeated-edit,
mesh-after-Boolean, export, and native/WASM tests establish the alternative.
Preserve export/import isolation. Prefer a no-change decision over an unproven
ownership shortcut.

### E9 — Conditional secondary experiments

Select only a variant justified by the remaining profile:

- **Topology cleanup:** compare one final `clean()`/same-domain unification when
  excessive coplanar faces dominate downstream meshing. Count cleanup time and
  validate analytic surfaces, part identities, and geometry; do not clean after
  every operation by default.
- **Build tuning:** audit existing release/LTO/wasm-opt settings, then compare
  isolated settings with pinned toolchains and reproducible archives. Do not use
  fast-math or reduced precision. Record size, startup/compilation, warm latency,
  and memory together; changing Rust flags does not rebuild precompiled OCCT.
- **Parallelism feasibility:** verify the actual OCCT archive's thread support,
  WASM shared-memory/toolchain requirements, and deployment/browser compatibility
  before an implementation. The current mesher explicitly requests serial work;
  merely setting a flag cannot establish working WASM parallelism. Native results
  are diagnostic only. Defer browser threading if it requires an incompatible
  hosting change; do not introduce a worker pool or enlarge caches as a shortcut.

## Correctness and acceptance

For each variant, define the affected invariant before implementation. Bug fixes
need a regression that fails on the control for the expected reason. Geometry
equivalence tests should pass on the control; the performance harness establishes
its measured deficit. Do not fabricate a functional failure for an optimization.

Use existing native equivalence tests and CAD integration fixtures, extending them
where the experiment changes a boundary. Check solid validity/closure, solid and
part counts, bounds, positive volume, dimensions, holes, pocket depths, and
material occupancy. Compare both directions of material difference where reliable;
equal volume alone is insufficient. Reimport STEP through the independent
development reader as well as the production path. Preserve existing numeric
thresholds and require finite meshes, correct normals/winding, seam continuity,
and bounded tessellation deviation. Byte-identical STEP or triangle order is not
required when topology partitioning legitimately changes.

Run native tests, real WASM integration, and affected app/browser regressions
before performance acceptance. Preserve one-step Undo, linked/unlinked movement,
Escape, invalid moves, save failure, draft/committed isolation, manual/pause modes,
latest-only scheduling, context switches, and rejection of stale replies. A faster
successful case cannot conceal a new failure or fallback on another fixture.

### Measurement and promotion rules

1. Screening uses alternating control/candidate release builds on a quiet host,
   one factor at a time. Use at least five pairs for a finalist diagnostic, retain
   all samples/order/host observations, and report per-session distributions.
   Instrumented, busy-host, native-only, or undersampled reports are diagnostic.
2. Do not select winners from one minimum or aggregate stage medians into a p95.
   Report absolute and relative changes in stage time, worker time, and exact
   paint opportunity separately. Pause timing while builds/tests run. Preserve
   failed reports; investigate tail variation rather than rerunning until green.
3. Promote a candidate only after correctness passes and a comparable CAD run
   demonstrates at least 10% targeted latency improvement or 25% targeted
   copy/transfer reduction, with every frozen completion/paint/RSS budget passing.
   A reduction in one internal stage is screening evidence, not this acceptance.
   The copy criterion must not be presented as a user-visible latency gain.
4. Final combined validation requires five fresh hardware-accelerated live
   sessions with the existing five actions per warm scenario per session, and
   the full CAD protocol (at least the existing five sessions and 20 warm samples
   per scenario per session, preserving other required coverage). Compare
   incremental gains against the final Phase 2 control,
   not the slower Phase 1 reference. Retain all frozen absolute budgets and live
   regression limits. Report whether numeric or gasket p95 improves by at least
   10%; do not imply improvements add when combining accepted variants.
5. Keep the proposed 200 ms exact p95 target separate. Preserve pointer p95
   <= 4 ms and submission p95 <= 33 ms. Report cold startup and the six-edit burst
   separately using the established protocol; five cold samples are descriptive.
6. Run the established 256-cycle full-UI soak, including exports, cancellations,
   Undo, and project/board switches. Retain its JS/WASM growth limits and warmup.
   Re-run the CAD editing soak for new cache/ownership behavior. Report process
   RSS separately; synthetic 4x CPU coverage is not a physical mobile-device test.

If host/browser provenance is incompatible with historical reports, retain those
reports and obtain a fresh paired control from `49d33a91` on the new environment.
Do not overwrite references, reduce sample requirements, or relax budgets.

Relevant existing production-integration commands (not run for isolated screening):

```sh
pnpm --dir cad test
pnpm --dir app test
pnpm run build
pnpm run check:repo
pnpm run check:contracts
pnpm run test:contracts
pnpm run check:boundaries
pnpm run bench:cad -- --output cad/bench/results/phase3-<experiment>-<variant>
node cad/bench/compare.mjs cad/bench/results/phase3-<experiment>-<variant> cad/bench/results/phase2-scene-final
node app/scripts/run-live-performance.mjs app/performance-results/phase3-<experiment>-<variant> app/performance-results/phase2-scene-live/summary.json
pnpm --dir app test:perf
```

Replace placeholders with fresh paths; these illustrative commands are not a
shell script. Run relevant browser tests with the configured Chromium, and native
core/renderer tests when affected. Run Pages/offline deployment checks if assets,
toolchain features, or worker initialization change. At final integration use the
repository's complete `pnpm run check` gate, separately from timing runs.

## Deliverables and decision points

- **First checkpoint:** E0 report, reproducible patch/build path, and E1/E2
  results. Stop or reprioritize if the suspected hidden costs are negligible.
- **Second checkpoint:** E3/E4 results and updated cost ranking. Select E5–E8
  based on remaining cost and eligibility, recording deferrals with evidence.
- **Final checkpoint:** a minimal combined patch containing accepted changes,
  complete correctness/performance/memory results, and updated unmet targets.

Record each experiment's hypothesis, changed files/options, control/candidate
hashes, eligibility/fallback rates, correctness result, latency/memory results,
and adopt/reject/defer decision in a new Phase 3 section of the performance report.
Keep raw artifacts in fresh experiment directories. Do not mark an experiment
successful merely because it was implemented or because a microbenchmark improved.

## Implementation and API references

- [Construction](../../../cad/wasm/src/model/construction.rs),
  [existing equivalence tests](../../../cad/wasm/src/model/construction/equivalence.rs),
  [cache ownership](../../../cad/wasm/src/model/construction/cache.rs),
  [planar mesher](../../../cad/wasm/src/model/construction/planar_mesh.rs),
  [metrics](../../../cad/wasm/src/model/metrics.rs), and [mesh bridge](../../../cad/wasm/src/model.rs).
- [Pinned dependency](../../../cad/wasm/Cargo.toml),
  [container](../../../cad/wasm/Containerfile), [build script](../../../cad/scripts/build-cadrum-wasm.py),
  and [verified OCCT archives](../../../cad/scripts/prepare-cadrum-occt.py).
- [Cadrum pinned C++ bridge](https://github.com/lzpel/cadrum/blob/8788df70c60b986b5ab387edb75a2f6f341a8c7a/src/ffi.cpp)
  and [mesh extraction](https://github.com/lzpel/cadrum/blob/8788df70c60b986b5ab387edb75a2f6f341a8c7a/src/occt/io.rs).
- [OCCT Boolean documentation](https://github.com/Open-Cascade-SAS/OCCT/wiki/boolean_operations).
  Context7 did not establish version-specific 8.0.1 coverage. The installed,
  checksum-verified `8_0_1_rev2` headers were checked for `SetUseOBB`,
  `SetNonDestructive`, `SetToFillHistory`, `PerformWithFiller`, and glue restrictions.
  Their availability does not prove an option benefits our workload. Recheck the
  pinned implementation when making the corresponding experimental change.
