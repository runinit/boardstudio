# Integrated CAD performance: baseline readiness and paired measurement plan

Researched 2026-10-18 at `797ab74e31f608a630c6983216842c7ea9b829e8` for
[011](../../plans/cad-backend-replacement/issues/011-integrated-cad-performance.md)
and [012](../../plans/cad-backend-replacement/issues/012-final-cutover-remove-cadrum.md).
This is preparation, with no final timing, performance acceptance or ticket closure.
The parent must freeze the measured behavior and grant an exclusive measurement
window before timed runs. No Cargo, wasm-pack or image compilation ran in this lane.

## Measurement design

Compare freshly built historical Cadrum **production exports** at
`6c9c4867755bd2316f751783729a3ff9778fc04a` with the eventual accepted Monstertruck
commit. Keep two explicit views: the same prepared inputs through each shipped
worker/provider, and the same accepted project through each complete packaged page.
The latter includes intervening application changes and is not a kernel-only delta.
Never substitute the historical staged `model/generated.rs` for its registered
`model/construction.rs` exports; that would measure the wrong Cadrum baseline.

Before measurement, run one untimed admission per workload on **both** fresh builds.
Check complete successful output against the existing independent geometry controls,
not merely exit status or nonempty meshes. Freeze the exact request/project bytes,
selectors, expected body IDs, transforms and result gates in the benchmark commit.
Only admitted matching workloads get paired timing rows. If admission fails, retain
the named failure and missing comparison; do not quietly replace the workload,
include failure latency as a speedup, or report only surviving samples. These rows
are a finite admission list, not a claim they have already run on both providers:

1. **C1, holed plate preview:** `cad/test/fixtures/holed-plate.json`, one
   120 × 80 × 2 mm body. Convert its authored outer/hole contours to prepared
   regions using the existing conversion in `cad/wasm/tests/collision-analysis.cjs`;
   freeze that result, do not run different preparers in the provider timer.
   Validate one body, bounds and the manifest's independent 14400 mm³ volume.
2. **C2, tray with bosses:** `cad/test/fixtures/boss-tray.json`, prepared once
   through Core's `prepare-case` flow. Retain that exact prepared assembly for both
   runs. Validate its mounts, cavities, solid count and expected bounds; finish
   independent material/volume expectations before admitting it.
3. **C3, gasketed assembly:**
   `cad/monstertruck/tests/fixtures/layered/gasketed-pair.prepared.json` and its
   analytic/material controls in `cad/monstertruck/src/layered/tests.rs`
   (`prepared_gasket_pair_retains_inner_material_for_tray_and_lid`): 9122 mm³
   per body and explicit groove/interior material probes. Validate both bodies,
   openings and groove material as well as bounds; bounds alone can miss a groove.
4. **P1, drilled PCB preview:**
   `cad/monstertruck/tests/fixtures/layered/pcb-simple.json`, a 20 × 16 × 1.6 mm
   plate with a 2 mm circular through drill. Validate the drill and analytic
   `(320 − π) × 1.6` mm³ volume, not a polygonal substitute. This common prepared
   input does not certify the newer source-backed PCB producer.
5. **V1, switch preview:** the `imported-switch` entry in
   `cad/test/fixtures/followup-manifest.json`,
   `ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step`.
6. **V2, assembly/occurrence preview:** the `imported-multi-solid` entry in
   `cad/test/fixtures/manifest.json`,
   `ergogen/library/vendor/infused-kim/3d_models/trackpoint/TP_Red_T460S_platform_z_offset_+0.0_pcb_offset_-2.0.step`.
   Validate the complete 63-solid source/occurrence inventory and placements,
   rather than accepting a fast partial import. For V1/V2 use the existing pinned
   bounds/source controls; current Monstertruck must additionally retain verified
   source bytes, every source face/occurrence, lines, units and repair metadata.
7. **K1, keycap appearance:** freeze the literal DSA/MX, spherical 0.65 mm dish,
   `O` legend spec from `cad/scripts/browser-smoke/smoke.html`, in a deterministic
   5 × 5 placement grid at 19.05 mm pitch, front side and the same elevations.
   Use stable IDs/revision and verify all 25 caps and legends, finite buffers,
   placement, dimensions and socket/material controls. Different mesh topology
   does not justify changing the requested cap geometry.
8. **E1, case STEP:** C1's exact assembly via the production `export-step` path;
   validate the downloaded bytes with existing independent STEP accuracy controls,
   authored hole/material checks and one solid, outside the timer.
