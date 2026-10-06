# Gasket generation comparison results

> Historical research: measurements, code paths and integration status below
> describe the recorded revision. For current ownership and commands, use
> [architecture](../../architecture.md) and the [development guide](../../../README.md).

Historical raw artifacts and experimental sources now live in the performance
worktree. See Development worktrees (historical record in Git at `323967ff`) for their location.

Measured 2026-09-28 against `49d33a91`. The current design and production backend
are unchanged. These are isolated, uncached **single-bottom** experiments, not
new application release-to-paint measurements. The exact gasket target remains
200 ms p95; the latest full-app result remains 370 ms.

This is the **first bounded batch**, not the entire optimization plan. Follow
the E0–E9 coverage table (historical record in Git at `323967ff`) and
[continued OCCT experiments](occt-optimization-round2.md) for the remaining work.

## Decisions

| Candidate | Median / p95 | Compared with Cadrum control | Decision |
| --- | ---: | ---: | --- |
| Cadrum control | 108.4 / 113.2 ms | — | Retain for exact CAD and export |
| Cadrum without edge output | 108.4 / 113.0 ms | 0.0% median | No meaningful latency gain |
| Cadrum without face-ID output | 108.6 / 112.0 ms | +0.2% median | No meaningful latency gain |
| Cadrum without either output | 108.8 / 114.0 ms | +0.4% median | Do not prioritize for speed |
| Cadrum height-band profiles, lean mesh | 151.6 / 160.0 ms | +39.9% median | Reject this construction variant |
| Manifold preview | 24.1 / 26.2 ms | −77.8% median, about 4.5× faster | Candidate for a separate preview integration experiment |
| Monstertruck direct construction | No completed bottom | Failed/timed out on first circular hole | Does not qualify for browser speed comparison |

The table uses the retained regression fixture. Three actual gasket-edit states
show the same result: lean Cadrum varies from −0.4% to −0.1%, the profile variant
is 39.1–39.5% slower, and Manifold preview is 77.8–77.9% faster. All five session
medians support the profile and Manifold conclusions. Differences below 1% in
mesh-output variants are not evidence of a useful improvement.

Manifold's time cannot be called faster exact CAD generation. It produces a
polygonal preview, while Cadrum also constructs exact solids. Neither result
establishes the performance of a dual-engine application or its export flow.
No candidate has been promoted into production.

## What consumes Cadrum time

Twenty-five separate instrumented runs of the regression bottom measured:

| Stage | Median | Scope |
| --- | ---: | --- |
| Boolean `Perform` | 79.1 ms | Four CellsBuilder operations |
| Intersection | 35.7 ms | Separate PaveFiller attribution variant |
| Building the result from intersections | 43.8 ms | `PerformWithFiller` in that same variant |
| Cell selection | 0.8 ms | Four operations |
| Internal-boundary cleanup | Below clock resolution | Four calls; not proof of zero cost |
| Bridge history relay | 0.1 ms | Eight intervals; excludes internal OCCT history work |
| Post-Boolean deep copies | 2.8 ms | Four copies; not every copy in the application |
| Surface meshing | 15.6 ms | One combined mesh call |
| Mesh extraction and normals | 0.1 ms | C++ extraction |
| Edge polyline sampling | 0.2 ms | Output currently discarded by our adapter |

The split-filler variant passed geometry checks and took 109.1 ms median with
instrumentation. Normal instrumented control took 108.6 ms versus 108.4 ms without
instrumentation. This is an approximate overhead check, not a claim of zero
instrumentation cost. Stage medians are not additive percentiles; intersection
and result-building replace `Perform` in a separate experiment.

Cadrum's WASM clock stub returns zero, so the isolated instrumentation uses
`performance.now()`. The clock resolves about 0.1 ms in this browser. All speed
samples keep stage timers disabled. Ownership copies remain enabled.

The profile prototype adds extrusions and unions of touching height bands. It
still spends 82.1 ms in Boolean `Perform`, then adds 4.8 ms cell selection,
6.5 ms cleanup, 6.7 ms copying, and 21.8 ms meshing. Its slower result rejects
this particular implementation, not parametric design generally.

## Geometry and validation

Four fixtures cover the retained bottom regression and captured gasket edits
0, 2, and 5. They preserve both regions, all lobes/tabs, partial-depth polygon
openings, circular mounting holes, and nut pockets.

- Six Cadrum variants on each fixture passed solid count, bounds, volume,
  bidirectional material subtraction, and STEP round-trip checks. Output-only
  variants have exactly identical mesh positions and normals to control.
- All **24 STEP files** independently passed libcascade BRep validity,
  two-solid count, and both directional material differences below 0.01 mm³.
- Twelve browser meshes passed finite buffer, triangle winding/normal,
  nondegeneracy, and sampled material-occupancy checks. Every mesh also had zero
  unpaired or inconsistently directed welded edges.
- Each Cadrum browser mesh had about 31,100 occupancy samples; each Manifold mesh
  had 33,426, including targeted hole/depth probes. All had zero mismatches.
  Samples within the applicable circular tessellation error band are excluded
  and counted. Sampling supplements exact CAD checks; it is not a proof over
  every possible input.
- Manifold returned two components and `NoError`. Its 32-sided circular cutters
  have maximum chord error **0.00530 mm**, within the existing 0.1 mm linear and
  0.5 rad angular mesh limits. Its volume difference from exact CAD was
  **3.3642 mm³**, below the conservative polygonal-hole bound of **3.8908 mm³**.
  This approximation is acceptable evidence for preview testing, not exact STEP.
- The staged crate's **16 ordinary native tests** passed; five diagnostic tests
  were excluded by their existing ignored markers, with the new equivalence
  diagnostic then run explicitly on all four inputs. Repository and Rust/WASM
  boundary checks passed.

