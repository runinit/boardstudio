# Monstertruck STEP import: current support and BoardStudio routes

Researched 2026-10-16. Upstream source was pinned to `virtualritz/monstertruck`
HEAD `d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652` (2026-10-07). That tree still
declares crate version 0.4.1, but the `v0.4.1` tag resolves to the older
`e6520c2b1bde6f46a85649024663583cd03f484c`; pin the Git revision when relying
on the inspected source. A local Git tree comparison found the `monstertruck-io`
and `monstertruck-wasm` trees byte-identical between those commits; only
`.blueprints` and `.gitmodules` differ. Thus HEAD contains no post-v0.4.1 STEP
implementation changes, and the October 5 spike tested the same importer source.
No Monstertruck code was changed or built for this note.
BoardStudio code was inspected at `be319d54347d70613b746eef0e02cf3db0a0fd88`.
[Pinned manifest](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/Cargo.toml),
[v0.4.1 tag](https://github.com/virtualritz/monstertruck/tree/e6520c2b1bde6f46a85649024663583cd03f484c)

## Finding

**Confirmed requirement (2026-10-16): remove OCCT/C++ completely.** A separately
loaded OCCT STEP importer is not an acceptable destination. The target is a
Rust-only CAD implementation and required build/test path, preserving generated
case/keycap geometry, vendor STEP previews, exact STEP exports and original
asset retention. Historical OCCT/FreeCAD measurements remain useful comparison
evidence; they do not justify retaining an OCCT dependency in the replacement.
The hybrid route discussed below is an evaluated alternative rejected by this
requirement. No backend migration has been implemented.

STEP is a first-class Monstertruck exchange format. The source has a parser, an
entity table, conversion into trimmed B-rep shells/solids, an assembly DAG with
placement matrices, and loss reports. It also has native STEP corpus,
assembly, and round-trip tests. These capabilities were present in the same
0.4.1 implementation BoardStudio measured on 2026-10-05; upstream HEAD has not
changed them since. There is no source delta that justifies rerunning the same
spike as a supposed current-version improvement.

BoardStudio's direct evidence remains adverse: the resolved Monstertruck 0.4.1
spike loaded and validated all 53 BoardStudio-generated STEP exports; 47/53
initially matched comparison measurements. The remaining six matched under the
OCCT oracle after fixing a
JSON float-rounding issue in the harness inputs, which is not independent proof
that Monstertruck matched all 53 baseline measurements. Only 16 of 33 vendor
corpus files passed the spike's validity and measurement gate. Several imports had
large non-watertight tessellation openings, three had placement mismatches, and
the slowest import took 129.5 seconds. The spike's strict OCCT oracle also
rejected 17 files; that result does not mean BoardStudio's production Cadrum
reader rejects those files, since its sewing behavior accepts some orphan-face
cases. The two kernel paths disagreed on some corpus files. The importer source
is unchanged since the spike, so these results remain relevant to that adapter.
[Resolved spike commit](https://github.com/runinit/boardstudio/commit/767abb19401da1aebebd4317a60ef542ca27ac9e),
[October 8 evidence reconciliation](../dependency-and-cad-options-2026-10-08.md)

There is a separate blocker for using Monstertruck as the production CAD
backend: the 2026-09-28 gasket comparison records a direct Monstertruck
generation failure/timeout at the first circular mounting-hole cut, while OCCT
accepted the same base and cutter. The experiment produced no final STEP or
browser comparison. Fixing STEP import alone therefore cannot qualify a kernel
cutover. [Gasket comparison results](../performance/gasket-comparison-results.md)

BoardStudio's current STEP feature is narrower than a general editable CAD
importer. `read_step_model_data_with` imports a shape and immediately computes
its mesh and bounds; the worker returns `MeshArrays` with no colors or retained
imported topology. The original STEP bytes are retained with hash validation and
copied into KiCad footprint exports and project archives. This means a
replacement only needs to preserve the current preview mesh, bounds, transforms,
identity, and original bytes to keep today's vendor model workflow intact.
Generated case/keycap geometry remains a separate exact-B-rep workflow. See
[model loading](../../../cad/wasm/src/model.rs),
[worker routing](../../../web/crates/host/src/cad_worker.rs),
[model delivery](../../../web/crates/runtime/src/model_delivery.rs),
[footprint export](../../../web/crates/runtime/src/export_footprints.rs), and
[project archive](../../../core/src/archive.rs).

The actual CAD feature surface that needs a replacement kernel is broader than
STEP preview: case construction covers plates/trays/lids, holes and cavities,
gaskets, mounts/openings, support prisms, and round/conical seats; keycaps use
lofts, sphere/cylinder dishes, stems/sockets, font contours and lettering cuts.
These paths and caches retain `cadrum::Solid` directly, while the `CadKernel`
trait only abstracts case build, STEP import/export, and meshing. So the trait
is a useful boundary for a generated-solid experiment but does not yet isolate
all production operations. [Case construction](../../../cad/wasm/src/model/construction.rs),
[keycap generation](../../../cad/wasm/src/model/keycaps.rs),
[kernel seam](../../../cad/wasm/src/model/kernel.rs),
[solid cache](../../../cad/wasm/src/model/construction/cache.rs)

## What the pinned importer does

- `monstertruck-io` enables STEP read/write by default. `Table::from_step` and
  `Table::from_step_bytes` parse ISO 10303-21 into typed maps; the byte route
  accepts UTF-8 and falls back to ISO-8859-1 for raw 8-bit vendor strings.
  [I/O feature manifest](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/Cargo.toml),
  [STEP table loader](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/load/mod.rs)
- Supported typed geometry includes elementary planes/cones/cylinders/spheres/
  tori, B-spline and rational spline curves/surfaces, Bezier forms, extrusion
  and revolution surfaces, conics, and STEP pcurves/trim boundaries. Topology
  includes vertices, edge curves, loops, face bounds, advanced faces, shells,
  shell-based surface models, and manifold solid B-reps. Conversion offers
  compressed shell and trimmed-shell/solid paths; the trimmed route retains
  face-local parameter curves. This is a useful native B-rep path for analytic
  geometry, but the supported-entity list is not equivalent to all AP203/AP214/
  AP242 entities. [Entity table](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/load/mod.rs),
  [surface types](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/load/step_types/surface.rs),
  [conversion API](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/load/convert/mod.rs)
- Assembly import is an explicit `Table::step_assy` path. It models products,
  shape representations, `NEXT_ASSEMBLY_USAGE_OCCURRENCE`,
  `ITEM_DEFINED_TRANSFORMATION` and transforming relationships, returning a DAG
  with matrices. Source tests report that two Scania files yield 832 and 254
  solids, respectively. Crucially, occurrence placement is carried on the
  assembly edge; merely loading a product does not transform its geometry. The
  caller must apply the matrix exactly once when building world-space geometry.
  This supports one assembly encoding, not every representation-map/mapped-item
  dialect. [Assembly entities and
  traversal](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/load/step_types/product.rs),
  [assembly conversion](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/load/convert/mod.rs)
- The importer exposes loss reports. `Table` records refused entities; reported shell
  conversion tallies missing faces, wires, edges, and vertices; strict helpers
  can turn those reports into typed errors. The ordinary conversions remain
  permissive, so callers must ask for the reports and enforce the acceptance
  policy themselves. Parsing `Ok` alone still does not mean all file entities
  were understood or that the resulting shell is valid/closed. [Load errors and
  table report](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/load/mod.rs),
  [loss report](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/load/report/mod.rs)
- Assembly export and single-shape STEP export exist too. Upstream tests cover
  round trips and watertightness on its fixtures. Those establish an API and
  local regression coverage, not independent OCCT validity for BoardStudio's
  generated case/keycap outputs. [STEP save modules](https://github.com/virtualritz/monstertruck/tree/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/save),
  [I/O tests](https://github.com/virtualritz/monstertruck/tree/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/tests)

### Remaining questions in upstream source

The typed representation context stores only identifier/type strings; unit
assignment entities are not represented in the importer table or applied during
conversion. Therefore the load path does not normalize inches, millimetres, and
metres into a common coordinate convention. Geometry coordinates are retained
as parsed numbers; callers cannot infer millimetres from `Table` or
`step_assy`. The resolved spike's test-only adapter already has a basic
`declared_length_unit_mm` scan for assigned SI units and conversion-based units
(including inch factors), and scales measured geometry after applying assembly
transforms. Reuse and audit that code before writing another parser. Its limits
are material: it strips whitespace textually, assumes one common assigned
length factor (rejecting conflicting units), defaults missing units to mm, and
is not used by the kernel itself. Likewise, the table has an explicit
entity allowlist and a dummy bucket for unknown entities. The loss report makes
some drops visible, but unsupported classes, assemblies encoded through
different STEP relationship patterns, colors/appearance, PMI, and advanced
metadata need a feature-specific audit. [Context/table declarations](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/load/step_types/mod.rs),
[entity dispatch](https://github.com/virtualritz/monstertruck/blob/d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652/monstertruck-io/src/step/load/mod.rs)

The parser and many conversion paths return `Result` with parse/conversion
errors. This source inspection did not prove panic-freedom for malformed or
adversarial files; the importer has code paths guarded by assumptions and its
fixtures/tests are not a substitute for fuzzing. Set file-size/time/memory
limits at the worker boundary and verify cancellation and error reporting in the
WASM build before exposing vendor imports.

The historical oracle adapter is valuable code to recover, but its report is
not a clean guarantee of placement correctness: it ignores `step_assy` errors,
substitutes identity when a placement matrix fails conversion, and falls back
to flat identity instances when no placements are found. That can hide exactly
the assembly defects being measured. It also chooses compressed-solid
conversion because Monstertruck's regular `Shell` refuses valid STEP loops with
both edge ends at the same vertex (such as full circles); a trimmed-solid path
should be compared as a targeted probe, not assumed superior. [Spike adapter](https://github.com/runinit/boardstudio/blob/767abb19401da1aebebd4317a60ef542ca27ac9e/cad/step-validator/src/lib.rs)

## Evaluated STEP routes

| Route | What changes | What it retains / costs |
| --- | --- | --- |
| Qualify and extend Monstertruck import | Add a BoardStudio adapter that parses bytes, calls the reported trimmed-solid/assembly path, normalizes units, applies or retains placements, and reports losses/errors. Extend its parser/converters for the high-value corpus misses. | Keeps exact B-rep only where Monstertruck can represent and validate it; one Rust/WASM direction. Requires an explicit refusal policy for partial solids and new independent geometry qualification. This is the best route if the objective is to remove OCCT from STEP input, but it does not solve the independent CSG failure. |
| Retain an OCCT STEP-to-mesh compatibility path | Keep STEP import on the existing Cadrum/OCCT route and return the same mesh/bounds data. Continue preserving/hashing the original STEP bytes for downstream KiCad exports and archives. If generated B-rep work moves to Monstertruck, route only `ReadStep` through OCCT. | Preserves today's vendor preview behavior without translating B-reps between kernels or replacing imported model assets. OCCT stays in shipped WASM/build artifacts, so this retains STEP support but does not complete the pure-Rust dependency/build replacement. The current separate worker route for `ReadStep` and `ExportStep`/generated construction makes this a small routing seam. |
| Preconvert vendor STEP into cached meshes | Run a trusted exchange converter locally or in-browser and store mesh plus placement/identity metadata. | The current import already returns preview meshes and retains original STEP bytes; preconversion adds a local converter/cache path. The app has no glTF loader or remote conversion service, and its current local/static deployment means a server converter changes the product boundary. Preserve placements and identity sidecars. |

Under the confirmed requirement, the OCCT compatibility route is excluded.
Preconversion also fails the requirement if it retains an OCCT/C++ converter;
preconverting bundled assets alone does not preserve arbitrary user-uploaded
STEP support. A Rust STEP reader may still be separate from the modeling
kernel: today's component previews need mesh/bounds, not a transfer of imported
solids into Monstertruck. That separation remains useful even with OCCT removed.

## Recommended next evidence

Do not rerun the same 0.4.1 importer on the same 33 files just because upstream
HEAD advanced: its I/O source is identical. Recover the test-only Monstertruck
adapter and first make its
assembly handling fail closed. Reuse its unit scan as a starting point, then
test its stated single-unit/default behavior and conversion-based inch factor.
The code change would be isolated to STEP decode/conversion and the `ReadStep`
route; no imported Monstertruck solids need to escape into current preview
state. These are research candidates only; no production changes were made.
Start with concrete vendor failures, paired inch/mm STEP
fixtures, a placement fixture that must fail if its matrix is unavailable, a
compressed-vs-trimmed conversion probe, and the minimal mounting-hole boolean
reproducer. Run these as isolated experiments against the current baseline;
do not introduce a production hybrid as their outcome. For each probe, record:

1. Parser/entity loss report and strictness outcome; B-rep count; shell closed /
   oriented status; and whether a part/assembly graph was recovered. Fail if
   assembly conversion or any referenced matrix fails; never substitute
   identity or silently flatten an assembly.
2. Bounds and volume before and after applying product/occurrence transforms;
   units must be compared in millimetres against OCCT/FreeCAD reference output.
3. Mesh watertightness and opening count, STEP re-export/reimport validity under
   OCCT, import time and peak memory. Include an explicit oversized/malformed
   input and verify a located error rather than a hang, panic, or partial model.
4. Separate importer pass criteria from Monstertruck generation/boolean gates.
   The existing 16/33 vendor pass rate and 129.5 s worst import remain the
   relevant baseline because current upstream STEP source is the same revision.

The implementation/test split should follow existing worker operations:

| Operation | Initial migration routing | Focused acceptance checks |
| --- | --- | --- |
| `ReadStep` | Qualify a Rust reader against accepted files before replacing Cadrum/OCCT. Preserve worker result shape and retained source bytes/hash/export/archive behavior. | Current source validation ledger (including VIK and THQ/MX fixtures); transformed 63-solid cache fixture; concrete vendor failures; paired inch/mm files; bounds and mesh parity; placements, oversize and malformed-file errors; KiCad footprint export and archive retain original bytes. |
| `Preview` / generated `Exact` assembly | Candidate kernel may build generated case bodies only after the full existing case fixture gate passes. | Plates/trays/lids, gasket, mounts/openings, cavities/support prisms, round/conical seats; compare body count, closedness, volume, bounds, mesh, errors. |
| `ExportStep` | Candidate writer only after generated solids qualify; keep STEP-only cache behavior and independent reference evidence. | Case and keycap STEP interoperability checks during qualification; keycap loft, dish, stem/socket and lettering cuts; exact geometry and repeated-cache behavior. Replace required OCCT-based tests before complete removal. |

The current CAD worker dispatches these operations through one initialized
WASM provider. Preserve that simple boundary unless evidence requires separate
Rust providers. A second loader or a generic backend-selection framework is
not a prerequisite for this experiment.

## Concrete code affected by a backend replacement

This is a change map, not a proposed general-purpose kernel framework:

| Code | Required change and behavior to retain |
| --- | --- |
| [Kernel boundary](../../../cad/wasm/src/model/kernel.rs) and [case construction](../../../cad/wasm/src/model/construction.rs) | Implement the candidate's generated solids and writer against existing prepared inputs. Preserve feature validation, multiple bodies, units and structured errors. The current builder itself uses Cadrum operations; changing the trait implementation alone does not port it. |
| [Preview/export caches](../../../cad/wasm/src/model/construction/cache.rs) | Replace retained `Rc<Solid>` geometry and direct `Solid::write_step` calls when generation migrates. Preserve bounded retention, ownership, reuse, cache misses and STEP-only export. |
| [Keycaps](../../../cad/wasm/src/model/keycaps.rs) and [lettering](../../../cad/wasm/src/model/keycaps/lettering.rs) | Port the lofts, curved dishes, cavities, sockets, stems and letter solids, including templates and placed exports. These bypass the kernel trait today. |
| [Worker](../../../web/crates/host/src/cad_worker.rs) and [model delivery](../../../web/crates/runtime/src/model_delivery.rs) | Replace provider internals while retaining mesh/bounds responses, hash/size checks, revision and scope checks, cancellation and worker recovery. Existing Rust rendering and placement math can consume the same buffers. |
| [Manifest](../../../cad/wasm/Cargo.toml), [initialization](../../../cad/wasm/src/lib.rs), [CAD build](../../../cad/scripts/build-cadrum-wasm.py) and [web packaging](../../../scripts/build-web.py) | Remove Cadrum/OCCT linking, archives, C++ initialization and the OCCT-specific build pipeline after all consumers migrate. Verify lazy loading and offline assets. Measure build time and payload instead of assuming savings. |
| [Case gate](../../../cad/wasm/src/model/construction/validation_tests.rs), [source assets](../../../cad/wasm/src/model/source_validation_tests.rs), [STEP oracle](../../../cad/step-oracle/src/oracle.cc) and [browser smoke](../../../cad/scripts/browser-smoke/smoke.html) | Reuse native geometry expectations and source ledgers, then verify actual WASM bindings, worker transfers, recovery and root/subpath delivery. Replace the required native OCCT oracle with sufficient Rust checks and independently established fixtures; do not weaken coverage by merely deleting it. External differential checks can inform qualification without becoming a retained project dependency. |

Core's prepared contour/feature IR, document schema, archived original assets,
and Rust renderer do not need redesign merely to change the CAD provider.
That is an architectural inference from the current boundaries, not a proven
zero-change migration. See [current architecture](../../architecture.md).

Complete replacement requires both gates to pass: Rust vendor STEP handling
and Rust generated geometry/export. Start with failure attribution in the old
adapter and the minimal boolean reproducer, then expand to the existing feature
fixtures. If Monstertruck cannot satisfy either gate, investigate a Rust-only
complement or targeted upstream fixes; retaining OCCT or dropping currently
supported features does not satisfy the request. The feasibility and scope of
those fixes remain open research questions.

## Evidence limits

This note records source/API inspection at an exact upstream commit, earlier
BoardStudio spike results, and published OCCT API documentation. It does not
claim the current Monstertruck source passes BoardStudio's corpus, builds to
BoardStudio's WASM target, has no panic paths, preserves all STEP metadata, or
generates geometry equivalent to OCCT. Those need the focused experiment above.