9. **E2, PCB STEP:** P1 through the same path; validate exact cylindrical drill,
   thickness and independent volume. Keep newer curved/source-backed PCB export
   acceptance separately; never downgrade its curves to make an old comparison run.
10. **U1, page case/PCB workflow:** reuse the actual-page smoke's 2 × 2 matrix,
    rigid-mount case and physical PCB scenario. Freeze one accepted `.boardstudio`
    project and identical selections, camera and viewport after confirming both
    historical/current loaders preserve it. Time entering Case to committed meshes
    and a rendered frame; separately time the case/PCB STEP download. Keep worker
    ready, request and accepted-revision/render markers. If document/schema or
    source geometry cannot match, U1 is an explicit missing page comparison;
    provider-only timings cannot stand in for it.

The raw fixture/manifests for C1/C2, the prepared C3/P1 fixtures,
V1/V2 originals and the G20 legacy expectation file
have identical Git blobs at the historical ref and this research HEAD. The
read-only audit also checked `large-plate`, `gasketed-pair` raw inputs and the
`nrf52840.step` controller, but they are not extra timed rows in this bounded plan.
K1's expanded request and U1's frozen project still need materializing/admission.

Fix the common generated/vendor preview settings at 0.1 mm linear deflection and 0.5 rad angular
sampling, with absolute millimetre geometry; record each backend's actual returned
settings and mesh counts. These are meshing settings, not certified error bounds.
Current vendor requests use authenticated `sourceSha256`,
`targetDeflectionMm:null` and `previewRepairBudgetMm:null`, selecting the existing
default preview allowance `max(2 × declared precision, 0.02 mm)`. The historical
`read_step_model(bytes)` has no equivalent options/provenance argument: document
that API/capability difference while using identical source bytes and preview
settings. If a proposed file needs a larger approved repair budget, fail default
admission and report it; an explicitly approved per-file budget can only become
a separately named workload before the run list is frozen. Never tune repair or
deflection after seeing a timing. Retain declared precision, selected allowance
and actual movement separately, including unknown values.

### Repetitions and cache states

For each admitted runtime row, collect **five alternating pairs** (old/new,
new/old, old/new, new/old, old/new). Each backend observation has one cold
operation followed by five warm operations. Keep all five warm values within
their session rather than treating them as independent cold observations.
Repeat the same schedule for checked and release artifacts, reporting them
separately. Use a 180-second per-operation watchdog as an execution bound, not a
performance acceptance limit; a timeout fails the row and requires diagnosis.
Do not retry into a passing statistic without recording why and what changed.

- **Browser cold:** new owned Chromium process/profile, no service worker,
  CacheStorage, IndexedDB assets, HTTP cache or prior V8/WASM module instances.
  Disable HTTP cache consistently. Start the frozen local preview server before
  the timer. The OS filesystem cache remains ordinary/warm; do not call this a
  cold-disk run or flush the user's global caches.
- **Operation cold:** modules initialized and ready, input bytes resident, but a
  fresh CAD worker/module and fresh application derived-asset caches. Time before
  dispatch through completed validated worker reply/typed transfer; U1 additionally
  includes source loading/Core preparation and the accepted rendered frame.
- **Operation warm:** repeat identical geometry/source/options/placement in that
  worker/session, fresh request IDs and valid revisions; keep returned geometry
  released consistently. Report which application/provider caches can hit. For
  vendor rows distinguish an application asset-cache hit from a repeated direct
  importer invocation; the latter must not be called an importer-cache hit.
- **STEP export:** follow the real worker lifetime. Current `ExportStep` deliberately
  creates a fresh module and uses `m1-independent-step-cache-miss`; a prior preview
  does not make this a retained exact-solid hit. Label warm export as warm browser/
  network/application state with a fresh export worker when that is what ran.

Report cold startup separately with five alternating pairs per profile: navigation to
usable editor, then first CAD-worker creation to its `ready` message, and first
user CAD action to visible geometry. Preserve resource/worker events so download,
compilation/initialization and operation time are not silently combined. Shared
monotonic timestamps from the browser host bound dispatch/reply; never subtract
unrelated page and worker time origins. A direct provider `init()` diagnostic is
supplemental, not production worker/page startup evidence.
For warm startup collect another five alternating pairs per profile, each after
one untimed priming navigation in its owned profile with HTTP caching enabled
and service-worker installation settled. Reload the same frozen project, retain
those caches, create a fresh CAD worker and record the actual service-worker/
resource-cache state. Do not pool this with cache-disabled cold navigation.

