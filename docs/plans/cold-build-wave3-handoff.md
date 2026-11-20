# Cold Build & Codebase Optimization — Wave 3 Implementation Handoff Plan

**Status:** Ready for handoff  
**Baseline commit:** `71cb6cd9b` (`perf/cold-build-cpu`)  
**Prior waves:** Wave 1 (`787aeac09..e9f7caef4`) + Wave 2 (`e9f7caef4..71cb6cd9b`)  
**Evidence logs:** `.evidence/cold-build-port/cpu-opt2-{warm,bypass}.{log,jsonl}`, `cpu-opt2-report-{warm,bypass,lanes}.log`, `cpu-opt2-count-audit.log`

---

## 1. Current Baseline & Wave 3 Goals

Waves 1 and 2 reduced full cold `python3 scripts/check.py` runs from `689.8 s` warm / `687.2 s` bypass down to **`404.0 s` warm / `456.1 s` bypass**, and cut compiler child CPU by **`-53.3%` warm (`4,127 s → 1,929 s`)** and **`-42.5%` bypass (`4,916 s → 2,827 s`)** with zero test-coverage regressions.

### Current Step & Lane Breakdown (`71cb6cd9b`)

| Step / Lane | Warm (`cpu-opt2-warm`) | Bypass (`cpu-opt2-bypass`) | Dominant Bottleneck |
|---|---:|---:|---|
| `repo` + `tooling` + `test-css` | `26.0 s` wall | `26.2 s` wall | — |
| `build` (overlapped critical path) | **`275.2 s` wall** (`642 s` CPU) | **`327.7 s` wall** (`1,001 s` CPU) | `build-web.py --fast` (`217.1 s` / `271.6 s`) + serial `BROWSER_CRATE_BUILD` (`39.2 s`) + serial `PAGE_TEST_BUILD` (`18.8 s`) |
| `lint` (overlapped) | `193.9 s` wall (`470 s` CPU) | `250.7 s` wall (`498 s` CPU) | Finishes `81 s` before `build` (warm) |
| `test` (overlapped) | `170.1 s` wall (`817 s` CPU) | `241.8 s` wall (`1,328 s` CPU) | `boardstudio_core_engine` (`90.3 s` / `121.7 s` test bin) + `cad/host/cad-test` (`374 s` / `801 s` CPU) |
| `browser` (serial after overlap) | **`102.2 s` wall** | **`101.5 s` wall** | `run-drag-browser.py` (`101.7 s` single serial Chrome) |
| **Total** | **`404.0 s` wall / `1,929 s` compiler CPU** | **`456.1 s` wall / `2,827 s` compiler CPU** | **`1,786` / `1,785` compile units** |

### Wave 3 Target Outcomes
- **Tier 1 (Quick Wins: W3-1, W3-2, W3-3):** Target **`~300–330 s` warm cold wall** (`-75` to `-105 s`) and **`~385–410 s` bypass cold wall** (`-45` to `-70 s`) without altering crate topology.
- **Tier 2 (Structural: W3-4, W3-5, W3-6):** Attack the three largest remaining compile-CPU sinks (`boardstudio-core-engine` `208.6 s`, `cad/host/cad-test` `801 s` bypass / `374 s` warm, `web_sys` + `boardstudio-web-host` `90+ s`) and improve long-term codebase modularity and incremental edit speed.

---

## 2. Mandatory Guardrails for Every Handoff Agent

Read [AGENTS.md](../../AGENTS.md) and [CODING_STANDARDS.md](../../CODING_STANDARDS.md) before making edits. Every Wave 3 task must obey:

