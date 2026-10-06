# CAD kernel options for v2

Compared 2026-09-24 against Board Studio `b7b7c3d`; decision updated
2026-09-25 against the Cadrum implementation at baseline `fd991be3`. Cadrum is
the production CAD backend for v2. The comparison below remains as context for
future kernel work; it is no longer a candidate-selection plan.

## First separate the choices

There are two decisions here:

1. **Which geometry kernel?** Keep OpenCascade, or replace it with a Rust B-rep
   kernel such as Monstertruck.
2. **Which API and worker boundary?** Continue using the TypeScript
   `libcascade` adapter, or put the case operations behind a Rust API such as
   Cadrum or a modified `opencascade-rs`.

Cadrum and `opencascade-rs` are both Rust interfaces to the C++ OpenCascade
kernel. They move application-side CAD operations toward Rust, but they do not
remove the C++ kernel. Monstertruck replaces that kernel with its own Rust B-rep
implementation. The current app already uses `libcascade` 3.0.2; the prior
package assessment identifies its OCCT version as 8.0.1. The latest official
OCCT release checked for this comparison is also 8.0.1. Therefore, moving from
`libcascade` to another OCCT wrapper is primarily an API, build, and ownership
decision, not an OCCT-version upgrade.

## Options at a glance

| Option | Geometry kernel | What moves to Rust | Browser evidence | Main uncertainty for Board Studio |
| --- | --- | --- | --- | --- |
| Keep `libcascade` 3.0.2 | OCCT 8.0.1 | Nothing new | Kept as a development-only STEP reimport oracle | TypeScript production adapter removed |
| Cadrum 0.8.20 | OCCT 8.0.1 | Case construction, STEP handling, mesh preparation | Production Rust/WASM bridge in the lazy case worker | See [integration assessment](cadrum-assessment.md) for fixture, browser, and measurement evidence |
| Fork/update `opencascade-rs` to OCCT 8.0.1 | Target OCCT 8.0.1 | Same operations through a wrapper we can extend | Upstream repository does not demonstrate an in-browser OCCT worker | Updating bindings/build glue and creating the browser build path |
| Monstertruck | Rust B-rep kernel | Kernel and case operations | Upstream has STEP and wasm/JS crates; Board Studio integration is untested | Boolean and STEP behavior against our case and component corpus |
| `occt-wasm` control | OCCT 8.x | Nothing; TypeScript API | Upstream provides a worker API | It does not advance Rust ownership, and its browser feature baseline needs checking |

The Cadrum row reflects the implemented backend; the other browser and kernel
statements remain candidate capabilities, not measured Board Studio
compatibility. Cadrum’s changelog lists 0.8.20 and records its OCCT 8.0.1 update
in 0.8.17. Its WASM build changes required exact version and toolchain pinning.
Monstertruck 0.4.1 documents
solid booleans, STEP read/write, assemblies, meshing, and a WASM bindings crate;
those features still need testing on our inputs.

## Rust wrapper over OpenCascade

### Cadrum

Cadrum is the production Rust-owned CAD adapter, pinned at 0.8.20 with OCCT
8.0.1. It implements `buildCase`, `buildAssembly`, and `readStepModel` behind the
existing TypeScript and worker contracts. Prepared contours, booleans, STEP
streams, and mesh extraction run through the Rust/WASM bridge. The case worker
stays lazy and Three.js remains the mesh consumer. The 11 former-adapter
fixtures, transformed component import, malformed input, worker restart,
Chromium preview/export, and Pages deployment evidence are recorded in the
[integration assessment](cadrum-assessment.md).

Cadrum still links C++ OpenCascade, so its license and C++ initialization,
binary, and memory costs remain. This adapter migration does not claim a
kernel replacement.

### Modified `opencascade-rs`

`opencascade-rs` provides lower-level CXX bindings and a higher-level Rust API
that its README describes as work in progress. The repository builds against
an OCCT source submodule and also documents using a preinstalled library. A
previously inspected source snapshot used OCCT 7.8.1; targeting 8.0.1 means
updating the source/build setup and resolving any C++ binding compile changes.
OCCT 8.0.1 keeps the 8.0.0p1 API/ABI baseline, but that does not promise
compatibility from 7.8.1 or make the update a version-number-only change.

The supplied Wasmtime example runs the model WASM against native host functions;
it does not demonstrate the OCCT kernel itself running in the browser. An
Emscripten or another C++/Rust WASM build is a separate project. This option is
strongest if we want to **own a small, targeted Rust binding over OCCT and
control exactly which operations are exposed**. It has more build and wrapper
maintenance than adopting Cadrum, with no Board Studio browser recipe yet.

The comparison should pin the upstream commit and OCCT submodule before making
claims about the upgrade cost. The prior 7.8.1 snapshot is recorded in
[`opencascade-rs-assessment.md`](opencascade-rs-assessment.md).

## Replacing OpenCascade

### Monstertruck