### Build, size and memory measurements

Run **three alternating paired build cycles per profile**, each cycle consisting
of clean compilation, unchanged rebuild, then one incremental source rebuild.
Profiles are checked `python3 scripts/build-web.py --fast` and shipped
`python3 scripts/build-web.py`, both the single `/boardstudio/` route. This is
36 full build invocations in total across both backends/profiles, not work to
start during reconnaissance. Capture wall time and command/stage boundaries for
providers, page, model staging and packaging, plus the total. No-op results are
separate from incremental compilation results.

“Clean” means no generated products/stamps or compiled outputs in the dedicated
measurement workspace's root target, CAD target, provider packages, staged runtime
assets, renderer package and packaged site. Toolchain, image, verified OCCT archive,
Cargo registry/git downloads, wasm-bindgen/wasm-opt tools and content inputs are
already provisioned. Describe all these retained caches. This measures compilation
and packaging, not network/toolchain provisioning. Any cleanup of existing work
requires the parent's explicit approval; prefer newly provisioned T3 measurement
workspaces with fresh outputs. Never clean active/shared targets or user data.

For incremental rebuild, add the **same harmless comment** to the registered CAD
`src/lib.rs` in each measurement workspace. Capture the exact Git diff; restore
only that owned edit after its cycle. This triggers each build helper's source
digest and a CAD recompile without changing capabilities. Do not change only a
native test (excluded inputs), touch mtimes (content cache may still hit), or call
a stamp-cache hit an incremental compile. Retain normal compiler/profile flags;
no special optimization/accuracy tradeoff is part of this experiment.

After each clean/incremental product, record exact bytes for generated CAD JS/WASM,
CAD-worker WASM/JS, Core/renderer/page/offline-worker products and the complete
packaged site. Report raw per-file bytes and non-duplicated total, plus deterministic
gzip bytes with one fixed level/mtime for a transfer-size comparison. Report STEP
download bytes and returned mesh/analysis buffer bytes per workload. Compression
is an offline diagnostic, separate from localhost transfer timing. Do not compare
an optimized old artifact against a `--no-opt` new one or infer size deltas from
the unbound retained packages.

For each runtime row/profile, capture an idle baseline, peak and retained memory
after each reply/download and after dropping result references. Use one owned
browser process tree, tracking renderer/worker PID ownership; sample Linux
`smaps_rollup` PSS/RSS at a fixed 50 ms interval and retain the raw samples and
sampling interval. Report tree PSS and RSS separately, not the sum of process
high-water marks. Dedicated workers may share a renderer process; this is page/
worker process memory, not isolated kernel allocations. Short peaks can be missed.
Add one fixed 20-operation repeat sequence per admitted row/profile/backend,
dropping downloaded/result buffers consistently, to show retained growth.
Keep GC policy identical (natural GC for the primary run); no claim of leak freedom
or qualified allocator high-water follows from this small sequence.

Record JS heap and WASM linear-memory capacity separately only where the browser
or actual module exports expose them. A diagnostic provider harness may retain
`init()`'s memory export; the production worker currently discards that return.
Do not widen the production API to manufacture a memory number. If worker WASM
capacity cannot be observed, report that limit while retaining process memory;
`performance.memory` alone, Linux `wait4` high-water, or container-launcher RSS
does not measure total CAD memory. `/usr/bin/time` is absent in this lane and is
not a prerequisite: Python monotonic subprocess timing and `/proc` sampling suffice.

### Binding and reporting

For each sample retain backend source commit plus exact owned patch if any,
benchmark/harness commit, command/profile/features, successful build log, target
paths, provider input stamp, packaged build identity, workload Git blob/request
bytes, accepted revision, mesh settings, import options and cache reset/hit state.
Use existing `cad_build.py`/historical `cadrum_build.py` input stamps, Git and
`scripts/run-drag-browser.py`'s `source_identity`, `digest_files`, and receipt
pattern. Bind the actual packaged artifacts that were served, not merely the
checkout HEAD. Input stamps alone are neither artifact checksums nor compiler/
environment receipts. Keep run metadata/results with that receipt, not a new
custom source SHA manifest, PREIMAGES/POSTIMAGES ceremony or source-tree copies.

