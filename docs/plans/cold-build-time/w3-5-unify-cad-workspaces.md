# W3-5: Unify `cad/monstertruck` & `cad/monstertruck-qualification` Workspaces + CAD Debt

**Status:** Implemented, verified & benchmarked  
**Branch:** `perf/w3-5-unify-cad-workspaces` (from `perf/cold-build-cpu` at `d22c9b9b3`)  
**Parent handoff:** Wave 3 Handoff Section W3-5; extends [Issue 20](issues/20-merge-cad-workspaces.md)

---

## 1. Root Causes Identified

1. **`cgmath` source divergence:** `cad/monstertruck/Cargo.toml` patched `cgmath` to `../../renderer/vendor/cgmath-next-0.18.1`, while `cad/monstertruck-qualification/Cargo.toml` resolved registry `cgmath 0.18.0`.
2. **`[profile.cad-test]` divergence:** `cad/monstertruck` used `opt-level = 1` + 13 per-package `opt-level = 2` overrides, while `cad/monstertruck-qualification` used `opt-level = 0`.
3. **Resolver & two-invocation feature-unification split:** `cad/monstertruck` had no `resolver` key (implicit v1) while `cad/monstertruck-qualification` used `resolver = "3"`. Running production tests and owner tests as two separate `cargo test` commands also caused Cargo feature-unification drift (`libc`, `serde_derive`, `serde`, `cgmath`), compiling all 6 `monstertruck_*`/`step_p21` crates twice even in a shared workspace.
4. **`vtkio 0.6.3` CVEs:** Once `cad/Cargo.lock` brought qualification/vendor crates under `scripts/security-audit.py`, `vtkio 0.6.3` pulled `quick-xml 0.22.0`, failing `check.py security` with 2 high-severity advisories (`RUSTSEC-2026-0194`, `RUSTSEC-2026-0195`), plus `syn 1.0.109`, `nom 3.2.1`, and `num-derive 0.3.3`.

---

## 2. Implemented Architecture

### Unified Virtual Workspace (`cad/Cargo.toml`)
- Created virtual workspace root [`cad/Cargo.toml`](../../../cad/Cargo.toml) (`resolver = "3"`) with all 8 members (`monstertruck`, `wasm`, `monstertruck-qualification`, and the 5 vendored crates `monstertruck-io`, `monstertruck-meshing`, `monstertruck-mesh`, `monstertruck-traits`, `monstertruck-solid`) and explicit `exclude` for the 5 non-member vendor crates (`inflector`, `monstertruck-core`, `monstertruck-derive`, `monstertruck-geometry`, `monstertruck-modeling`).
- Consolidated `[patch.crates-io]` (all 11 entries, including the reviewed `cgmath-next` fork and `monstertruck-solid`) and `[profile.cad-test]` into `cad/Cargo.toml`:
  - Base `opt-level = 1`, `debug = false`, `incremental = false`.
  - Explicit `opt-level = 0` on heavy dev-only crates (`clap`, `clap_builder`, `image`, `moxcms`, `pxfm`, `png`, `proptest`, `monstertruck-fillet`, `monstertruck-healing`, `flate2`, `zlib-rs`, `toml`, `regex-syntax`, `tempfile`, `miniz_oxide`, `fdeflate`).
  - `opt-level = 2` on leaf geometry/math crates (`monstertruck-modeling`, `monstertruck-geometry`, `monstertruck-mesh`, `monstertruck-core`, `manifold-rust`, `parry3d-f64`, `cavalier_contours`, `spade`, `glam`, `wide`, `lyon_tessellation`).
- Converted `cad/monstertruck/Cargo.toml`, `cad/wasm/Cargo.toml`, and `cad/monstertruck-qualification/Cargo.toml` to clean member manifests; replaced both old lockfiles with single unified [`cad/Cargo.lock`](../../../cad/Cargo.lock).
- Updated root [`Cargo.toml`](../../../Cargo.toml) `exclude` to `"cad"`.

