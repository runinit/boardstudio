# Map: Complete CAD backend replacement

## Destination

Replace BoardStudio's entire Cadrum CAD backend with Monstertruck, preserve intended features and accuracy, and remove OCCT/C++ from CAD production, build, packaging and required-test paths. Own the outcome through verified production integration; qualification experiments and individual resolved tickets are progress. Production remains Cadrum until full acceptance is established.

Completion requires the [feature assessment](../../research/cad/monstertruck-feature-assessment-2026-10-16.md) as amended by the dated scope decisions below in current production code, passing required native/WASM/browser checks, comparable performance measurements, and an actual dependency/build/packaged-artifact audit demonstrating CAD no longer requires OCCT/C++. Exact geometry and exact STEP export are required for generated **cases and PCBs only**, built through [013](issues/013-layered-case-construction.md)'s layered 2.5D path. Preview meshes cannot substitute for those exact solids. Keycaps use appearance/optional-print meshes; vendor STEP uses complete source-retaining preview/alignment/collision meshes. Those mesh paths are the approved design. Preserve independent accuracy and strict source completeness; internal STEP roundtrips alone do not prove interoperability. No general exact 3D Boolean engine is required, and OCCT/C++ cannot remain as a final fallback.

Vendor STEP assets retain immutable original bytes, original source geometry/face information and every occurrence, unit conversion and placement; reject incomplete imports. Assume millimetres when units are absent. Repairs use the user-approved vendor-preview allowance `max(2 × authenticated declared precision, 0.02 mm)`; retain declared precision separately. An explicit per-file preview repair budget may exceed declared precision when source constraints are inconsistent: retain declared precision, selected budget and actual repair movement separately, preserve original bytes/source geometry, and reject incomplete or over-budget repair. Basic collision validation in [014](issues/014-vendor-mesh-collision-validation.md) reports intersection, clearance and contact locations with supported tolerance bands by default, refinement on demand, and `0.01 mm` as an explicit tighter check; results without sufficient accuracy remain uncertain.

The [qualification record](../../research/cad/monstertruck-qualification-2026-10-16.md) and [collision research](../../research/cad/vendor-collision-validation-2026-10-17.md) document baseline findings and limitations.

## Scope decisions (2026-10-17, user)

- **Exact geometry:** cases and PCBs only, built via [013](issues/013-layered-case-construction.md): vertical features → authored slice heights → per-band line/arc profiles → direct planes/cylinders/cones → compatible face merge. PCB outline plus drilled holes at thickness is included; cones vary continuously rather than becoming stepped cylinders. Exact STEP export [008](issues/008-step-export-independent-accuracy.md) covers cases and PCBs.
- **Keycaps:** display and optional 3D printing only. Mesh pipeline, with no exact keycap STEP. [004](issues/004-valid-bounded-dish-cutters.md) is off the critical path, and [009](issues/009-complete-keycaps.md) is mesh-only.
- **Vendor STEP:** preview, alignment and basic collision validation ([007](issues/007-strict-step-import.md) / [014](issues/014-vendor-mesh-collision-validation.md)). All 34/34 vendor STEP files import under the approved preview allowance.
- **General 3D Booleans:** no longer on the critical path. [006](issues/006-complete-case-features.md) adopts [013](issues/013-layered-case-construction.md) and closes only demonstrated case/PCB gaps.
- **OCCT/C++ removal ([012](issues/012-final-cutover-remove-cadrum.md)):** unchanged.

## Workflow & Execution Guardrails

Follow the [existing tracker conventions](../../agents/issue-tracker.md) and [AGENTS.md](../../../AGENTS.md):
- Work solo on the critical path by default; delegate at most one bounded agent when it materially unblocks integration.
- Run `code-review` **once per completed ticket**, never on every intermediate commit or lint fix.
- Never create Cargo `target` directories or copy source trees inside `.evidence/`.
- Never generate `PREIMAGES.json`, `POSTIMAGES.json`, `before/` or `after/` file copies, or custom SHA-256 manifest files—Git tracks source state.
- Never pass `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, or `CARGO_PROFILE_DEV_DEBUG=2`.

## Slices & Current Status

- [001: Certify closed STEP surface boundaries and retain trim coverage](issues/001-certify-closed-step-boundaries.md) — `claimed`; closed-chart, collapsed-pole (v49), and partial closed-spline (v50) fixes integrated in vendored crates.
- [002: Enforce shared-edge angular sampling without whole-model regressions](issues/002-preserve-shared-angular-meshing.md) — `claimed`; angular sampling and fallback mesher caching integrated.
- [003: Support contained blind and partial-depth prism cuts](issues/003-contained-blind-prism-cuts.md) — `resolved`.
- [004: Construct valid bounded keycap dish cutters](issues/004-valid-bounded-dish-cutters.md) — `wontfix` (superseded by [009](issues/009-complete-keycaps.md)).
- [005: Production case profiles](issues/005-case-profiles.md) — `resolved`.
- [006: Adopt layered cases and PCBs](issues/006-complete-case-features.md) — `claimed`; layered builder, PCB source producer/consumer, legacy arc holes, `BoardReference` mechanical export, and multi-loop analytic ownership integrated.
- [007: Complete vendor STEP asset import](issues/007-strict-step-import.md) — `claimed`; 34/34 vendor corpus files import (`8206af04a`), retained STEP line view (`c0ffd2ec`), staged provider adapter (`2efb01be`), and worker SHA verification (`8e413cb4`) integrated.
- [008: Exact case/PCB STEP and independent accuracy](issues/008-step-export-independent-accuracy.md) — `claimed`; exact STEP writer and independent raw/analytic accuracy checker integrated.
- [009: Mesh keycaps](issues/009-complete-keycaps.md) — `claimed`; display migration, MX and Alps/ChocV1 closed stems, pure-Rust Manifold printable keycap provider, and STL ZIP delivery integrated.
- [010: Production caches, worker and browser](issues/010-production-cache-worker-browser.md) — `claimed`; generated-case facade staged, Host/Runtime/page/Case WASM gates green (`47e6bb3f`). Remaining: complete `vendorStep` worker transfer/cache/line rendering and switch the active provider registration from Cadrum to Monstertruck.
- [011: Integrated CAD performance](issues/011-integrated-cad-performance.md) — `ready-for-agent` (blocked by [010](issues/010-production-cache-worker-browser.md)).
- [012: Final cutover and removal](issues/012-final-cutover-remove-cadrum.md) — `ready-for-agent` (blocked by [001](issues/001-certify-closed-step-boundaries.md), [002](issues/002-preserve-shared-angular-meshing.md), [010](issues/010-production-cache-worker-browser.md), [011](issues/011-integrated-cad-performance.md)).
- [013: Layered exact cases and PCBs](issues/013-layered-case-construction.md) — `claimed`; layered 2.5D case/PCB builder and internal-gasket inventory integrated in `cad/monstertruck/src/layered/`.
- [014: Mesh collision validation](issues/014-vendor-mesh-collision-validation.md) — `claimed`; blocked by [007](issues/007-strict-step-import.md) worker delivery.
