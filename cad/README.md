# v2 CAD

The CAD provider consumes Rust-prepared case geometry. The core worker handles
`CaseAssemblyIR` through `prepare-case` and returns revision-matched
`PreparedCaseAssemblyIR` regions with contours, holes, cavities, gasket grooves,
and mounts. The native offset engine uses `i_overlay` on a 0.001 mm coordinate
grid and remains the source for contour cleanup, offsets, containment, and
derived regions.

The WASM module exports `build_case`, `build_assembly`, `preview_body`,
`export_cached_assembly`, `build_keycaps` and `read_step_model`; the Rust worker
in `web/src/cad_worker.rs` calls them through the generated `assets/cad-worker/entry.js`.
The module pins Cadrum 0.8.20 and OCCT 8.0.1, uses absolute 0.1 mm linear and
0.5 rad angular tessellation settings, and expands indexed Cadrum meshes into
non-indexed `Float32Array` buffers. STEP imports are limited to 32 MiB and return
millimeter bounds. Operations remain in the dedicated lazy CAD worker; request
IDs, revisions, worker restarts and transferable buffers are owned by the Rust
worker protocol. There is no TypeScript CAD package.

`python3 cad/scripts/test-cadrum.py` builds the native Core preparation driver
and the STEP oracle, runs the native Cadrum tests, then builds the WASM artifact
through the pinned WASI SDK container. The native tests build every case,
assembly and fixture, check them against `cad/test/fixtures/step-expectations.json`
(original assertions, tolerances and the libcascade baseline) and the pinned
source ledgers, and re-read each export with the STEP oracle.

The STEP oracle (`cad/step-oracle`) is test-only and native-only. It runs as a
separate process that reads STEP with OCCT's own `STEPControl_Reader` and checks
`BRepCheck_Analyzer`, volume, optimal bounds, solid count and orphan faces. It is
adapter-independent from the production Cadrum reader (which sews orphan faces)
but **not kernel-independent**: it uses the same OCCT 8.0.1 archive. Independent
kernel evidence comes from the pinned FreeCAD measurements and analytic expectations
in the fixtures. Native tests cannot prove WASM bindings, worker transfer or
cancellation; the Rust worker tests (`python3 scripts/check.py browser`) own transport, and
`python3 cad/scripts/test-cadrum-browser.py` runs the real WASM bindings in headless
Chromium (result shapes, STEP import bounds and rejection, buffer ownership). Run it
after `build-cadrum-wasm.py`.

The first build downloads target-specific OCCT 8.0.1 archives and verifies their
SHA-256 digests before extraction. The WASM build requires Podman or Docker;
the native tests use the verified Linux x86_64 archive. The toolchain pins and
integration evidence are recorded in the
[Cadrum assessment](../docs/research/cad/cadrum-assessment.md).

CAD orchestration uses Python 3.11+ and the standard library. Direct commands:

```sh
python3 cad/scripts/prepare-cadrum-occt.py native
python3 cad/scripts/build-cadrum-wasm.py
python3 cad/scripts/test-cadrum.py
```

Run these from the repository root. `test-cadrum.py` builds the Core preparation driver and
the STEP oracle, runs the native tests and builds WASM.