### `vtkio 0.6.3` Removal
- Removed unused `vtkio` dev-dependency from `cad/monstertruck-qualification/vendor/monstertruck-io/Cargo.toml` and optional `vtkio` dependency from `cad/monstertruck-qualification/vendor/monstertruck-meshing/Cargo.toml` (keeping `vtk = []` so `#[cfg(feature = "vtk")]` emits zero warnings).
- Eliminated `quick-xml 0.22.0` (`RUSTSEC-2026-0194`, `RUSTSEC-2026-0195`), `syn 1.0.109`, `nom 3.2.1`, and `num-derive 0.3.3` from `cad/Cargo.lock`.

### Single-Invocation Prebuild & `cad/wasm` Isolation
- **`cad/wasm/target` (`WASM_BUILD_TARGET`):** Kept completely separate from `cad/target` (`NATIVE_TEST_TARGETS = ("target",)`), so `build` (`wasm-pack build wasm --release --locked`) never serializes behind native CAD tests during `check.py` overlap.
- **`cad/scripts/cad_build.py`:** Runs ONE unified `cargo test --no-run --manifest-path Cargo.toml` selecting `boardstudio-cad`, `boardstudio-cad-wasm`, `monstertruck-io`, and `monstertruck-meshing` together (dynamically discovering production test/example targets via `cargo metadata`), then executes all 15 compiled test binaries in parallel (capped by `BOARDSTUDIO_CHECK_JOBS`).
- Updated `cad/monstertruck/src/layered/tests.rs` (`build_has_no_general_3d_boolean_dependency_or_runtime_calls`) to perform a transitive BFS over `cad/Cargo.lock` proving neither `boardstudio-cad` nor `boardstudio-cad-wasm` depends on `monstertruck-solid`.
- Updated `cad/scripts/test-cad-build.py`, `scripts/check.py`, `scripts/test-check.py`, `.github/workflows/check.yaml`, `scripts/test-ci-workflow.py`, `scripts/build-time/cold.py`, `scripts/security-audit.py`, `scripts/security-audit.test.py`, `cad/monstertruck-qualification/README.md`, and `docs/research/cad/collision-qualification-plan-2026-10-18.md`.
- Formatted `cad/monstertruck/src/step/mod.rs` and `cad/monstertruck/tests/step_import.rs` with `rustfmt --edition 2024`.

---

## 3. Measured Results & Verification

| Metric | Before (`d22c9b9b3`) | After (Unified `cad/`) | Delta |
|---|---:|---:|---:|
| Crate compilations (`Compiling` lines, bypass) | `303` (`95` duplicated crates) | **`196`** (`0` duplicated `monstertruck_*`/`cgmath`/`serde`/`step-p21`/`nom`/`zerocopy`) | **`-107` crates (`-35.3%`)** |
| Shared `.rlib`s (`io`/`geometry`/`modeling`/`meshing`/`core`/`step_p21`) | `2` each (`syn` `5`, `nom` `3`, `zerocopy` `2`) | **`1` each** (`syn` `2`, `nom` `1`, `zerocopy` `1`) | **100% deduplicated** |
| Cold `test-cad.py` compile CPU (bypass) | `801 s` (`cpu-opt2-bypass`) | **`623.6 s`** (`591.6 s` unthrottled) | **`-177` to `-210 s`** |
| Cold `test-cad.py` warm wall / CPU | `104.86 s` / `1,086.28 s` | **`95.02 s`** / **`1,004.22 s`** | **`-9.84 s` wall / `-82.1 s` CPU** |
| CAD WASM (`boardstudio_cad_wasm_bg.wasm`) | `7,321,657 B` | **`7,315,454 B`** | **`-6,203 B`** |
| `wasm32-unknown-unknown` `cargo tree -e features` | baseline | **identical (0 diff)** | **unchanged** |