Two initial correctness failures are retained. Overlay quantization left almost
coincident Cadrum profile edges: volume and STEP validity looked correct, but
native material subtraction failed. Restoring unchanged authored vertices made
all subtraction checks pass. Manifold initially produced four components because
one band ended at `1.7999999999999998` and the next began at `1.8`. Assigning both
endpoints to the exact shared authored height fixed the gap without overlapping
material. These are reasons to keep material/topology checks in future experiments.

## Monstertruck result and limits

Monstertruck **0.4.1**, with its default marching SSI backend, was tested natively
using analytic polygon extrusions and circular cutters before polygon openings.
Its public binary Boolean API requires sequential cuts in this probe, unlike
Cadrum's batched expression. The first mounting-hole operation prevented the
complete bottom from reaching meshing or export:

| Boolean tolerance / cutter extension | Result |
| --- | --- |
| 0.01 / none | `EmptyOutputShell` |
| 0.0000001 / none or 0.01 mm | Panic: internal meshing requires tolerance ≥ 0.000001 |
| 0.000001 / none | Exceeded 30-second limit on first hole |
| 0.000001 / 0.01 mm | Exceeded 30-second limit on first hole |
| 0.01 / 0.01 mm | `InvalidOutputShell`, `NotClosedShell` |

The 0.01 trials are deliberately coarse failure diagnostics and cannot satisfy
our unchanged exact-geometry contract. Extending a cutter outside the part tests
coincident-face sensitivity without changing the intended material boundary.
The base and extended cutter exported by Monstertruck both passed independent
OCCT validity with positive volumes. OCCT successfully subtracted those same
inputs. This narrows the problem to this construction/kernel combination rather
than an obviously invalid input solid.

There is no valid Monstertruck generation-time number, no successful final STEP,
and no WASM ranking. This bounded result is sufficient to keep it out of the
production path for now; it does not establish that every Monstertruck algorithm
or future version fails. A future retry should start with this minimal hole
reproducer and a confirmed fix, before any engine migration or browser benchmark.

## Measurement limits and retained evidence

Five fresh browser workers, four fixtures, six variants, ten warm samples per
fixture/variant/session: **1,200 warm observations**, plus separately retained
cold/warmup and attribution observations. Variant order reverses on alternating
iterations. Builds and correctness checks ran outside timing sessions.

Host: AMD Ryzen 9 8945HS, Linux; headless Chromium 153.0.8010.12; Node 26.10.0;
Cadrum 0.8.20 and OCCT 8.0.1 rev2; Manifold 3.5.4. This browser differs from the
historical full-app reference, so those absolute timings must not be compared.
Cadrum control and candidates use the same diagnostic binary with runtime output
switches. It retains disabled hook calls and is not a byte-identical production
binary. A production integration still needs the unmodified application control.

Generation includes construction, forced lazy evaluation, meshing, normals, and
expanded f32 render buffers. It excludes module startup, JSON transport,
validation, object destruction, app preparation, rendering, and other bodies.
Thus the table is not total worker wall time or a promise of a 200 ms UI result.

Within each session Cadrum's shared WASM capacity grew from 51,773,440 to
52,559,872 bytes and then remained at that level. This is allocator capacity,
not live memory or RSS, and cannot attribute retention to a single variant.
Manifold's package does not expose equivalent heap capacity here. Its handles are
explicitly released, but no complete memory/renderer/export soak has been run.
The existing full application acceptance suite was not rerun for these isolated
experiments. It remains required before any production adoption.

- Reproduction commands and implementation (`cad/experiments/gasket/README.md` in the performance worktree).
- Raw observations and artifacts (`cad/bench/results/phase3-gasket-comparisons/README.md` in the performance worktree).
- Per-fixture/session comparisons and attribution (`cad/bench/results/phase3-gasket-comparisons/analysis.json` in the performance worktree).
- Preview geometry checks (`cad/bench/results/phase3-gasket-comparisons/preview-validation.json` in the performance worktree).
- Failed attempts and fixes (`cad/bench/results/phase3-gasket-comparisons/failure-investigations.json` in the performance worktree).

## Revised priorities

1. **Exact path: specialize or avoid expensive 3D Booleans.** Test a guarded
   `BRepAlgoAPI_Cut` path against CellsBuilder for simple body-minus-tools
   expressions, and independently test OCCT OBB filtering. Keep ownership,
   error checks, and material/STEP equivalence. The result-building stage is at
   least as significant as intersection, so broad-phase changes alone may not win.
2. **Exact path: construct eligible circular through holes in the extrusion
   profile.** Preserve analytic circles and apply the existing partial-depth
   openings afterward. This targets mount Booleans without the unsuccessful
   full-height-band union strategy. Reject or fall back for overlapping/touching
   holes, unsupported features, or any change in manufacturing geometry.
3. **Preview path: test Manifold in an isolated end-to-end branch.** Preserve
   authoritative prepared inputs, explicit preview status, newest-revision
   scheduling, cancellation, exact fallback, and Cadrum readiness/STEP export.
   Measure all seven changing bodies, duplicate CPU work, startup/download size,
   renderer cost, and a full memory/export soak before selecting integration.
4. **After attribution changes, revisit exact meshing and narrowly scoped reuse.**
   Keep topology cleanup, history suppression, and ownership-copy removal below
   the Boolean work. The measured bridge overheads do not justify their risks
   as the next speed project. Internal OCCT history suppression is still an
   untested hypothesis, not disproved by the small bridge-relay measurement.

The broad E0–E9 plan remains a menu of unexecuted alternatives outside this bounded
screening. No case redesign or production kernel migration is selected.
