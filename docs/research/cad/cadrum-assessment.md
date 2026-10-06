# Cadrum integration assessment

> Historical research: measurements, code paths and integration status below
> describe the recorded revision. For current ownership and commands, use
> [architecture](../../architecture.md) and the [development guide](../../../README.md).

Updated 2026-09-25 after the production bridge and acceptance checks. The
production CAD backend is Cadrum 0.8.20 at
[`8788df70c60b986b5ab387edb75a2f6f341a8c7a`](https://github.com/lzpel/cadrum/tree/8788df70c60b986b5ab387edb75a2f6f341a8c7a),
using OpenCascade 8.0.1. This moves case construction, STEP handling, and mesh
preparation to Rust/WASM. Cadrum still uses the C++ OpenCascade kernel.

## Integration boundary

The existing TypeScript exports and worker protocol remain in place:

```text
case.worker.ts -> cad/src/index.ts -> lazy local Cadrum WASM
                                  -> STEP bytes + mesh buffers
                                  -> existing export and Three.js preview
```

The WASM is loaded only when the case worker first requests CAD. Vite bundles
the glue and WASM locally, including under the Pages `/boardstudio/` base path;
there is no CDN dependency or runtime fallback. Cadrum MIT and OpenCascade LGPL
with exception notices are shipped in the app's local `licenses` directory.
The existing `CaseResult` and `StepModel` shapes, request IDs, revisions, queue,
worker replacement, and transferable-buffer contracts remain unchanged.
Cadrum indexed meshes are expanded to the current
non-indexed `Float32Array` positions and normals. Coordinates remain in
millimeters and imported model bounds are retained.

The bridge builds prepared contours, holes, cavities, gasket grooves, mounting
features, openings, disconnected regions, and multi-body assemblies. It writes
and reads STEP through Cadrum streams, with a 32 MiB input limit and finite
mesh/bounds checks. Tessellation uses absolute 0.1 mm linear and 0.5 rad angular
deflection. The bridge invokes both `cadrum::__anchor_wasi_stub()` and
`__wasm_call_ctors()` at WASM startup; the latter is required to initialize
OpenCascade globals in this build.

## Pinned and reproducible build

- Cadrum is pinned to `=0.8.20` in Cargo, with `Cargo.lock` checked in.
- Cadrum's OCCT 8.0.1 `rev2` WASM and native archives are downloaded by
  `cad/scripts/prepare-cadrum-occt.py` and checked against SHA-256 before
  extraction. The WASM archive digest is
  `8149e781acdbd21507cfb29937e48e5fdbe628ee5c37007f720dcf5d4000e6ad`; the
  Linux x86_64 native archive digest is
  `95e068936c0cb4ba2668707c0dca209d3396103dfc1db85eb72d192badfb4143`.
- The builder image is pinned by OCI digest
  `sha256:737535e2e75a90303fb2b9783c03002c1b8623264e2c853297e9a965a6b6d795`.
  It verifies wasi-sdk 33 source commit
  `c10c0507deb3e5aad506f1f9f32084e49a21834b`, builds its exception-enabled
  sysroot, and pins Rust 1.98.0, wasm-pack 0.15.0, and wasm-bindgen-cli 0.2.128.
- `pnpm run build:cad` performs the same locked build used by validation and
  Pages workflows. The generated glue and WASM are local build outputs, not
  checked-in binaries.

## Geometry and worker evidence

All 11 integration fixtures from the previous adapter pass through the
optimized Cadrum WASM artifact. They cover STEP roundtrips, cutouts, concave
regions, tray/lid cavities, mounts, gasket grooves, multi-body assemblies,
battery-stack geometry, integrated frames, and component-local openings. A
twelfth test imports the transformed, multi-solid TrackPoint STEP component.
The suite compares reimported solids, volume, bounds, mesh validity, placement,
units, and normals rather than STEP text or triangle order. The independent
`libcascade` reader remains in `cad` development dependencies as a STEP
reimport oracle; it is not imported by the production adapter and is not a
runtime fallback.

Native bridge tests passed for a holed plate and the 63-solid transformed
component. The component bounds and mesh are verified in millimeters. The
malformed-input test rejects invalid STEP data. The client worker failure test
verifies that pending work rejects after worker failure, the worker is replaced,
and a later request succeeds.

## Performance sample

Five fresh serial Node processes each measured one first plate-with-hole CAD
request followed by five warm requests. Cold time includes the lazy module load
and WASM initialization, but excludes contour preparation. Warm latency is the
median of each process's five requests, then the median across processes.
Memory figures are process RSS; retained delta compares RSS after the first
result is collected with RSS after five more requests and collections. WASM
sizes use Brotli quality 11. These are local diagnostics, not acceptance
thresholds.

| Measurement | Previous `libcascade` adapter | Cadrum 0.8.20 bridge |
| --- | ---: | ---: |
| WASM raw | 42,691,285 B | 12,821,079 B |
| WASM Brotli q11 | 8,558,254 B | 3,025,027 B |
| Glue Brotli q11 | 23,195 B | 4,253 B |
| Combined compressed artifact | 8,581,449 B | 3,029,280 B |
| Median cold first request | 677.6 ms | 135.5 ms |
| Median warm request | 12.42 ms | 9.27 ms |
| Median peak process RSS | 485.7 MiB | 236.2 MiB |
| Median retained RSS delta after five warm requests | -21.6 MiB | +47.7 MiB |

The Cadrum sample is smaller and faster for this fixture. Its positive retained
RSS delta means memory grew during the measured sequence, so repeat-operation
memory should continue to be monitored in browser workloads. Neither the
improvement nor that RSS sample replaces geometry and deployment checks.

## Cutover checks

The cutover checks passed:

- `pnpm test`: core, Ergogen, KiCad, CAD, and app test suites passed. This
  includes 12 CAD integration tests and 101 app unit tests.
- `pnpm run build`: production TypeScript and Vite build passed with the local
  Cadrum WASM and bundled STEP models.
- `BOARDSTUDIO_CHROMIUM=/usr/bin/chromium pnpm run test:e2e`: all 115 Chromium
  tests passed, including lazy CAD loading, preview, and STEP export.
- `BOARDSTUDIO_CHROMIUM=/usr/bin/chromium pnpm run test:e2e:pages`: the
  `/boardstudio/` deployment test passed, including Cadrum WASM path, bundled
  models, service-worker scope, offline reload, preview, and STEP export.
- The production-build WASM and subsequent validation-build WASM have the same
  SHA-256 (`6386c8222f7c01a149827674109884fcb8dad00e8730a60ad7ddcd4cc9085e59`);
  the generated JS glue hash also matched
  (`b958ed15042d49aabab2b93fab96cd489c34bff70f41f775dc2b645ecd47d8ed`). Both
  workflows invoke `pnpm run build:cad`, using the same pinned,
  checksum-verified build path.

The approach preserves OpenCascade and its C++ runtime. Removing that kernel
would require a separate kernel migration, such as a Rust-native B-rep
implementation, and is outside this adapter replacement.

## Upstream references

- [Pinned Cadrum source](https://github.com/lzpel/cadrum/tree/8788df70c60b986b5ab387edb75a2f6f341a8c7a)
- [Cadrum STEP stream API](https://github.com/lzpel/cadrum/blob/8788df70c60b986b5ab387edb75a2f6f341a8c7a/src/lib.rs)
- [Cadrum tessellation settings](https://github.com/lzpel/cadrum/blob/8788df70c60b986b5ab387edb75a2f6f341a8c7a/src/traits.rs)
- [Cadrum WASI stub](https://github.com/lzpel/cadrum/blob/8788df70c60b986b5ab387edb75a2f6f341a8c7a/src/wasi_stub.rs)