### Verification Gates Passed
- `python3 cad/scripts/test-cad-build.py`: **18/18 passed**
- `python3 cad/scripts/test-cad.py`: **566 passed across 15 result blocks (`0` failed)**
- `python3 scripts/check.py security`: **exit 0** (all 3 lockfiles including unified `cad/Cargo.lock`)
- `python3 scripts/check.py repo tooling build browser release`: **exit 0** (`364` CAD browser checks passed, `face-completeness-gate.py`, `test-cad-provider.py`, `test-cold-responses.py`, release build, **0** Cargo manifest warnings)

> Note: the two isolated `test-cad.py` CPU rows above (`623.6 s` bypass, `1,004.22 s` warm) did not
> reproduce under full `cold.py` measurement and are superseded by §4. The crate-count rows
> (`303`→`196` compilations, deduplicated `.rlib`s) are confirmed by the full runs (`373`→`244`
> `cad-test` units, `-34.6%`).

---

## 4. Full `cold.py` Runs (Authoritative)

Method: `python3 scripts/build-time/cold.py w3-5-seed` (sccache seed, full check),
`cold.py w3-5-warm` (wiped, sccache), `cold.py w3-5-bypass --without-cache` (wiped, no cache),
each a full default `check.py` under the blocking build lock. Baseline is the
`cpu-opt2-{warm,bypass}` full runs from the `cold-build-port-verification` worktree
(`perf/cold-build-cpu` @ `d22c9b9b3`, same base; non-CAD unit sets are identical, so the
comparison is valid). Single run per arm. `cad/host/cad-test` below is the `cad-test`-profile
host lane (all of it falls in the `test` step, i.e. `test-cad.py` compilation).

### Step walls (`check: step times`)

| Step | Warm before | Warm after | Δ warm | Bypass before | Bypass after | Δ bypass |
|---|---:|---:|---:|---:|---:|---:|
| `repo` | `6.1 s` | `6.0 s` | `-0.1 s` | `6.0 s` | `5.9 s` | `-0.1 s` |
| `tooling` | `19.7 s` | `19.5 s` | `-0.2 s` | `20.0 s` | `19.6 s` | `-0.4 s` |
| `test-css` | `0.2 s` | `0.2 s` | `0.0 s` | `0.2 s` | `0.2 s` | `0.0 s` |
| `build` | `275.2 s` | `201.3 s` | `-73.9 s` | `327.7 s` | `300.4 s` | `-27.3 s` |
| `lint` | `193.9 s` | `208.3 s` | `+14.4 s` | `250.7 s` | `244.4 s` | `-6.3 s` |
| `test` | `170.1 s` | `190.4 s` | `+20.3 s` | `241.8 s` | `250.1 s` | `+8.3 s` |
| `browser` | `102.2 s` | `98.5 s` | `-3.7 s` | `101.5 s` | `97.4 s` | `-4.1 s` |
| **total** | **`403.6 s`** | **`332.6 s`** | **`-71.0 s`** | **`455.6 s`** | **`423.7 s`** | **`-31.9 s`** |
| `cold: wall` | `404.0 s` | `333.2 s` | `-70.8 s` | `456.1 s` | `424.2 s` | `-31.9 s` |

### Compiler CPU, units & process CPU (`report.py`)

| Metric | Warm before | Warm after | Δ warm | Bypass before | Bypass after | Δ bypass |
|---|---:|---:|---:|---:|---:|---:|
| Total compiler CPU | `1,929 s` | `1,887 s` | `-42 s` | `2,827 s` | `2,854 s` | `+27 s` |
| Total units | `1,786` | `1,656` | `-130` | `1,785` | `1,656` | `-129` |
| `cad/host/cad-test` CPU | `374.2 s` | `486.7 s` | **`+112.5 s`** | `800.7 s` | `820.2 s` | **`+19.5 s`** |
| …of which `--test` binaries (12×) | `217.8 s` | `395.9 s` | `+178.1 s` | `210.3 s` | `390.8 s` | `+180.5 s` |
| …of which rlibs | `156.7 s` (`361`×) | `90.9 s` (`232`×) | `-65.8 s` | `590.5 s` (`361`×) | `429.5 s` (`232`×) | **`-161.0 s`** |
| Process-tree CPU | `2,868.1 s` | `3,008.4 s` | `+140.3 s` | `3,770.3 s` | `3,976.8 s` | `+206.5 s` |