Monstertruck is a Rust-native B-rep kernel and is the leading option here if the
goal is to remove the C++ OpenCascade kernel, rather than just write the CAD
adapter in Rust. Its current upstream crate separates modeling, boolean
operations, healing, STEP, and wasm/JS bindings; its license is Apache-2.0.
This could avoid shipping OCCT, but it substitutes a less-established kernel
and STEP implementation for a kernel already validated by our current flows.

Boolean robustness is the primary risk. Monstertruck’s own parity notes say it
reverted an upstream boolean rewrite after regressions on punched and adjacent
cubes. That is useful evidence of active regression work, and a direct reason
to test through-holes, adjacent features, nearly touching walls, and split
regions before investing in its browser integration. Its upstream passing tests
are not a substitute for our 11 case fixtures or vendor STEP corpus.

Monstertruck is the right first **pure-Rust kernel experiment** only if removing
the C++ kernel is a real requirement. If that is not a requirement, its kernel
rewrite adds risk without helping preserve OCCT behavior.

### Other reference points

`occt-wasm` is a TypeScript-first wrapper around OCCT 8.x with a worker API. It
is not a Rust migration candidate, but it is a useful control if we want to
compare a newer OCCT browser API without changing kernels. Its upstream README
claims a roughly 4.5 MB Brotli payload and requires WASM SIMD, tail calls, and
exceptions, so verify those browser requirements and its artifact against our
existing `libcascade` build before considering it.

The earlier [pure-Rust CAD assessment](pure-rust-cad-assessment.md) also covers
brepkit. Keep it as a secondary comparison: the examined version raised both
license-fit questions and boolean fallback behavior that can return mesh-based
results. Refresh that assessment before promoting it to the shortlist.

## Fit to Board Studio

Every serious option needs the same narrow case-worker contract:

- Input: the committed request revision, Rust-prepared contour regions, case
  features and resolved STEP asset bytes.
- Output: structured failure or mesh buffers with positions, normals and the
  current coordinate convention, plus STEP bytes when requested.
- Runtime: a lazy worker, request identity and revision checks, bounded
  cancellation/restart behavior, no server-side CAD service.

Keep Rust `i_overlay` as the contour preparer. Preserve current tessellation
settings, case units, solids and assembly transforms. Convert an indexed mesh if
necessary at the worker boundary; do not change the Three.js API as part of the
kernel comparison.

The retained integration suite in `cad/test/case.test.mjs` (retired; see git history at `cbcfcc5f3`)
contains all 11 original case fixtures plus transformed multi-solid component
STEP import. It checks STEP reimport volume and bounds, mesh validity, component
placement and units, and worker recovery. The development-only `libcascade`
reader remains an independent oracle. Do not compare textual STEP output or
triangle ordering. Add touching/coplanar features and thin-wall fixtures if a
future geometry change creates a specific coverage need.

The Cadrum assessment records five serial measurements against the prior
backend on the same host: Brotli-compressed WASM size, cold first request, warm
case latency, and repeated-operation process memory. These are diagnostics,
while geometry and deployment parity gate cutover. The app's lazy loading keeps
CAD cold-start separate from layout startup.

## Recommendation and decision gates

The options answer different goals:

| Goal | Current choice | Why |
| --- | --- | --- |
| Put case logic in Rust and keep OCCT | Cadrum | Same OCCT 8.0.1 family; production worker bridge and deployment are verified in the assessment |
| Own a narrow binding and C++ build | `opencascade-rs` fork on OCCT 8.0.1 | More control over the Rust boundary, but browser support must be built and proven |
| Remove the C++ kernel | Monstertruck | Rust-native B-rep, boolean, STEP, mesh, and wasm/JS components |
| Keep an independent regression oracle | `libcascade` in devDependencies | Reimports generated STEP during integration tests without becoming a runtime fallback |

Cadrum moves case construction, STEP import/export, and mesh preparation to
Rust while retaining the C++ OCCT kernel. Replacing that kernel remains a
separate project; Monstertruck is still the Rust-native option if that becomes
a requirement. `opencascade-rs` remains a possible future wrapper if Cadrum
needs capabilities its public API does not expose.

## Sources checked

- [Cadrum README](https://github.com/lzpel/cadrum) and
  [changelog](https://github.com/lzpel/cadrum/blob/main/CHANGELOG.md).
- [Cadrum browser example](https://github.com/lzpel/opencascade-wasm32-unknown-unknown-example).
- [`opencascade-rs` source and build notes](https://github.com/bschwind/opencascade-rs).
- [OCCT 8.0.1 release notes](https://github.com/Open-Cascade-SAS/OCCT/releases).
- [Monstertruck source](https://github.com/virtualritz/monstertruck),
  [parity notes](https://github.com/virtualritz/monstertruck/blob/master/TRUCK-PARITY.md),
  and [0.4.1 crate documentation](https://docs.rs/crate/monstertruck/0.4.1).
- [`occt-wasm` source](https://github.com/andymai/occt-wasm).
