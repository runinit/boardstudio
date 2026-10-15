# CAD build time: what to skip and what could replace OCCT

Researched 2026-10-14 against the current BoardStudio checkout. The target is
shorter developer builds for the offline browser CAD worker, while preserving
the existing STEP import/export and generated case behavior. This is a source
audit; no build timing or candidate kernel build was run.

## Short answer

The biggest low-risk opportunity was in the checked build: `check.py build`
calls `build-web.py --fast`, but the fast setting did not reach the CAD
provider. The CAD helper ran a release `wasm-pack` build with the `wasm-opt`
pass enabled. The checked-build flag now reaches that helper, which passes
`--no-opt`; production builds remain optimized. `wasm-pack` 0.15 documents
`--no-opt` as the way to skip `wasm-opt`
([options](https://docs.rs/wasm-pack/0.15.0/wasm_pack/command/build/struct.BuildOptions.html)).

Replacing the kernel is possible, but there is no drop-in pure-Rust switch in
the pinned Cadrum version. The realistic choices are to accelerate the
existing build, narrow CAD product capability to mesh-only output, or fund a
kernel migration with separate STEP and geometry acceptance work.

## What the current build compiles

`scripts/check.py build` invokes `scripts/build-web.py --fast`. That script adds
`--no-opt` only to commands that directly start with `wasm-pack`; the CAD
provider is a Python command and is left out. [Build command list](../../../scripts/check.py),
[provider command construction](../../../scripts/build-web.py), and the
[CAD build helper](../../../cad/scripts/cadrum_build.py) show this path.

The CAD helper runs `wasm-pack build ... --release --locked`, and only skips the
command when the output exists and its input digest matches. The digest
previously included every file under `cad/wasm/src`, including native-only
tests, so editing a test-only file could invalidate the provider artifact
cache. It now excludes those files from the digest and includes the optimizer
mode in the cache key so an unoptimized checked artifact cannot stand in for a
production artifact. These source changes have not yet been build-verified.

The normal build does **not** compile all of OpenCascade from source. The
project downloads and checksum-verifies a prebuilt OCCT archive, then passes
`OCCT_ROOT` into Cadrum. Cadrum compiles its C++ bridge and links the prepared
static OCCT libraries ([archive preparation and container build](../../../cad/scripts/cadrum_build.py),
[pinned Cadrum build script](https://github.com/lzpel/cadrum/blob/8788df70c60b986b5ab387edb75a2f6f341a8c7a/build.rs)).
The separate `step-oracle` crate is native and test-only; it is not in the
browser provider build ([CAD README](../../../cad/README.md)).

There are two cold-build exceptions to distinguish from ordinary source edits:

- If the pinned builder image is missing or cannot be pulled, the helper falls
  back to building it locally. Its Containerfile builds the WASI SDK from
  source, which is a substantial one-time toolchain build ([fallback](../../../cad/scripts/cadrum_build.py),
  [Containerfile](../../../cad/wasm/Containerfile)).
- If the CAD source changes, Cargo still uses `--release`. Rust's release
  profile defaults to optimization level 3; Cargo documents that higher levels
  can increase compiler time. After disabling `wasm-opt` in checked builds, a
  separate lower-optimization development profile is a second experiment if
  Rust compilation remains slow ([Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)).
  Such a build can make CAD operations slower and should not replace the shipped
  profile.

## What is already excluded

The CAD manifest already sets `default-features = false` for Cadrum. That turns
off its optional color and PNG features. The `pure` feature in Cadrum 0.8.20 is
not a usable alternative: the pinned source says the pure-Rust backend is not
implemented and leaves the module disabled ([manifest](../../../cad/wasm/Cargo.toml),
[pinned source](https://github.com/lzpel/cadrum/blob/8788df70c60b986b5ab387edb75a2f6f341a8c7a/src/lib.rs)).
Enabling that feature is expected to fail to compile, not remove OCCT.

BoardStudio uses the CAD worker for solids, boolean cuts, tessellation, STEP
read/write, and keycap geometry ([WASM entry points](../../../cad/wasm/src/model.rs),
[case construction](../../../cad/wasm/src/model/construction.rs),
[keycaps](../../../cad/wasm/src/model/keycaps.rs)). Removing those Cadrum
operations can trim the bridge or linked OCCT toolkits only if the public API
and product capability are also narrowed. There is no Cargo feature in this
project that independently disables just one of those operations.

## Replacement choices

| Choice | Build effect | Capability cost and evidence | Assessment |
| --- | --- | --- | --- |
| Keep Cadrum/OCCT; skip `wasm-opt` in checked builds | Removes an unnecessary optimizer pass from routine builds; keeps the same Rust release compilation | Same production API and geometry; checked artifact is less optimized | Do this first and time it |
| Mesh-only kernel such as Manifold | Could remove C++/OCCT from a preview or mesh-only provider | Manifold's source defines its booleans over triangle meshes and guarantees manifold output for manifold inputs; it does not preserve exact curved B-rep faces or provide the current OCCT STEP workflow ([upstream algorithm](https://github.com/elalish/manifold/wiki/Manifold-Library)) | Viable only if STEP/exact-solid behavior can be dropped or retained in a separate slow path; running both kernels in one artifact will not reduce builds |
| Rust B-rep kernel such as Monstertruck | Could remove the C++ compiler and OCCT archive from the CAD provider; compile time may instead move to a multi-crate Rust geometry stack | Upstream provides STEP, booleans, meshing, and WASM crates, but no BoardStudio parity or compile-time comparison follows from those features ([upstream](https://github.com/virtualritz/monstertruck), [parity notes](https://github.com/virtualritz/monstertruck/blob/master/TRUCK-PARITY.md)) | Prototype candidate, not a safe replacement today |
| Move exact CAD to a native/server worker | Removes OCCT from browser WASM compilation and download | Adds a service or native installation and breaks offline CAD unless a local native host is supplied | Architectural choice, not a build flag |

The prior BoardStudio comparison gives a concrete warning for the Rust-kernel
route: its pinned Monstertruck 0.4.1 spike was promising on BoardStudio-generated
STEP files but did not pass the inspected arbitrary vendor STEP corpus as a
drop-in replacement. See [the existing candidate assessment](cad-kernel-options.md)
for the exact corpus and gates. It did not measure current Monstertruck compile
time, so it cannot establish that a Rust rewrite would build faster.

## Recommendation

1. Done in the build scripts: propagate `--fast` to the CAD helper so routine
   builds add `--no-opt` while keeping `--release`; production builds remain
   optimized. The optimizer mode is part of the artifact cache key.
2. Done in the build script: exclude native-only CAD test files from the
   provider artifact digest.
3. Compare a clean and an incremental build with stage timings before changing
   Rust optimization levels. If the container fallback is rebuilding the
   toolchain, restore use of the pinned published image.
4. Only prototype a replacement if the product can accept mesh-only CAD or if
   removing OCCT is itself worth a full STEP/geometry compatibility project.

The larger kernel experiment should separately check generated cases and
arbitrary imported components. Keep the current OCCT path as the comparison
oracle, and measure the candidate's compile time, release WASM size, fixture
geometry, STEP round-trips, cold startup, and repeated-operation memory before
considering a cutover.