### `test-cad.py` wall decomposition

| Metric | Warm before | Warm after | Δ warm | Bypass before | Bypass after | Δ bypass |
|---|---:|---:|---:|---:|---:|---:|
| `test-cad.py` wall | `157.5 s` | `184.4 s` | `+26.9 s` | `241.6 s` | `250.0 s` | `+8.4 s` |
| Compile span (first→last `cad-test` unit) | `87.6 s` | `91.3 s` | `+3.7 s` | `183.5 s` | `170.0 s` | `-13.5 s` |
| Execution tail (last compile→`test-cad.py` end) | `68.4 s` | `91.6 s` | `+23.2 s` | `56.8 s` | `79.0 s` | `+22.2 s` |

### Cache & count verification (both `w3-5` runs)

- `w3-5-warm` sccache delta: `+786` executed, `+786` hits, `+0` misses, `+0` non-cacheable, `+0` errors. (`--test` binaries, bins, proc-macros and metadata-only units bypass sccache by design in `rustc-sccache-fail-open`, so the `+180 s` test-binary cost is real compile in both arms.)
- `2,454` native baselined tests executed; `84`/`84` `test result: ok` blocks, `0` failed (identical to baseline).
- `test-cad.py`: `15` blocks, `566` passed, `0` failed, exit `0` (verified by direct re-run on the identical post-run binaries; both cold logs show `test-cad.py` exit `0`).
- Browser crate launches: `12` blocks, `799` passed (identical to baseline, both runs).
- Page: `run-wasm-tests: executed 126, failed 0` (both runs).
- Drag: `PASS drag browser smoke: 15 checks` (both runs).
- CAD browser: `364 browser checks passed` (both runs).
- Cargo manifest warnings: `0` (`did not match any packages`: `0`); remaining `warning:` lines are only the pre-existing `proc-macro-error2` future-Rust notice — the `nom`/`quick-xml` one is gone with `vtkio`.

### Analysis

1. **Dedup is real, but test-binary codegen ate it.** Bypass rlibs fell `-129` units / `-161.0 s` CPU, while the 12 `--test` binaries rose `+180.5 s`: the qualification suites (`monstertruck_meshing` test `25.4 s`→`134.7 s`, `monstertruck_io` test `15.5 s`→`65.4 s`, `input` test `7.2 s`→`39.3 s`) moved from `opt-level = 0` to the unified `opt-level = 1`. Net bypass `cad-test` CPU is flat at `+19.5 s` (`+2.4%`, within run-to-run noise), not the isolated `-177 s` projection.
2. **The warm `build`-step win is a baseline seed-miss artifact, not a W3-5 effect.** Identical `root/wasm32/wasm-check` units missed in `cpu-opt2-warm` (`boardstudio_core_engine` `41.6 s`, `web_sys` `40.4 s`) and hit in `w3-5-warm` (`1.0 s`, `9.7 s`). Do not claim the `-73.9 s` warm `build` delta.
3. **Test execution got slower (`+22 s` tail, both arms).** Compile spans shrank (bypass) or held (warm), but parallel binary execution under full-check overlap now costs more than the old sequential flow saved. Hypothesis is CPU contention with overlapping `build`/`lint` compiles; not root-caused further.
4. **Full-check wall improved (`-31.9 s` bypass, `-70.8 s` warm cold-wall), but neither is a robust W3-5 claim.** Warm is confounded by (2); bypass is under the 60 s single-run threshold and plausibly comes from reduced overlap contention (fewer test-step compiles competing with `build`/`lint`) rather than less work.
5. **Kept value is structural, not CPU:** one workspace, one lockfile, no `vtkio` CVEs, no duplicated dependency graph. A follow-up could recover the `+180 s` by scoping the qualification `--test` binaries (or their heaviest suites) back to `opt-level = 0` via per-package profile overrides.
