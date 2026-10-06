# v2 CAD

The CAD package consumes Rust-prepared case geometry. The core worker handles
`CaseAssemblyIR` through `prepare-case` and returns revision-matched
`PreparedCaseAssemblyIR` regions with contours, holes, cavities, gasket grooves,
and mounts. The native offset engine uses `i_overlay` on a 0.001 mm coordinate
grid and remains the source for contour cleanup, offsets, containment, and
derived regions.

`buildCase(PreparedCaseIR)`, `buildAssembly(PreparedCaseAssemblyIR)`, and
`readStepModel(Uint8Array)` preserve the existing TypeScript API while calling a
locally bundled Rust/WASM bridge. The bridge pins Cadrum 0.8.20 and OCCT 8.0.1,
uses absolute 0.1 mm linear and 0.5 rad angular tessellation settings, and
expands indexed Cadrum meshes into the existing non-indexed `Float32Array`
buffers. STEP imports are limited to 32 MiB and return millimeter bounds.
Operations remain in the dedicated lazy CAD worker; request IDs, revisions,
worker restarts, and transferable buffers are unchanged.

`pnpm --dir cad test` builds the native core preparation driver, runs native
Cadrum bridge tests, builds the WASM artifact through the pinned WASI SDK
container, and runs the 11 case integration fixtures plus transformed
multi-solid STEP import coverage. `libcascade` is a development-only
independent STEP reimport oracle. The browser build uses the same WASM artifact
builder and bundles its glue and binary locally for offline and GitHub Pages
subpath use.

The first build downloads target-specific OCCT 8.0.1 archives and verifies their
SHA-256 digests before extraction. The WASM build requires Podman or Docker;
the native fixture validation uses the verified Linux x86_64 archive. The
toolchain pins and integration evidence are recorded in the
[Cadrum assessment](../docs/cadrum-assessment.md).

CAD orchestration uses Python 3.11+ and the standard library. Direct commands:

```sh
python3 cad/scripts/prepare-cadrum-occt.py native
python3 cad/scripts/build-cadrum-wasm.py
python3 cad/scripts/test-cadrum.py
```

Run these from the repository root. `test-cadrum.py` builds the Core preparation driver, runs native bridge tests
and builds WASM; the independent JavaScript integration oracle remains in
`pnpm --dir cad test` until its separate migration.
