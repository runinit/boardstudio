# opencascade-rs assessment for v2

> Historical research: measurements, code paths and integration status below
> describe the recorded revision. For current ownership and commands, use
> [architecture](../../architecture.md) and the [development guide](../../../README.md).

Researched 2026-09-24, America/Toronto, against upstream commit
[`32758df23a137f4e7c786c618b0317fc3badb3b2`](https://github.com/bschwind/opencascade-rs/tree/32758df23a137f4e7c786c618b0317fc3badb3b2),
dated 2026-08-24. The inspected `opencascade` package declares version 0.3.0.
Local baseline is Board Studio HEAD `e43145f` with existing uncommitted changes.

**Assessment:** a credible candidate for a larger Rust CAD layer, subject to a
browser feasibility prototype. It exposes much of the modeling functionality we
need. Its documented WASM example does not demonstrate running the CAD kernel
in a browser. Source inspection alone cannot establish that it will replace the
current browser worker successfully.

Follow-up: the [Cadrum assessment](cadrum-assessment.md) identifies a stronger
first candidate for our browser prototype, with a working related demo,
`wasm32-unknown-unknown` build recipe and stream-based STEP APIs. The analysis
below remains useful as an alternative; the proposed Emscripten experiment is
no longer the preferred first route.

“Contour preparation” means producing the 2D outlines for case clearance,
walls, cavities, holes, and gasket grooves. `opencascade-rs` could cover the
larger step after that: constructing the 3D solids and producing STEP and mesh
outputs from Rust. It wraps the C++ OpenCascade kernel through CXX; adopting it
would move application code to Rust while retaining that kernel.

## Fit with the current CAD package

The current `cad/src/index.ts` (retired; see git history at `cbcfcc5f3`) uses libcascade 3.0.2 for
solids, STEP, and meshing, plus ClipperLib for planar offsets.

| Current need | Inspected Rust API | Remaining integration work |
| --- | --- | --- |
| Polygon plate, tray and lid solids | `Wire::from_ordered_points`, `Face::from_wire_with_holes`, `Face::extrude` | Preserve orientation, elevations, cavities and validation |
| Mount holes and bosses | `Shape::cylinder`, boolean operations | Preserve dimensions and report failed operations |
| Multi-body assembly | `Compound::from_shapes`, STEP writers | Preserve relative body positions and one export result |
| STEP component import/export | `Shape::read_step`, `write_step`, `write_all_step` | Adapt path-based APIs to browser bytes and temporary storage |
| Mesh preview | `Mesh` vertices, normals, indices | Convert to existing transferable buffers and preserve tolerances |
| Future edge finishing | Fillet and chamfer methods | Separate feature work and geometric validation |

These are implemented APIs, not merely items from the README's goals. See the
pinned sources for [faces](https://github.com/bschwind/opencascade-rs/blob/32758df23a137f4e7c786c618b0317fc3badb3b2/crates/opencascade/src/primitives/face.rs),
[shapes and STEP](https://github.com/bschwind/opencascade-rs/blob/32758df23a137f4e7c786c618b0317fc3badb3b2/crates/opencascade/src/primitives/shape.rs),
[compounds](https://github.com/bschwind/opencascade-rs/blob/32758df23a137f4e7c786c618b0317fc3badb3b2/crates/opencascade/src/primitives/compound.rs),
and [meshing](https://github.com/bschwind/opencascade-rs/blob/32758df23a137f4e7c786c618b0317fc3badb3b2/crates/opencascade/src/mesh.rs).

The mapping is promising, but not exact. Our mesher specifies both 0.1 mm linear
deflection and 0.5 radians angular deflection; the inspected high-level Rust
mesher accepts one tolerance. Its source also leaves transforming normals by
the shape location as a TODO. Rotated imported STEP assemblies need a regression
case before relying on that mesh output. Several construction methods return
shapes directly, so preserve our explicit operation checks and worker recovery
rather than assuming every invalid model becomes a recoverable Rust error.

## What upstream WASM currently demonstrates

The supplied example has this execution flow:

```text
Rust model program compiled to WASM
  -> Wasmtime inside the native viewer
  -> host functions implemented with opencascade-rs
  -> native C++ OpenCascade
```

The [example instructions](https://github.com/bschwind/opencascade-rs/blob/32758df23a137f4e7c786c618b0317fc3badb3b2/crates/wasm-example/README.md)
launch a Cargo viewer executable. The
[viewer implementation](https://github.com/bschwind/opencascade-rs/blob/32758df23a137f4e7c786c618b0317fc3badb3b2/crates/viewer/src/wasm_engine.rs)
implements Wasmtime component host resources using native OpenCascade objects.
This is useful for loading model programs, but differs from our offline browser
application executing the entire CAD kernel in a Web Worker.

There is also evidence of Emscripten work. Upstream merged
[PR 161](https://github.com/bschwind/opencascade-rs/pull/161) in January 2024 to fix
OCCT package discovery during WASM builds; that change remains in the pinned
CMake source. The maintainer's October 2023
[browser-support reply](https://github.com/bschwind/opencascade-rs/issues/143#issuecomment-1769813584)
predates that fix and must not be treated as proof of present impossibility.
The inspected [test workflow](https://github.com/bschwind/opencascade-rs/blob/32758df23a137f4e7c786c618b0317fc3badb3b2/.github/workflows/test.yml)
runs host Cargo tests on Ubuntu; it does not demonstrate browser-kernel execution.

The fair conclusion is **browser integration is unverified for our use**, not
that Rust/OpenCascade cannot run in browsers.

## Proposed browser experiment

Investigate an isolated CAD worker artifact built with Rust, CXX, and OCCT using
Emscripten. Retain the existing `wasm32-unknown-unknown` core artifact and pass
case requests/results across the worker boundary. Avoid adding OCCT to the
interactive layout core's initial download.

Rust's [`wasm32-unknown-emscripten` target](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-emscripten.html)
provides C/C++ interoperability. Its documentation requires compatible Rust,
Emscripten, and standard-library ABIs, including exception settings. This is a
plausible build route, not a verified build recipe for this particular crate.
Expect to pin the toolchains and establish worker initialization, filesystem or
stream adapters, memory ownership, and error conversion. Some JavaScript loader
and browser messaging code would remain.

The bundled OCCT submodule points to
[`bd2a789`](https://github.com/Open-Cascade-SAS/OCCT/blob/bd2a789f15235755ce4d1a3b07379a2e062fdc2e/src/Standard/Standard_Version.hxx),
which declares 7.8.1. Our installed libcascade README reports OCCT 8.0.1.
Moving to the Rust wrapper would therefore also change the kernel version
unless newer-OCCT binding compatibility is established separately. Neither
smaller binaries nor equivalent geometry should be assumed from the language
change.

The prototype should meet these gates before implementation migration:

1. Build the pinned wrapper and kernel; load them in a browser worker without
   a server-side CAD service or native helper.
2. Extrude a plate with a hole, cut a cavity, and combine two bodies. Return
   valid mesh buffers and STEP bytes through a small Rust-owned case API.
3. Reimport the exported STEP, check solid count, volume and bounds, then import
   a representative rotated component STEP and verify mesh normals.
4. Reproduce the seven existing CAD fixtures, including concave insets, mounts,
   gasket geometry, and multi-body assembly. Keep the current planar-offset
   semantics; `Wire::offset` returning one wire does not establish support for
   a cavity that splits into multiple regions.
5. Measure download size, cold initialization, peak/repeated-operation memory,
   and case latency; verify errors, worker restart, stale revisions, offline
   reload, and model/asset URLs under a subdirectory.

If these pass, port `buildCase`, `buildAssembly`, and `readStepModel` behind the
existing app-facing boundary, keeping Three.js as the mesh consumer. Rust
`i_overlay` can still own the 2D preparation. This would move substantially more
CAD logic than the original contour-only proposal and could remove libcascade
from v2. Replacing that dependency does not itself eliminate React or Vite.

If the browser build requires too much ongoing port maintenance, retain the
current kernel adapter while moving case preparation into Rust. A native
desktop/CLI backend is another possible use of opencascade-rs, but changes the
deployment scope and is not the proposal for the current browser application.

## Validation performed

Inspected pinned upstream manifests, CXX/CMake build paths, modeling and I/O
methods, WASM example/host implementation, CI, and relevant issue/PR history.
Context7 did not resolve this project, so the assessment uses upstream source
and official Rust target documentation.

No native or browser build of opencascade-rs was attempted. `emcc` is not on the
current PATH, and the installed Rust targets do not include Emscripten. No
toolchains or project dependencies were installed. Prior CAD tests validate the
existing adapter only; they provide no runtime evidence for this candidate.