Record OS/kernel/CPU/RAM, CPU governor, load and competing processes before/after,
browser/executable/version/flags, viewport, graphics mode, server headers/route,
Rust/Cargo/wasm-pack/bindgen/dx/wasm-opt versions, env overrides, registry cache
state and container runtime/image digest. Keep the default check scheduler and
project build flags; record the historical container's effective build concurrency
too (its narrow mounts do not mount the root `.cargo/config.toml`). Do not add a
new concurrency override to disguise that historical recipe. Sequential source
builds only, no shared-target overlap and no competing agent builds in the timed
window. If interference occurs, invalidate and preserve the affected pair.

Publish all raw samples, success/failure counts, median and min–max per row/state,
paired absolute deltas and ratios with direction stated. Sizes are bytes, latency
is milliseconds and build time seconds; memory has explicit units/scope. Five
pairs are descriptive, not a tail-latency or statistical-significance claim.
Diagnose material observed regressions with the parent; no numerical acceptance
threshold, mesh simplification, lower precision or capability deletion is invented.

## Scope differences and required acceptance

The [dated migration decisions](../../plans/cad-backend-replacement/map.md) require
exact **case and PCB** geometry/STEP, mesh keycap appearance/optional printing and
complete source-retaining vendor previews. Obsolete exact keycap STEP and general
3D Boolean workloads are excluded explicitly, not timed failures or reasons to
restore OCCT. At this particular historical baseline, registered keycap construction
is **already mesh-only**, including printing; its keycap files have no Git diff
to research HEAD. Do not misdescribe K1 as exact-to-mesh kernel acceleration.
Optional print latency can later use the same finite K1 spec with `printable:true`
and STL delivery on both sides after admission; it is not added to this run list.

The current-only source-backed PCB and generated/vendor collision qualification
must keep their own functional gates. Add a separate unpaired diagnostic block
with the real-page default and opt-in 0.01 mm collision flows and the actual shared
generated-analysis fixture; do not invent a Cadrum analogue or pool those timings
with V1/V2 import. The
[analysis contract](../../../cad/wasm/src/model/generated/ANALYSIS_CONTRACT.md)
and its JSON fixture remain authoritative, with identity, transfer, refinement,
source completeness and unknown-bound semantics unchanged. The dated user decision
requires supported bands for generated bodies **and qualified vendor classes with
positive pairs**. An all-vendor-uncertain implementation does not pass migration
acceptance; unsupported classes, ineligible containment and insufficient accuracy
remain uncertain. Benchmark admission never changes those requirements.

Keep all required native/WASM/browser/independent-accuracy checks and final review
gates from 010/014/012. Reuse fresh source-bound parent evidence where sufficient;
do not run full suites merely to prepare this note. Failed/partial imports,
timeouts, stale revisions, source-incomplete output, unsupported source-backed
curves and unknown accuracy are recorded as failures/gaps, never successful paired
work. Timing success does not certify interoperability or collision accuracy.
No workload was executed or excluded for an observed execution failure in this
preparation slice; every paired row still awaits actual legacy/current admission.

The existing two-fixture native layered comparison is construction-only: it omits
parse, meshes, STEP, startup, destruction and qualified memory. Its original
protocol is historical evidence, including superseded build flags, not current
operating instructions. It cannot close 011. Read in the parent at
`/home/chris/.t3/worktrees/ts-boardstudio2/t3-fad4a930/.evidence/layered-case-013/performance/root-run-v1/REPORT.md`;
preserve that report and its output-drift note without copying its source trees or
creating more hash manifests. Latest feature/browser evidence stays in the 010/014
ticket comments; this readiness note does not supersede it.

## Historical baseline prerequisites and feasibility

Audited directly with `git show 6c9c4867:<path>`, especially
`cad/scripts/cadrum_build.py`, `cad/scripts/build-cadrum-wasm.py`,
`cad/scripts/prepare-cadrum-occt.py`, `cad/wasm/Containerfile`,
`cad/wasm/Cargo.toml`, `cad/wasm/Cargo.lock`, `scripts/build-web.py`,
`.github/workflows/cad-image.yaml`, `.github/workflows/check.yaml`, and README.
No Git submodule entries or tracked `.gitmodules` exist at that ref. Do not remove
an unrelated unreadable untracked `.gitmodules` if present in a future workspace.

- Manifest: `boardstudio-cadrum-wasm`, `cadrum = "=0.8.20"`, defaults disabled;
  lock checksum `c666eb59f8fdf88e484dfa929e59d7d9f2ba4b7ff1dddd73a66d43c2720f6962`.
  Rust 1.98.0, wasm-pack 0.15.0, bindgen 0.2.129, dx 0.7.10 and Python 3.11+.
  CAD builds `wasm32-unknown-unknown`, release/locked, in an independent workspace.
