# Pure Rust CAD options for v2

> Historical research: measurements, code paths and integration status below
> describe the recorded revision. For current ownership and commands, use
> [architecture](../../architecture.md) and the [development guide](../../../README.md).

Researched 2026-09-24, America/Toronto, for Board Studio HEAD `e43145f` plus the
existing uncommitted v2 work. This evaluates kernels implemented in Rust;
Cadrum and opencascade-rs retain the C++ OpenCascade kernel.

**Recommendation:** test Monstertruck first if eliminating the C++ kernel is
the objective. Include brepkit as a comparison candidate if its current license
fits the project. Cadrum remains the previously researched option for moving
application CAD logic into Rust while retaining OCCT. No replacement kernel has
been adopted or validated against Board Studio.

## Shortlist

| Candidate | Relevant capabilities | Main qualification |
| --- | --- | --- |
| Monstertruck 0.4.1 | B-rep/NURBS, extrusion, booleans, STEP read/write, meshes, assemblies, WASM bindings | Promising prototype; contact cases and component STEP corpus need testing |
| Truck | B-rep/NURBS, modeling, STEP read/write, meshing, WASM wrapper | Explicitly documents unsupported tangent-face booleans |
| brepkit 3.4.18 | Rust/WASM solid modeling, STEP, tessellation, measurement | Difficult booleans can degrade to meshes; current AGPL/commercial licensing |
| csgrs 0.23.0 | Rust curve/mesh CSG, extrusion, booleans, STL/GLB, WASM wrappers | Mesh solids; no STEP route in the inspected format list |
| Fornjot | Experimental Rust B-rep implementation | Development ended 2026-06-19; exclude from a new implementation shortlist |

B-rep describes a solid using connected faces, edges and vertices, with
underlying surfaces such as planes, cylinders and NURBS. A triangle mesh is
useful for previews and printing, but does not preserve that curved CAD
representation. A file ending in `.step` alone would not establish equivalent
geometry or interoperability.

## Monstertruck and Truck

