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
`query_collision_mesh_pair` answers vendor mesh collision and clearance queries
with `parry3d-f64`.

The kernel is `boardstudio-cad` (`cad/monstertruck`), a pure-Rust crate over the
vendored Monstertruck 0.4.1 crates in `cad/monstertruck-qualification/vendor`.
It builds layered 2.5D cases and source-backed PCBs, writes exact STEP, meshes
profiles into non-indexed `Float32Array` buffers and imports vendor STEP with
surface and retained wireframe lines. STEP imports are limited to 32 MiB and
return millimeter bounds. Operations remain in the dedicated lazy CAD worker;
request IDs, revisions, worker restarts and transferable buffers are owned by the
Rust worker protocol. There is no TypeScript CAD package and no C or C++
toolchain: `cad/wasm` builds for `wasm32-unknown-unknown` with `wasm-pack`.

`python3 cad/scripts/test-cad.py` runs the native `boardstudio-cad` tests
(including the independent STEP accuracy checker in `src/step_accuracy/`, checked
against `cad/test/fixtures/step-expectations.json` and the pinned FreeCAD
measurements) and the native `cad/wasm` provider tests. Native tests cannot prove
WASM bindings, worker transfer or cancellation; the Rust worker tests
(`python3 scripts/check.py browser`) own transport, and
`python3 cad/scripts/test-cad-browser.py` runs the real WASM bindings in headless
Chromium through ChromeDriver (result shapes, STEP import bounds and rejection,
buffer ownership). Chrome and ChromeDriver must be installed; set `CHROME` or
`CHROMEDRIVER` when their binaries are not on `PATH`. Run it after
`build-cad-wasm.py`. Its 180-second default bounds browser execution; owned
browser cleanup has a separate, shared four-second maximum grace period.

CAD orchestration uses Python 3.11+ and the standard library. Direct commands:

```sh
python3 cad/scripts/build-cad-wasm.py
python3 cad/scripts/test-cad.py
python3 cad/scripts/test-cad-browser.py
```

Run these from the repository root. `build-cad-wasm.py` skips the build when the
hashed inputs match the last build.