- The historical helper verifies the pinned `occt-8_0_1_rev2` WASM archive, then
  mounts CAD, contracts and a temporary minimal workspace manifest in Podman/Docker.
  Its Cargo target is a real `cad/wasm/target`; CAD cache/pkg/target symlinks reject.
  Normal builds link prebuilt OCCT and compile the C++ bridge, not all OCCT source.
  Native OCCT plus `cad/step-oracle` are required by historical native tests, not
  by the production browser baseline build; do not launch them for reconnaissance.
- The helper's exact Containerfile-derived tag is
  `ghcr.io/runinit/boardstudio-cadrum-wasm:containerfile-3e43d7948267f553`.
  Podman 6.1.4 lists it and the helper's local alias
  `boardstudio-cadrum-wasm:0.8.20-rust-1.98.0-wasi-sdk-33-bindgen-0.2.129`
  as the same local image ID
  `4fe3f768e74e86385db537dd58414b39eca12626d4d48b0e119b5f6a3a2bdeee`
  (2,532,117,924 bytes). Manifest inspection returns linux/amd64 digest
  `sha256:569918e5874f8908957a76e2b2df7518d9c7886f5d224c5536ec2ca9896fd2f0`,
  also present in the local image's RepoDigests. Tags alone are not provenance;
  bind that digest and rerun the toolchain probe before use.
- A read-only, network-disabled, no-pull container probe succeeds: Rust 1.98.0,
  wasm-pack 0.15.0, bindgen 0.2.129, clang 22.1.0-wasi-sdk and the exception-enabled
  WASI sysroot library directory. No compilation/pull/image mutation was needed.
  This establishes runnable prerequisites, not a successful old provider build.
- The exact retained 35,498,172-byte OCCT WASM archive at
  `/home/chris/01_Projects/ts-boardstudio2/cad/.cache/cadrum/wasm/occt-8_0_1_rev2-wasm32_unknown_unknown.tar.gz`
  matches the historical helper's pinned SHA-256. Reuse verified archive/download
  caches as prerequisites only, without sharing mutable build targets or products.
- A pinned-base manifest probe returned Podman's single-image/list limitation:
  `Treating single images as manifest lists is not implemented`. That is not
  evidence the base image is missing. The fallback Containerfile would rebuild
  the wasi-sdk-33 sysroot from its pinned source commit with legacy exceptions,
  then install Rust/tools. The ready final image makes that costly fallback
  unnecessary here; do not start it automatically if readiness drifts.

Retained Cadrum JS/WASM exists under the main checkout's `cad/wasm/pkg` (WASM
19,785,714 bytes, Oct 16 mtime), alongside input stamp
`e40a3dd169ec0e912b36c2cee28c5efd0c7c0ad093ad5974361359b8faf830dc`.
The official historical `cadrum_build.wasm_inputs_digest` at clean
`6c9c4867755bd2316f751783729a3ff9778fc04a` gives
`373b765945f8dc60b3052eefd84f43c7fd2af349e84b37f63e66a772a2d73f8f`
for release and
`af404602ba9de053fbc0938c820933b604c6207ea9a84c4915ff6160c8dbbc01`
for release-no-opt. These official helper-derived values supersede the earlier
reconstruction, which sorted pathname strings instead of `pathlib.Path`
components in the helper's order. Neither official value matches the retained
artifact's `e40a3dd169ec0e912b36c2cee28c5efd0c7c0ad093ad5974361359b8faf830dc`
input stamp. Even a matching input stamp would not bind
the current output bytes by itself. **Do not assign that retained artifact to
6c9c4867 or use its size/timings as this baseline.** Preserve it in place; a fresh
build/source/artifact receipt is needed. No build-history provenance was inferred
from the filename, mtime, neighboring stamp or container tag.

## Current path and smallest next action

The current [build orchestrator](../../../scripts/build-web.py) builds Core/CAD/
offline workers and renderer, invokes
[build-cad-wasm.py](../../../cad/scripts/build-cad-wasm.py), stages models, builds
Dioxus and packages `/boardstudio/`. The
[CAD helper](../../../cad/scripts/cad_build.py) uses host wasm-pack, release/locked,
`cad/wasm/target`, `boardstudio_cad_wasm` and
`cad/.cache/cad/wasm-build.sha256`; `--fast` supplies `--no-opt` while retaining
release CAD compilation and an unoptimized page. The independent CAD manifest
patches the vendored Monstertruck crates. The packaged worker entry imports
`assets/cad/boardstudio_cad_wasm.js`. The Rust
[worker](../../../web/crates/host/src/cad_worker.rs) initializes lazily, validates
requests/transfers/revisions and selects preview, exact, STEP, vendor or keycap
operations. STEP-only export avoids mesh allocation and uses its own worker.
`model/metrics.rs` stage hooks are no-ops; they are not usable internal timings.