Inspected [Monstertruck](https://github.com/virtualritz/monstertruck/tree/e6520c2b1bde6f46a85649024663583cd03f484c)
at `e6520c2b1bde6f46a85649024663583cd03f484c` (2026-09-19), version 0.4.1,
Apache-2.0. It is a Truck fork with modular modeling, meshing, booleans and
exchange crates. The [STEP implementation documentation](https://docs.rs/crate/monstertruck-io/0.4.1)
describes both reading and writing, including analytic surface handling.

The [boolean API](https://github.com/virtualritz/monstertruck/blob/e6520c2b1bde6f46a85649024663583cd03f484c/monstertruck-solid/src/lib.rs)
exports union, intersection, difference and symmetric difference with typed
errors. Its [WASM crate](https://github.com/virtualritz/monstertruck/blob/e6520c2b1bde6f46a85649024663583cd03f484c/monstertruck-wasm/Cargo.toml)
depends on modeling, booleans, meshing and STEP. A
[CI job](https://github.com/virtualritz/monstertruck/blob/e6520c2b1bde6f46a85649024663583cd03f484c/.github/workflows/ci.yml)
builds the WASM target. This is source/build-configuration evidence, not a
browser run performed in this research.

Use only the modeling, meshing, solid and STEP features for an initial worker;
the existing Three.js renderer does not require adopting its GPU renderer.
Do not disable the solid crate's default boolean backend indiscriminately:
without it, the inspected implementation returns `ShapeOpsError::NoBackend`.

The fork's [change notes](https://github.com/virtualritz/monstertruck/blob/e6520c2b1bde6f46a85649024663583cd03f484c/README.md)
describe reverting a boolean rewrite after it broke closed-shell results for
punched and adjacent cubes. This is evidence of relevant regression work, not
proof that all coplanar or tangent cases are solved.

Inspected upstream [Truck](https://github.com/ricosjp/truck/tree/8d03d8f7900d6aaff784d02092bf5126c1425749)
at `8d03d8f7900d6aaff784d02092bf5126c1425749` (2026-09-07), Apache-2.0. Its
[shape operation source](https://github.com/ricosjp/truck/blob/8d03d8f7900d6aaff784d02092bf5126c1425749/truck-shapeops/src/lib.rs)
explicitly limits booleans to transversal face intersections and excludes
tangent faces. Its [STEP package](https://github.com/ricosjp/truck/blob/8d03d8f7900d6aaff784d02092bf5126c1425749/truck-stepio/README.md)
provides import/export. Truck is a valid baseline and source of reusable crates,
but that boolean limitation is directly relevant to touching case features.

## brepkit

Inspected [brepkit](https://github.com/andymai/brepkit/tree/f7ca96016abdba93523c30b1b2feca41534fb024)
at `f7ca96016abdba93523c30b1b2feca41534fb024` (2026-09-23). Its manifest declares
3.4.18. Upstream offers Rust operation/I/O crates and a wasm-bindgen interface,
with STEP, extrusion, booleans, tessellation and measurement. Assemblies are
marked beta in the inspected feature table.

The consequential limit is in the
[boolean implementation](https://github.com/andymai/brepkit/blob/f7ca96016abdba93523c30b1b2feca41534fb024/crates/operations/src/boolean/mod.rs):
when the main algorithm fails or produces invalid results, it can use a mesh
fallback that loses analytic surfaces and does not guarantee a watertight
result. A public `mesh_fallback_count()` exposes fallback use. A prototype
should monitor it for serial operations and reject degraded manufacturing
exports; successful function return alone is insufficient.

Its [current license statement](https://github.com/andymai/brepkit/blob/f7ca96016abdba93523c30b1b2feca41534fb024/COMMERCIAL-LICENSE.md)
offers AGPL-3.0-only or a commercial license. It states versions through 2.129.x
retain MIT OR Apache-2.0 terms. Those older versions were not evaluated here;
do not transfer current feature or bug-fix claims to them.

The [stability policy](https://github.com/andymai/brepkit/blob/f7ca96016abdba93523c30b1b2feca41534fb024/STABILITY.md)
warns that Rust APIs and geometric behavior continue to change. Its prose still
mentions the older 2.x version line, so use the pinned manifest for version
identity. This candidate needs explicit STEP corpus coverage and exact version
pinning, just as Monstertruck does.

## Mesh modeling and inactive projects

Inspected [csgrs](https://github.com/timschmidt/csgrs/tree/4e5b9ebb59127a559ecda9779d1b8af05ed40fcb)
at `4e5b9ebb59127a559ecda9779d1b8af05ed40fcb` (2026-09-17), version 0.23.0,
MIT. Its [current architecture and format list](https://github.com/timschmidt/csgrs/blob/4e5b9ebb59127a559ecda9779d1b8af05ed40fcb/readme.md)
describe curve modeling through Hypercurve and triangle solids through
Hypermesh, with STL/GLB and other mesh/planar formats. STEP is absent from that
list. This could serve a mesh-output path but is not a demonstrated replacement
for our component STEP imports and CAD exports. Older descriptions of a
self-contained BSP-based csgrs do not describe this inspected version.

Fornjot's author [announced its shutdown](https://www.fornjot.app/blog/shutting-down-fornjot/)
on 2026-06-19. It remains useful research material, but should not be presented
as an actively developed option for a new dependency.

## Application-specific experiment

Use the existing CAD fixture set (`cad/test/case.test.mjs`, retired; see git history at `cbcfcc5f3`, now native tests in `cad/wasm/src/model/construction/validation_tests.rs`) as the starting
comparison: holed plates, concave contours, tray/lid cavities, mounting geometry,
gasket grooves and multiple bodies. Add touching/coplanar faces, concentric
boss/hole cylinders, very thin walls and split contours. Check closed oriented
solids, volume/bounds, curved-surface preservation, and STEP reimport through
the existing OCCT adapter. Test component STEP files separately: generating our
own controlled cases and reading arbitrary vendor models are different tests.

Much of the case builder could also use a narrower pure-Rust route: prepare
planar regions and extrude/tessellate them for preview or mesh export. That
could reduce reliance on general 3D booleans, but does not solve STEP component
import or establish an analytic STEP writer. It is a scope option, not a new
geometry algorithm validated here.

Keep the CAD module separate from the interactive core. Compare WASM download,
initialization, memory and request latency on the actual fixtures. A mixed
transition could use Rust for generated cases and keep OCCT for STEP imports;
that would still retain a C++ dependency until the importer is replaced.

## Evidence limits

This is a source and documentation assessment. Context7 resolved Truck and
csgrs; its Truck examples were unversioned and csgrs had no matching STEP answer,
so pinned upstream sources control the conclusions. It did not resolve brepkit.
No candidate was compiled, run in a browser, benchmarked or exercised against
Board Studio's fixtures. Upstream feature labels and performance claims are not
independent validation. No production code or dependencies were changed.