1. **Shared build lock & shallow `TMPDIR`:** Hold `~/.cache/tmp/boardstudio-build.lock` with a **blocking** `flock` (never `flock -n`) and export `BOARDSTUDIO_BUILD_LOCK_FD=9 TMPDIR="$HOME/.cache/tmp" PATH="$HOME/.cargo/bin:$PATH"` for every Cargo, `check.py`, `cold.py`, and browser run. Never set `CARGO_TARGET_DIR` when running `scripts/build-time/cold.py`.
2. **Profiles & concurrency:** Never pass `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, or `CARGO_PROFILE_DEV_DEBUG=2`, and never add `rustc-wrapper` to repo `.cargo/config.toml`. Never overlap manual `cargo`/`wasm-pack` runs against the same `target` directory.
3. **Zero weakened gates or regressions:** Never suppress lints, weaken assertions, lower test count floors, use nightly features, use `git stash`, or inspect/print `target/dx/.captured-args/*.json`.
4. **Hard count floors (must match or exceed in every full `check.py` / `cold.py` run):**
   - Native workspace tests: **`>= 2,454`** baselined tests executed (`0` dropped; all `901` Core integration test names preserved)
   - CAD tests: **`566`** passed across **`15`** result blocks (`0` failed)
   - Browser crate tests: **`799`** passed across **`12`** launches (`0` failed)
   - Page tests: **`126`** passed (`0` failed, `0` incomplete)
   - Drag browser smoke: **`15`** checks passed
   - CAD browser labels: **`364`** browser checks passed
   - Cargo manifest warnings: **`0`** (`did not match any packages`, ignored workspace-member `resolver`)
5. **Evidence hygiene:** Redirect long logs into `.evidence/` (logs only; never create `target/` dirs or crate copies inside `.evidence/`).
6. **Pre-requisite note before branching:** If branching from `perf/cold-build-cpu` before PR #28 lands on `dev`, first fix PR #28's CI failures (`scripts/tailwind_tools.py` `BOARDSTUDIO_TAILWINDCSS` check against `.github/actions/web-tools`, `cad/scripts/test-cad-browser.py` in `web-packaged`, and the `audit` job failures specified in **W3-0** below).

---

## 3. Execution Order & Parallel Ownership Matrix

Tasks in the same wave have **disjoint owned files** and can be executed in parallel in isolated git worktrees.

| Wave | Task ID | Summary | Est. Effort | Est. Savings | Owned Files (Disjoint Within Wave) |
|---|---|---|---|---|---|
| **3A** | **W3-0** | Fix CI `audit` (`check.py security`) failures (`zip`, `rustix`, `heapless`) | 1–2 h | Unblocks CI `audit` gate | `core/crates/engine/Cargo.toml`, `core/Cargo.toml`, `Cargo.lock`, `cad/wasm/Cargo.toml`, `cad/monstertruck/Cargo.lock`, `scripts/security-audit.py` (only if upstream-unpatched) |
| **3A** | **W3-1** | Eliminate `build` 34 s serial tail | 2–4 h | `-25` to `-35 s` wall (warm & bypass) | `scripts/check.py`, `scripts/build-web.py`, `scripts/run-wasm-tests.py`, `scripts/test-check.py`, `scripts/test-build-web.py`, `scripts/test-run-wasm-tests.py`, `scripts/test-check-quick.py` |
| **3A** | **W3-2** | Fix `web_sys` warm `sccache` misses + `cold.py` UDS stats | 1–3 h | `-40` to `-60 s` warm CPU, `-30` to `-40 s` warm wall | `scripts/build-time/cold.py`, `scripts/build-time/report.py` (plus host `~/.local/bin/rustc-sccache-fail-open` if needed, documented separately) |
| **3A** | **W3-3** | Split `run-drag-browser.py` into 2 parallel browser lanes | 2–4 h | `-25` to `-38 s` `browser` wall | `tools/browser-harness/src/bin/drag_smoke.rs`, `scripts/run-drag-browser.py`, `scripts/test-drag-browser-*.py` (coordinate `scripts/check.py` lane hookup after W3-1 merges) |
| **3B** | **W3-6** | Audit `boardstudio-web-host` (4× copies) & `web-sys` 183-feature scope | 3–6 h | `-15` to `-40 s` CPU, smaller `.rlib`s | `Cargo.toml`, `web/Cargo.toml`, `renderer/Cargo.toml`, `web/crates/host/**`, `Cargo.lock` |
| **3C** | **W3-4** | Decompose `boardstudio-core-engine` (114k LOC) & `boardstudio-web-layout` (72k LOC) | 2–5 d | `-60` to `-120 s` CPU/wall + major incremental speedup | `core/crates/engine/**`, new `core/crates/*`, `web/crates/layout/**`, new `web/crates/*`, root `Cargo.toml`, `scripts/native-test-baseline.json` |
| **3C** | **W3-5** | Deduplicate `cad/monstertruck` & `cad/monstertruck-qualification` workspaces + CAD debt | 1–3 d | `-200` to `-350 s` bypass CPU (`-80` to `-150 s` warm) | `cad/monstertruck/**`, `cad/monstertruck-qualification/**`, `cad/scripts/**` |

> [!TIP]
> **W3-0**, **W3-1**, and **W3-2** have 100% disjoint owned files and can run in parallel immediately (W3-0 can also land as its own dependency/security PR ahead of Wave 3A). Land **W3-3**'s harness `--suite` flag in parallel too, then wire its second lane into `scripts/check.py` right after W3-1 lands. Measure a `cold.py` checkpoint (`cpu-opt3-warm` / `cpu-opt3-bypass`) before starting Tier 2 (**W3-4**, **W3-5**, **W3-6**).

---

## 4. Detailed Task Specifications

### W3-0: Fix CI `audit` (`check.py security`) Failures

**Problem & measured CI evidence:**
The `audit` job in [.github/workflows/security.yaml](../../.github/workflows/security.yaml) runs `python3 scripts/check.py security` ([scripts/security-audit.py](../../scripts/security-audit.py)), followed by `python3 scripts/check.py deny` and `cargo +nightly-2026-08-12 miri test --manifest-path renderer/vendor/cgmath-next-0.18.1/Cargo.toml --locked --test soundness`.

`scripts/security-audit.py` runs `cargo audit --file <tmp>.lock --deny unsound --ignore RUSTSEC-2026-0196 --ignore RUSTSEC-2026-0197 --ignore RUSTSEC-2026-0386` across the 3 `CGMATH_MANIFESTS` lockfiles. On PR #28 (`71cb6cd9b`), `check.py security` failed with **11 blocking findings** (preventing `check.py deny` and the Miri soundness test from running in that job):

1. **`Cargo.lock` (436 deps) — FAILED (`1 vulnerability` + `1 denied unsound warning`; `1 allowed warning`):**
   - **BLOCKING — Vulnerability (`RUSTSEC-2026-0548`, 2026-10-17):** `zip v9.0.1` — *"`ZipStreamReader` extracts entries under local-header names that can differ from the central directory"*. **Solution:** Upgrade to `>= 9.0.5`.
     - **Reverse chain:** Direct exact pin `zip = { version = "=9.0.1", default-features = false, features = ["deflate-flate2-zlib-rs"] }` in [core/crates/engine/Cargo.toml](../../core/crates/engine/Cargo.toml#L18) and [core/Cargo.toml](../../core/Cargo.toml#L27) (`[dev-dependencies]`).
     - **Existing PR note:** Check PR #25 (`fix/zip-dependency-audit`) before duplicating work — cherry-pick or rebase against it if already open/merged on `dev`.
   - **BLOCKING — Denied Warning (`unsound`, `RUSTSEC-2026-0486`, 2026-10-27):** `rustix v1.1.5` — *"xattr functions mark unwritten bytes as initialized when given a zero-length buffer"*.
     - **Reverse chain:** `rustix 1.1.5` ← `memfd 0.6.6` ← `subsecond 0.7.10` ← `dioxus 0.7.10` / `dioxus-core 0.7.10` / `dioxus-devtools 0.7.10` (Unix target dependency).
     - **Fix:** Patch release `rustix >= 1.1.6` is already published (and already locked at `1.1.6` in `tools/browser-harness/Cargo.lock`); bump via `cargo update -p rustix`.
   - **Non-blocking (`unmaintained`, allowed):** `smartstring v1.0.1` (`RUSTSEC-2026-0249`, 2026-05-03).

2. **`cad/monstertruck/Cargo.lock` (183 deps) — FAILED (`9 denied unsound warnings`; `3 allowed warnings`):**
   - **BLOCKING — 9 Denied Warnings (`unsound`, all dated 2026-11-18):** `heapless v0.8.0`:
     - `RUSTSEC-2026-0574` (`FindMutView::pop` UAF)
     - `RUSTSEC-2026-0575` (`Spsc::peek` `Relaxed` vs `Acquire` ordering)
     - `RUSTSEC-2026-0576` (`IndexMap::truncate` UB)
     - `RUSTSEC-2026-0577` (panicking `Drop` UB)
     - `RUSTSEC-2026-0578` (`SortedLinkedList::zeroize` UB)
     - `RUSTSEC-2026-0579` (`Deque` `TryFrom` UB)
     - `RUSTSEC-2026-0580` (Treiber stack race)
     - `RUSTSEC-2026-0581` (`Deque::make_contiguous` UB)
     - `RUSTSEC-2026-0582` (`IndexMap::insert` at `u16::MAX` UB)
     - **Reverse chain:** `heapless 0.8.0` ← `rstar 0.13.0` ← `parry3d-f64 0.31.1` ← `boardstudio-cad-wasm 0.1.0` ([cad/wasm/Cargo.toml](../../cad/wasm/Cargo.toml#L18), workspace member of [cad/monstertruck/Cargo.toml](../../cad/monstertruck/Cargo.toml#L8)).
     - **Reachability detail:** `rstar 0.13.0` only imports `heapless::binary_heap` (`rstar-0.13.0/src/algorithm/nearest_neighbor.rs:15`).
   - **Non-blocking (`unmaintained`, allowed):** `chrono v0.4.45` (`RUSTSEC-2026-0543`), `proc-macro-error2 v2.0.1` (`RUSTSEC-2026-0173`), `ttf-parser v0.25.1` (`RUSTSEC-2026-0192`).

3. **`renderer/vendor/cgmath-next-0.18.1/Cargo.lock` (27 deps) — PASSED** (`0` vulnerabilities, `0` warnings).

> [!NOTE]
> `scripts/security-audit.py` audits only the 3 `CGMATH_MANIFESTS` lockfiles (`Cargo.lock`, `cad/monstertruck/Cargo.lock`, `renderer/vendor/cgmath-next-0.18.1/Cargo.lock`); `cad/monstertruck-qualification/Cargo.lock` and `tools/browser-harness/Cargo.lock` are outside its current loop.

**Owned files:**
- [core/crates/engine/Cargo.toml](../../core/crates/engine/Cargo.toml)
- [core/Cargo.toml](../../core/Cargo.toml)
- [Cargo.lock](../../Cargo.lock)
- [cad/wasm/Cargo.toml](../../cad/wasm/Cargo.toml) (only if bumping `parry3d-f64` resolves `heapless`)
- [cad/Cargo.lock](../../cad/Cargo.lock) (was `cad/monstertruck/Cargo.lock` before the W3-5 workspace unification)
- [scripts/security-audit.py](../../scripts/security-audit.py) (only if an upstream-unpatched `heapless` advisory requires a documented temporary `ACCEPTED_ADVISORIES` entry)

**Implementation steps:**
1. **Upgrade `zip` (`>= 9.0.5`) in root workspace:**
   - Check PR #25 (`fix/zip-dependency-audit`) to match the chosen `zip` pin version (e.g., `=9.0.5` or latest `9.0.x` patch).
   - Update `zip` in [core/crates/engine/Cargo.toml](../../core/crates/engine/Cargo.toml#L18) and [core/Cargo.toml](../../core/Cargo.toml#L27), then run `cargo update -p zip` to refresh root [Cargo.lock](../../Cargo.lock).
2. **Upgrade `rustix` (`>= 1.1.6`) in root workspace:**
   - Run `cargo update -p rustix` in the repo root and verify `Cargo.lock` resolves `rustix` to `>= 1.1.6` with no unrelated crate churn.
3. **Resolve `heapless 0.8.0` in `cad/monstertruck/Cargo.lock`:**
   - First attempt a clean upstream upgrade: test `cargo update --manifest-path cad/monstertruck/Cargo.toml -p heapless` and `-p rstar` (and check whether a newer `parry3d-f64` in [cad/wasm/Cargo.toml](../../cad/wasm/Cargo.toml#L18) drops `heapless 0.8.0` or upgrades `rstar` to a patched `heapless >= 0.9` release).
   - **Only if no patched `heapless 0.8.x` / `rstar` / `parry3d-f64` release exists on crates.io:** audit each of `RUSTSEC-2026-0574`..`RUSTSEC-2026-0582` against `rstar`'s sole `heapless::binary_heap` usage (pay special attention to `RUSTSEC-2026-0577` panicking-`Drop` UB and verify whether `binary_heap` or its element type in `rstar` is affected). For any provably unreachable upstream-unpatched advisories, add dated entries to `ACCEPTED_ADVISORIES` in [scripts/security-audit.py](../../scripts/security-audit.py#L26) following the exact `RUSTSEC-2026-0386` comment convention (review date, newest lockable version, reachability justification, revisit trigger).
4. **Verify all `security.yaml` gates and build/test compatibility:**
   - Run `python3 scripts/check.py security deny` under the shared build lock.
   - Run `cargo +nightly-2026-08-12 miri test --manifest-path renderer/vendor/cgmath-next-0.18.1/Cargo.toml --locked --test soundness` (if the nightly toolchain is installed locally; otherwise verify in CI).
   - Run `python3 scripts/check.py test build` (or targeted `boardstudio-core-engine` + `boardstudio_core` + `cad/monstertruck` + `cad/wasm` builds/tests) to confirm zero compile or test regressions from the `zip`, `rustix`, and CAD lockfile updates.

**Verification & acceptance:**
- `python3 scripts/check.py security` exits `0` across all 3 lockfiles (`Cargo.lock`, `cad/monstertruck/Cargo.lock`, `renderer/vendor/cgmath-next-0.18.1/Cargo.lock`).
- `python3 scripts/check.py deny` exits `0`.
- Zero Cargo manifest warnings and zero dropped tests in root workspace and `cad/monstertruck` / `cad/wasm`.

---

### W3-1: Eliminate the 34 s Serial Tail at the End of `build`

**Problem & measured evidence:**
In `cpu-opt2-warm`, `build` (`275.2 s`) finishes `81.3 s` after `lint` (`193.9 s`) and `105.1 s` after `test` (`170.1 s`), making `build` the sole critical path of the overlapped compile phase. Two serial gaps sit at the end of `build`:
1. **`15.4 s` idle Cargo lock (`t = 201.7..217.1`):** `boardstudio-web` finishes compiling at `201.7 s`, then [scripts/build-web.py](../../scripts/build-web.py) spends `15.4 s` running `dx tools assets`, `wasm-bindgen`, and site staging while `BROWSER_CRATE_BUILD` waits in [scripts/check.py](../../scripts/check.py#L976).
2. **`18.8 s` serial `PAGE_TEST_BUILD` (`t = 256.8..275.0`):** `PAGE_TEST_BUILD` ([scripts/check.py](../../scripts/check.py#L120), [scripts/run-wasm-tests.py](../../scripts/run-wasm-tests.py#L46)) runs *after* `BROWSER_CRATE_BUILD` finishes (`39.2 s`) and compiles 4 `lib` crates (`boardstudio_web_{layout,pcb,keymap,library}`) plus the `boardstudio-web` test binary serially.

**Owned files:**
- [scripts/check.py](../../scripts/check.py)
- [scripts/build-web.py](../../scripts/build-web.py)
- [scripts/run-wasm-tests.py](../../scripts/run-wasm-tests.py)
- [scripts/test-check.py](../../scripts/test-check.py)
- [scripts/test-build-web.py](../../scripts/test-build-web.py)
- [scripts/test-run-wasm-tests.py](../../scripts/test-run-wasm-tests.py)
- [scripts/test-check-quick.py](../../scripts/test-check-quick.py)

**Implementation steps:**
1. **Combine `BROWSER_CRATE_BUILD` and `PAGE_TEST_BUILD` into one `cargo test --no-run` invocation (when both are scheduled):**
   - Inspect how `WASM_CRATE_BUILD` ([scripts/check.py](../../scripts/check.py#L190-L204)) and `PAGE_BUILD` ([scripts/run-wasm-tests.py](../../scripts/run-wasm-tests.py#L45-L47)) invoke Cargo (`--profile wasm-check --target wasm32-unknown-unknown --locked --message-format=json`).
   - Verify whether one Cargo invocation:
     ```text
     cargo test --no-run --target wasm32-unknown-unknown --profile wasm-check --locked --message-format=json \
       -p boardstudio-web-host ... -p boardstudio-web-layout --lib \
       -p boardstudio-web --bin boardstudio-web --no-default-features \
       --features boardstudio-web/page,boardstudio-web-host/test-support
     ```
     (or `--bins --lib` scoped cleanly without enabling unwanted workspace binaries) builds all 12 crate test `.wasm`s + the page test `.wasm` in one dependency graph with identical feature hashes.
   - If Cargo rejects mixing `--lib` and `--bin` across packages cleanly, evaluate launching `BROWSER_CRATE_BUILD` and `PAGE_TEST_BUILD` or passing `--bin boardstudio-web --lib` and verify zero duplicate crate hashes in `target/wasm32-unknown-unknown/wasm-check/deps/`.
   - Capture the emitted `boardstudio-web` test artifact path (`BOARDSTUDIO_PAGE_TEST_ARTIFACT` or JSON discovery) so `run-wasm-tests.py` skips re-invoking Cargo.
2. **Overlap `build-web.py --fast` post-Cargo staging (`dx tools assets`, `wasm-bindgen`, file copy) with `BROWSER_CRATE_BUILD`:**
   - Either let `build-web.py --fast` signal/complete Cargo before staging (or split `build-web.py` / allow `check.py` to launch the wasm test prebuild concurrently once `cargo build` releases `target/.cargo-lock`, since `wasm-bindgen` and `dx tools assets` do not hold `target/.cargo-lock` and do not write to `target/wasm32-unknown-unknown/wasm-check/deps/`).
   - Ensure `build-web.py` staging finishes before `browser` starts, and preserve exact error/cancellation semantics (`--keep-going`, `SIGTERM`/`SIGKILL` group cleanup, receipt observation).
3. **Update unit tests** in `test-check.py`, `test-build-web.py`, `test-run-wasm-tests.py`, and `test-check-quick.py`.

**Risks & guardrails:**
- Do NOT enable `test-support` in the packaged production/`--fast` page binary (`boardstudio-web`).
- Confirm `dx build` fallback path (when standalone Tailwind / locked `wasm-bindgen` / matching `dx` are absent) still runs serially without racing.

**Verification & acceptance:**
- `python3 scripts/check.py repo tooling lint` passes (`0` failures).
- `python3 scripts/check.py build browser` passes with exact test counts (`799/12` crate, `126` page, `15` drag, `364` CAD browser).
- Measured `build` wall drops by `>= 20 s` in a clean `cold.py` run with zero new duplicate `.rlib` hashes in `target/wasm32-unknown-unknown/wasm-check/deps/`.

---

### W3-2: Fix `web_sys` Warm `sccache` Misses & Automate `SCCACHE_SERVER_UDS` Reporting in `cold.py`

**Problem & measured evidence:**
1. Comparing `817` matched eligible registry link units between `cpu-opt2-warm.jsonl` (`385.9 s`) and `cpu-opt2-bypass.jsonl` (`880.4 s`) revealed `182` fast hits (`manifold_rust` `37.4 s → 0.52 s`, `parry3d_f64` `27.5 s → 0.78 s`) **but 9 expensive misses** (`102.9 s` warm vs `81.8 s` bypass — **`+21.1 s` worse warm than bypassed!**):
   - `web_sys` (wasm32, `24.9 MiB` `.rlib`): **`40.37 s` CPU / `56.1 s` wall warm** vs `30.70 s` CPU bypass
   - `rhai` (wasm32): `14.58 s` warm vs `12.07 s` bypass (note `rhai` host *did* hit at `1.78 s` vs `12.48 s`)
   - `wide` (wasm32 + host): `10.95 s` + `10.71 s` warm vs `8.52 s` + `8.35 s` bypass
   - `wasm_bindgen_macro_support` (`6.67 s`), 2× `syn` (`6.39 s` + `6.32 s`), `three_d_asset` (`4.74 s` + `2.17 s`)
2. [scripts/build-time/cold.py](../../scripts/build-time/cold.py) does not snapshot `sccache --show-stats` against `SCCACHE_SERVER_UDS="/run/user/$(id -u)/boardstudio-sccache/server.sock"`, so warm runs lack automated before/after hit/miss/non-cacheable verification.

**Owned files:**
- [scripts/build-time/cold.py](../../scripts/build-time/cold.py)
- [scripts/build-time/report.py](../../scripts/build-time/report.py)
- (Host-only diagnostic/fix if warranted: `~/.local/bin/rustc-sccache-fail-open`, documented in the PR/evidence rather than silently mutated)

**Implementation steps:**
1. **Automate `SCCACHE_SERVER_UDS` stats in `cold.py`:**
   - When `--without-cache` is not set and `$XDG_RUNTIME_DIR/boardstudio-sccache/server.sock` (fallback `/run/user/<uid>/boardstudio-sccache/server.sock`) exists, run `sccache --show-stats` with `SCCACHE_SERVER_UDS` before and after the cold run, saving `.evidence/cold-build-port/<name>-sccache-{before,after}.log` and printing a one-line delta (`hits`, `misses`, `non-cacheable calls`, `errors`) at the end of `<name>.log`.
2. **Root-cause why `web_sys`, `wide`, `three_d_asset`, `syn`, and `wasm_bindgen_macro_support` missed in `cpu-opt2-warm`:**
   - Run two back-to-back targeted `cargo build --locked --target wasm32-unknown-unknown --profile wasm-check -p web-sys` (or minimal repro with `SCCACHE_LOG=debug` redirected to `.evidence/`) under `SCCACHE_SERVER_UDS` to determine whether:
     - (a) `cpu-opt2-warm-seed` never populated them because `target/` was already populated when `warm-seed` ran (so a second wiped warm run hits 100%), **or**
     - (b) `sccache` rejects/fails to cache them (e.g., `SCCACHE_CLIENT_SIDE=1` hashing/timeout/size limit on `24.9 MiB` `web_sys`, `OUT_DIR`/proc-macro path sensitivity, or `const-random-macro` / `ahash` non-determinism), **or**
     - (c) `SCCACHE_MAX_FRAME_LENGTH` / cache eviction occurred.
3. **Apply the minimal fix** (e.g., `cold.py --seed` mode that wipes or ensures warm-seed populates sccache, or fixing the wrapper/daemon setting if `sccache` itself was rejecting `web_sys`/`wide`) and verify with a fresh before/after warm run.

**Verification & acceptance:**
- `cold.py` automatically captures non-zero `boardstudio-sccache` stats and prints exact hit/miss/non-cacheable deltas.
- `web_sys` wasm32 hits `sccache` in a warm cold run (`< 5 s` CPU instead of `40.4 s` CPU / `56.1 s` wall), or if an upstream `sccache` limitation prevents caching a specific crate, document the exact `SCCACHE_LOG` proof and bypass only proven non-cacheable patterns.
- `0` non-cacheable calls delta on the `boardstudio-sccache` daemon.

---

### W3-3: Split `run-drag-browser.py` into 2 Parallel Browser Lanes

**Problem & measured evidence:**
In `cpu-opt2-warm` (`browser` = `102.2 s`), **all 12 crate WASM launches, 126 page tests, and 6 CAD browser scripts finished in ~93 s**, while `python3 scripts/run-drag-browser.py` ran **`101.7 s`** serially in a single Chrome process. Inside [tools/browser-harness/src/bin/drag_smoke.rs](../../tools/browser-harness/src/bin/drag_smoke.rs), the checks are already structured into independent scenarios:
- **Lane A (Drag interactions):** `drag_scenarios` (`drag_smoke.rs:2041`: setup + Part + Column + Stagger + Ctrl Offset + Matrix move + Undo/reload — ~35–40 s).
- **Lane B (Collision & qualification):** `collision_scenario` (`drag_smoke.rs:1348`, ~31 s) + 2× `qualification_scenario` (`drag_smoke.rs:1558`, ~30 s) — already runs each scenario in its own isolated `BrowserContext`!

**Owned files:**
- [tools/browser-harness/src/bin/drag_smoke.rs](../../tools/browser-harness/src/bin/drag_smoke.rs)
- [scripts/run-drag-browser.py](../../scripts/run-drag-browser.py)
- [scripts/check.py](../../scripts/check.py) (coordinate after W3-1)
- [scripts/test-check.py](../../scripts/test-check.py)
- [scripts/test-drag-browser-consumer-controls.py](../../scripts/test-drag-browser-consumer-controls.py) (also replace its local `BROWSER_TMP` `> 40`-byte check at lines 20–24 with `browser_tools.shallow_browser_tmpdir()`!)

**Implementation steps:**
1. Add a `--suite {all,drag,collision}` (or `BOARDSTUDIO_DRAG_SUITE`) selector to `drag_smoke.rs` and `scripts/run-drag-browser.py`:
   - Default (`all`) runs all 15 checks identically so standalone `python3 scripts/run-drag-browser.py`, `check.py drag-smoke`, and all consumer controls keep exact existing behavior.
   - `drag` runs the drag-interaction checks; `collision` runs `collision_scenario` + both `qualification_scenario` checks.
   - Each suite writes to its own distinct receipt directory (no receipt path collision when running concurrently) and reports its exact check count (`drag_count + collision_count == 15`).
2. Wire both lanes (`--suite drag` and `--suite collision`) as parallel heads in `scripts/check.py:browser()` while **strictly respecting `BOARDSTUDIO_CHECK_JOBS`** (never exceeding the worker cap).
3. Verify `BOARDSTUDIO_DRAG_BINDING` / verification receipt handling in `scripts/verification_receipt.py` and `scripts/test-verification-receipt.py` accepts both lanes cleanly.

**Risks & guardrails:**
- Total check count across the two lanes must equal **15** with zero dropped assertions, identical failure-mode handling, and identical `HandlerErrors` / `PageErrors` / `service_worker` checks.
- Benchmark `check.py browser` wall time before/after under default `BOARDSTUDIO_CHECK_JOBS` (6) — only keep the split if it measurably beats `102 s` (target: `65–75 s`) without inducing Chrome contention timeouts.

**Verification & acceptance:**
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked` on `tools/browser-harness`.
- `env -u CHROME_BIN python3 scripts/test-drag-browser-deep-home.py` passes.
- `python3 scripts/check.py repo tooling browser` passes with all 15 drag+collision checks, `126` page tests, `799/12` crate tests, `364` CAD browser checks, and `browser` wall reduced to `<= 78 s`.

---

### W3-4: Decompose `boardstudio-core-engine` (114k LOC) & `boardstudio-web-layout` (72k LOC) (Deferred B1)

**Problem & measured evidence:**
In `cpu-opt2-warm` / `cpu-opt2-bypass`, monolithic workspace crates are now the single biggest compile bottleneck:
- **`boardstudio_core_engine` (`114,303 LOC`):** #1 crate in the repo (`208.6 s` CPU warm across 6 units). Its native unit-test binary alone takes **`90.3 s` wall warm / `121.7 s` bypass**, and its lib takes **`54.2 s` wall warm / `83.3 s` bypass** — completely serializing Cargo during `test` and `build`!
  - Top modules: `mirrored_half/` + `mirrored_half.rs` (**`51,401 LOC`**, 45%), `mechanical/` + `mechanical.rs` (**`19,375 LOC`**, 17%), `matrix/` + `matrix.rs` (**`10,538 LOC`**, 9%), `artifact/` (**`9,415 LOC`**, 8%), `modules/` (**`3,764 LOC`**).
- **`boardstudio_web_layout` (`71,811 LOC`):** #4 crate (`81.1 s` CPU across 9 units; `13.8 s` test + `10.5 s` lib on the `build` critical path).
  - Top modules: `objects/` + `objects.rs` (**`42,769 LOC`**, 60%), `outline/` + `outline_lifecycle_*.rs` (**`9,235 LOC`**), `part_placement.rs` (**`4,212 LOC`**), `matrix_transform_operation.rs` (**`3,896 LOC`**).

> [!NOTE]
> Check existing local branch/worktree `refactor/split-core-crate` (`6d3f62929`) and `origin/dev` first to see what crate-split conventions are already established in this repo (`core/crates/{model,script,engine}`).

**Implementation steps (split into two independent sub-PRs: W3-4a Core Engine, W3-4b Web Layout):**
1. **W3-4a — Split `core/crates/engine`:**
   - First extract **`boardstudio-core-mirrored-half`** (or `mirrored-half` + `mechanical` + `artifact` as leaf/mid crates above `boardstudio-model` and shared core helpers), prioritizing the `51.4k LOC` `mirrored_half` subsystem and `19.4k LOC` `mechanical` subsystem so Cargo compiles them in parallel with `matrix` and `artifact`.
   - Keep `boardstudio_core` (`core/src/lib.rs`) as the stable facade re-exporting public types so downstream crates (`application`, `web/*`) need minimal or zero call-site churn.
   - Update `scripts/check.py` / `scripts/check-core-tests.py` (if new crates have tests) and regenerate `scripts/native-test-baseline.json` **only** via `python3 scripts/check.py test --update-native-baseline`, verifying **set-equality** of test coverage (zero dropped tests; account for crate-prefix renames explicitly).
2. **W3-4b — Split `web/crates/layout`:**
   - Extract `objects/` (`42.8k LOC`, e.g., `boardstudio-web-layout-objects` or split transform/inspector vs tree) so `boardstudio-web-layout` and its extracted sibling compile and run browser tests in parallel.
   - Update `scripts/check.py` crate-launch list, `scripts/wasm-test-owners.json` (if applicable), and verify `check-wasm-tests.py` passes with zero raw `launch_virtual_dom` calls.

**Risks & guardrails:**
- Avoid circular dependencies: map the module dependency graph (`cargo modules` or `grep` on `crate::`) before moving files.
- Keep `boardstudio-core-engine` unit tests colocated with their moved modules so unit test binaries also shard across multiple parallel Cargo targets (cutting that `90.3 s` / `121.7 s` serial unit-test compile in half!).

**Verification & acceptance:**
- Full `python3 scripts/check.py` passes (`0` warnings, `0` dropped tests).
- `boardstudio_core_engine` single-unit compile wall drops from `90.3 s` warm / `121.7 s` bypass to `< 45 s` / `< 65 s`.
- `python3 scripts/check.py quick` on a localized `mirrored_half` or `objects` edit runs strictly faster.

---

### W3-5: Deduplicate `cad/monstertruck` & `cad/monstertruck-qualification` Workspaces + CAD Debt (Deferred B4)

**Problem & measured evidence:**
In `cpu-opt2-bypass`, `cad/host/cad-test` costs **`801 s` CPU** (`374 s` warm) — **28.3% of all bypass compile CPU** — because `cad/monstertruck` and `cad/monstertruck-qualification` are two separate Cargo workspaces with separate `target/` directories and lockfiles, compiling the entire vendored geometry stack **twice**:
- `monstertruck_io`: 2 copies (**`98.2 s`** bypass)
- `monstertruck_geometry`: 2 copies (`20.8 s`)
- `monstertruck_modeling`: 2 copies (`20.6 s`)
- `monstertruck_meshing`: 2 copies (`18.7 s`)
- `step_p21`: 2 copies (`8.0 s`)
- `syn`: 5 copies (`21.5 s`), `nom`: 3 copies (`7.0 s`), `zerocopy`: 2 copies (`7.4 s`)

Separately, `cad/monstertruck/Cargo.lock` carries **9 `heapless 0.8.0` advisories** (`check.py security` failure), `cad/monstertruck-qualification/Cargo.lock` still pulls `syn 1.0.109` via `vtkio 0.6.3 → num-derive 0.3.3` plus `nom v3.2.1` / `quick-xml v0.22.0` future-incompat warnings, and `cad/monstertruck/src/step/mod.rs` + `tests/step_import.rs` have pre-existing formatting drift.

**Reference:** Review [docs/plans/cold-build-time/issues/20-merge-cad-workspaces.md](cold-build-time/issues/20-merge-cad-workspaces.md) and `CadWorkspaceTests` in [cad/scripts/test-cad-build.py](../../cad/scripts/test-cad-build.py) before starting.

**Implementation steps:**
1. **Audit feature differences & lockfile conflicts** between `cad/monstertruck/Cargo.toml` and `cad/monstertruck-qualification/Cargo.toml` (especially whether `monstertruck-qualification` still needs `vtkio` or can drop it behind an optional feature, which also eliminates `syn 1.0.109`, `nom 3.2.1`, and `quick-xml 0.22.0`!).
2. **Unify shared vendored crates into one CAD workspace** (or a single shared `cad/` workspace with a clean feature boundary between `boardstudio-cad` production/WASM and qualification test crates):
   - Update `CadWorkspaceTests` in `cad/scripts/test-cad-build.py` and `cad/scripts/cad_build.py` / `cad/scripts/test-cad.py` to match the unified workspace while keeping `test-cad.py` prebuild fast and keeping `cad/wasm` release builds isolated from qualification-only dev dependencies.
   - Check that merging does **not** serialize `cad/wasm` release builds behind native CAD tests (keep `cad/wasm` on its own `target` dir or non-conflicting profile/target lock!).
3. **Clear CAD dependency & hygiene debt while lockfiles are open:**
   - Update `heapless` (and check `zip` / `smartstring` / `rustix` in root workspace if bundled as a companion security PR) so `python3 scripts/check.py security` moves toward green.
   - Format `cad/monstertruck/src/step/mod.rs` and `cad/monstertruck/tests/step_import.rs`.

**Risks & guardrails:**
- **Benchmark gate:** Merging workspaces MUST NOT regress `test` wall time by serializing two parallel CAD Cargo invocations onto one `.cargo-lock` without offsetting compile savings. Measure cold `cad/scripts/test-cad.py` wall & CPU (warm + bypass) before/after.
- Preserve exact CAD test counts (**566 passed / 15 result blocks**), **364** CAD browser checks, `face-completeness-gate.py`, `test-cad-provider.py`, and `test-cold-responses.py`.

**Verification & acceptance:**
- `python3 cad/scripts/test-cad-build.py`, `python3 cad/scripts/test-cad.py` (`566` passed / `0` failed), `python3 scripts/check.py build browser`, and `python3 scripts/check.py release` all pass.
- `cad/host/cad-test` bypass CPU drops by `>= 200 s` with zero manifest warnings.

---

### W3-6: Audit `boardstudio-web-host` (4× Copies) & `web-sys` 183-Feature Scope

**Problem & measured evidence:**
1. **`boardstudio_web_host`** still compiles **4 separate `lib` copies + 1 `test` copy** in `target/wasm32-unknown-unknown/wasm-check/deps/` (`0.3 s` + `5.6 s` + `7.6 s` + `6.7 s` + `7.7 s` = `27.9 s` in `build` alone) because `offline-worker`, `core-worker`, `cad-worker`, `page` (`boardstudio-web`), and `page + test-support` enable different feature combinations on `boardstudio-web-host`.
2. **`web-sys`** pins **183 unified features** in root [Cargo.toml](../../Cargo.toml), producing a **`24.9 MiB` `.rlib`** (`40.4 s` CPU / `56.1 s` wall in `build`, plus `21.8 s` CPU / `26.8 s` wall in `lint`).

**Owned files:**
- [Cargo.toml](../../Cargo.toml)
- [web/Cargo.toml](../../web/Cargo.toml)
- [renderer/Cargo.toml](../../renderer/Cargo.toml)
- `web/crates/host/**`
- [Cargo.lock](../../Cargo.lock)

**Implementation steps:**
1. **Inspect the 4 `boardstudio_web_host` feature sets** in `cpu-opt2-warm.jsonl` (map each of the 4 hashes `2b7d830d`, `a827e6f2`, `220d844e`, `41a42ae5` to its Cargo invocation: `core-worker`, `cad-worker`, `offline-worker`, `page`, `test-support`).
   - Evaluate whether splitting tiny worker-only host glue (`boardstudio-web-worker-host`) from page-only host services (`boardstudio-web-host`) eliminates recompiling the full `host` crate (`5–7 s` each) for workers without pulling page deps into workers.
2. **Audit all 183 `web-sys` features** in root `Cargo.toml`:
   - Script an AST/grep inventory of every `web_sys::*` type actually referenced across `web/`, `renderer/`, and `core/`.
   - Remove any unused `web-sys` features (dead features left behind after prior refactors) and measure whether `web_sys` `.rlib` size and `lint`/`build` CPU drop without re-splitting `web_sys` into multiple hashes.
   - Verify release `.wasm` sizes against `.evidence/cold-build-port/cpu-opt2-wasm-sizes.log` (`<= 0.1%` regression).

**Verification & acceptance:**
- `python3 scripts/check.py repo tooling lint test build browser release` passes.
- `boardstudio_web_host` copies drop from `4` to `<= 2` (or `web_sys` features/compile CPU measurably drops) with **zero** new duplicate crates in `target/wasm32-unknown-unknown/wasm-check/deps/`.

---

## 5. Companion Housekeeping Checklist (Outside Wave 3 Perf Scope)

Track these small items alongside landing PR #28:
- [ ] **Fix PR #28 CI failures:**
  - `scripts/tailwind_tools.py:40` rejecting `BOARDSTUDIO_TAILWINDCSS=/home/runner/work/_temp/web-tools/tailwindcss` in `native` and `web-wasm`.
  - `cad/scripts/test-cad-browser.py` failure in `web-packaged`.
- [ ] **Land or sync `dev` (`2ce123403`) → `origin/dev` (`db9afffd2`)** so PR #28's diff shrinks from 344 files to the cold-build commits.
- [ ] **Security advisories (`check.py security` / CI `audit` job):** Execute **W3-0** (`zip` `RUSTSEC-2026-0548`, `rustix` `RUSTSEC-2026-0486`, and `heapless` 9× `RUSTSEC-2026-0574`..`0582` in `cad/monstertruck/Cargo.lock`), ideally as a dedicated dependency PR or in parallel with Wave 3A.
- [ ] **Host `sccache` wrapper:** Version or document `~/.local/bin/rustc-sccache-fail-open` in-repo if shared across machines.
- [ ] **Prune merged delegate worktrees** once PR #28 lands (leaving `/home/chris/.t3/worktrees/ts-boardstudio2/t3-fdb1fd5f-bc5eb709f18ac2fb` untouched).