No existing tool records this complete paired benchmark. Reuse
[the actual-provider smoke](../../../cad/scripts/test-cad-browser.py),
[page smoke/receipts](../../../scripts/run-drag-browser.py),
[browser prerequisite probe](../../../scripts/browser_tools.py), local preview
server and worker protocol for a small measurement driver **after** both builds
and workload admissions exist. Do not create another build framework, duplicate
analysis contracts, add security manifests or instrument production before a
concrete measurement seam is known. No benchmark helper is added in this slice.

The host has the pinned Rust/wasm-pack/dx versions. An initial `browser_tools.locate()`
fails because `NODE_PATH` is unset. With
`NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules`, the existing
cached Playwright 1.62.1 passes the fail-fast probe with `/usr/bin/node` and
`/usr/bin/chromium` (Chromium 155.0.8059.79). No install/config change is needed.
Re-run this probe before any build; follow the existing browser guards rather than
discovering the missing module after a long compile. This lane has no generated
CAD/site products and no `.gitmodules`; it was clean before this note.

**Exact next executable action for the parent:** provision a separate **T3-bound
historical workspace at `6c9c4867755bd2316f751783729a3ff9778fc04a`**, with real
CAD output/cache directories and root/CAD targets proven distinct from active
workspaces. Do not use shell `git worktree`, reset this lane, restore old CAD into
production, or assume delegation at today's parent HEAD selects that old ref.
In that workspace, verify the image digest/alias again, supply the verified archive
through the historical preparer, and confirm its input digest. Then, when the
parent schedules the prerequisite build and its cost, the first untimed command is:

```sh
CADRUM_IMAGE_READY=1 python3 cad/scripts/build-cadrum-wasm.py --no-opt
```

This explicitly uses the verified local image and avoids the helper's automatic
pull/local-SDK-build fallback; `CADRUM_IMAGE_READY` is safe only after that image
check. It is a **fresh CAD compile/link**, cost not yet measured, and does not
produce a full packaged page. Follow with the historical
`python3 scripts/build-web.py --fast` and a source/artifact-bound smoke when the
parent allocates that work. Optimized provider/page builds remain necessary later;
do not reuse checked products for release. The existing image/archive avoids
provisioning compilation; it does not avoid provider compilation.

Readiness blockers are therefore: the historical T3 workspace and fresh bound
artifacts; successful matching workload admission/frozen project and driver;
stable current behavior including outstanding 010/014 acceptance; and the parent's
exclusive measurement window. There is no demonstrated missing toolchain/image
blocker. Do not start the timed matrix or claim performance acceptance until
those gates are satisfied.

Lane audit logs (gitignored, not copied from the parent) are
`.evidence/011-baseline-image-audit.log`,
`.evidence/011-baseline-toolchain-probe.log`,
`.evidence/011-retained-baseline-audit.log` and
`.evidence/011-baseline-input-audit.log`. Existing parent-only historical evidence
links remain intact; isolated-lane missing evidence is not a reason to delete them.

Verification: all 11 new-note local links resolve and `git diff --check` passes.
The doc-link tool's four tests pass. Documentation-only `check.py quick --files`
stops on 288 existing links to absent ignored `.evidence` paths in this isolated
lane, with no failure in this note; see `.evidence/011-doc-check.log`. These links
were preserved for the parent evidence checkout. No full native/WASM/browser
suite, independent review or final benchmark was run in this research slice.

## Readiness update — 2026-10-18

The historical baseline lane has now built actual Cadrum at
`6c9c4867755bd2316f751783729a3ff9778fc04a` in its separate T3 workspace, with
15 smoke checks passing. First untimed C1 paired admission passed for the frozen
120 × 80 × 2 mm plate with 24 square through-holes: independently derived volume
14400 mm³, mesh topology, 126 material probes and independent inspection of both
providers' emitted STEP passed. The one-time FreeCAD inspection is QA evidence;
it does not restore a required OCCT/C++ test dependency. This fresh artifact and
admission leave the older retained artifact unbound. No performance timing claim
follows. Historical page/worker baseline packaging, remaining workload admissions
and the acceptance gates above remain open; neither 011 nor 012 is closed.
