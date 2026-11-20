# Cold Build & Graph Cleanup — Wave 4 Implementation Plan

**Status:** Ready for implementation  
**Baseline:** `28e08ac7f` (`dev`, post PR #28 `perf/cold-build-cpu` & PR #29 `perf/w3-4-split-core-engine-and-web-layout`)  
**Prior docs:** [cold-build-wave3-handoff.md](cold-build-wave3-handoff.md), [cold-build-w3-4-design.md](cold-build-w3-4-design.md)  
**Target executor:** Astra 6.2 (`gpt-6.2-astra`, `reasoningEffort: "low"`) in an isolated worktree branched from `dev` (`28e08ac7f`)

---

## 1. Goals & Scope

Wave 3 (PR #28 + PR #29) cut cold compile CPU by 43–57%, split the `boardstudio-core-engine` (114k LOC) and `boardstudio-web-layout` (72k LOC) monoliths into crate DAGs, lifted `boardstudio-core-engine` off the production web build path (`16 → 1` web crate reverse deps), and removed 3 of the 4 test-only dependency cycles in Core.

Wave 4 closes the **four concrete build-graph and compile-unit leftovers** identified on `dev` at `28e08ac7f`:

| Slice | Target | Problem on `28e08ac7f` | Planned Fix |
|---|---|---|---|
| **W4-1** | `web/Cargo.toml` (`cad-worker`) | `cad-worker` still lists `"dep:boardstudio_core"` even though `web/src/lib.rs` only re-exports `boardstudio_web_worker_host::cad_worker::start_cad_worker` (which uses `kernel` + `model` only). | Remove `"dep:boardstudio_core"` from `cad-worker`; **keep `"dep:serde_json"`** to preserve `serde_json/alloc` unification. |
| **W4-2** | Root `Cargo.toml` (`web-sys`) | Workspace `web-sys` pins **183** features (~25 MiB `.rlib`); active third-party wasm32 crates + direct workspace symbols expand to **159**, leaving **26** candidate features to audit for dead weight. | Compiler-verified pruning of strictly unused `web-sys` features while preserving **exact** feature unification across all 6 wasm targets (`1` `libweb_sys` `.rlib` copy). |
| **W4-3** | `web/crates/runtime` (`test-support`) | `runtime/Cargo.toml` sets `test-support = ["dep:boardstudio_core"]`, and 11 web crates enable it in `[dev-dependencies]`, forcing the entire 14-crate web library DAG to wait behind `engine` during `test` and `combined_wasm_prebuild`. | Extract `CoreEngine`-backed test adapters (`InProcessCore`, `CoreServices::engine()`, `project_name_test_support`, native `TestRuntime`) into a dev-only `web/crates/test-runtime` crate; make `runtime/test-support` 100% engine-free (`test-support = []`). |
| **W4-4** | `core/crates/mirrored-half` tests | Last remaining Core dev-dependency cycle (`mirrored-half` → `engine/test-support`) AND the largest single compile unit in the repo (`~44 s`, 52k LOC / 370 tests in one `--lib --test` binary). | Move the ~250 engine-driving tests out of `mirrored-half/src/` into 4–5 parallel `[[test]]` integration targets under `core/crates/engine/tests/`, drop `mirrored-half`'s dev-deps on `engine` + `script`, and conserve all `2,454` native tests 1-to-1. |

**Non-goals:**
- No runtime behavior changes, no DOM/selector changes, no floating-point arithmetic reordering.
- No public API changes to `boardstudio_core` (`core/src/lib.rs`) or `boardstudio_web_layout` (`web/crates/layout/src/lib.rs`).
- No lowered test counts, no suppressed lints, no `git stash`, no nightly Rust features.

---

## 2. Mandatory Guardrails & Count Floors

Read [AGENTS.md](../../AGENTS.md) and [CODING_STANDARDS.md](../../CODING_STANDARDS.md) before editing.

1. **Shared build lock & shallow `TMPDIR`:** Every Cargo, `check.py`, `cold.py`, and browser invocation MUST hold `~/.cache/tmp/boardstudio-build.lock` with a **blocking** `flock` on fd 9 and export:
   ```bash
   exec 9>"$HOME/.cache/tmp/boardstudio-build.lock"
   flock 9
   export BOARDSTUDIO_BUILD_LOCK_FD=9 TMPDIR="$HOME/.cache/tmp" PATH="$HOME/.cargo/bin:$PATH"
   ```
   Never use fail-fast `flock -n`. Never set `CARGO_TARGET_DIR` when running `scripts/build-time/cold.py`.
2. **Profiles & concurrency:** Never pass `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, or `CARGO_PROFILE_DEV_DEBUG=2`, and never add `rustc-wrapper` to repo `.cargo/config.toml`. Never overlap manual `cargo`/`wasm-pack` runs against the same `target/` directory.
3. **WASM feature unification invariants (verify after W4-1, W4-2, W4-3):**
   - `target/wasm32-unknown-unknown/wasm-check/deps/libweb_sys-*.rlib`: **exactly 1 copy**
   - `target/wasm32-unknown-unknown/wasm-check/deps/libboardstudio_web_host-*.rlib`: **exactly 2 copies**
   - `cargo tree --locked --offline --target wasm32-unknown-unknown` across `page`, `core-worker`, `cad-worker`, `service-worker`, `renderer`, and `page` tests must not introduce new duplicate `.rlib` hashes for `web-sys`, `serde_json`, `boardstudio_core`, or `rhai`.
4. **Hard count floors (must match or exceed in the final full `check.py`):**
   - Native workspace tests: **`2,454`** baselined tests executed (`0` dropped; all `901` Core integration names byte-identical; any crate/module prefix renames in W4-4 proven 1-to-1 by multiset comparison).
   - CAD tests: **`566`** passed across **`15`** result blocks (`0` failed).
   - Browser crate tests: **`799`** passed across **`14`** launches (`0` failed).
   - Page tests: **`126`** passed (`0` failed, `0` incomplete).
   - Drag browser smoke: **`15`** checks passed.
   - CAD browser checks: **`364`** passed.
   - Cargo manifest warnings: **`0`** (`did not match any packages`, ignored workspace-member `resolver`).
   - Release `.wasm` sizes: **no artifact grows by > 0.1%** vs `dev` (`28e08ac7f`).
5. **Evidence hygiene:** Redirect long logs into `.evidence/w4/` (logs only; never create `target/`, crate copies, or `before/`/`after/` trees inside `.evidence/`). Commit cleanly after each slice.

---

## 3. Detailed Slice Plans

Execute slices in order (**W4-1 → W4-2 → W4-3 → W4-4**). Each slice compiles, verifies, and commits independently.

---

### Slice W4-1: Remove Stale `dep:boardstudio_core` from `cad-worker`

**Evidence from `28e08ac7f`:**
- [web/Cargo.toml:24](../../web/Cargo.toml#L24) currently declares:
  ```toml
  cad-worker = ["boardstudio-web-worker-host/cad", "dep:boardstudio_core", "dep:serde_json"]
  ```
- [web/src/lib.rs:11-12](../../web/src/lib.rs#L11-L12) only contains:
  ```rust
  #[cfg(all(target_arch = "wasm32", feature = "cad-worker"))]
  pub use boardstudio_web_worker_host::cad_worker::start_cad_worker;
  ```
- [web/crates/worker-host/Cargo.toml:10](../../web/crates/worker-host/Cargo.toml#L10) declares `cad = ["dep:boardstudio-core-kernel", "dep:boardstudio-model", "dep:serde_json"]`.
- `cargo tree --locked --offline --target wasm32-unknown-unknown -p boardstudio-web --no-default-features --features cad-worker --depth 2` confirms `boardstudio_core` is pulled in **solely** by `web/Cargo.toml:24`, dragging `boardstudio-core-engine`, `boardstudio-core-mirrored-half`, `boardstudio-core-artifact`, `boardstudio-core-mechanical`, and `boardstudio-script` into the `cad-worker` compile graph.

**Critical trap to avoid:**
- **KEEP `"dep:serde_json"` on `cad-worker`!** [web/Cargo.toml:46](../../web/Cargo.toml#L46) enables `serde_json = { version = "1", features = ["float_roundtrip", "alloc"], optional = true }` so every worker and the page unify `serde_json` with `wasm-bindgen-test`'s `alloc` feature. Removing `"dep:serde_json"` from `cad-worker` would drop `alloc` and create a duplicate `serde_json` `.rlib` hash.

**Steps:**
1. Edit [web/Cargo.toml:24](../../web/Cargo.toml#L24):
   ```toml
   cad-worker = ["boardstudio-web-worker-host/cad", "dep:serde_json"]
   ```
2. Verify via `cargo tree`:
   - `cargo tree --locked --offline --target wasm32-unknown-unknown -p boardstudio-web --no-default-features --features cad-worker --depth 2` shows **zero** `boardstudio_core` / `boardstudio-core-engine` / `boardstudio-core-mirrored-half` / `boardstudio-script`.
   - `cargo tree --locked --offline --target wasm32-unknown-unknown -p boardstudio-web --no-default-features --features cad-worker -e features -i serde_json` still includes `alloc`, `default`, `float_roundtrip`, `std` (identical to `page` and `core-worker`).
3. Verify compilation under the build lock:
   - `cargo check --locked --target wasm32-unknown-unknown --profile wasm-check -p boardstudio-web --no-default-features --features cad-worker`
   - `cargo clippy --locked --target wasm32-unknown-unknown -p boardstudio-web --no-default-features --features cad-worker -- -D warnings`
4. Commit Slice W4-1.

---

### Slice W4-2: Audit & Prune Strictly Dead `web-sys` Features

**Evidence from `28e08ac7f`:**
- Root [Cargo.toml](../../Cargo.toml) currently pins **183** `web-sys` features.
- Active wasm32 third-party crates across all 6 targets (`page`, `core-worker`, `cad-worker`, `service-worker`, `renderer`, `page` tests) directly request **100** features (`glow` 51, `dioxus-web` 25, `wasm-streams` 23, `dioxus-asset-resolver` 4, `subsecond` 4, `three-d-asset` 2, `dioxus-interpreter-js` 1), which expand via `web-sys`'s internal feature graph to **106** features.
- Workspace code directly names **62** `web_sys::` feature identifiers, expanding to **65** features.
- Union `(active TP | direct WS)` expanded = **159** features.
- **Exact 26 candidate features in root `Cargo.toml` outside that 159-feature union:**
  - *Likely implicitly needed via DOM/IDB/SW/Window method signatures (verify by compiler):* `Clients`, `CssStyleDeclaration`, `DomException`, `FocusEvent`, `IdbFactory`, `IdbObjectStore`, `Location`, `Navigator`, `ScrollBehavior`, `ScrollIntoViewOptions`, `ScrollLogicalPosition`, `ServiceWorker`, `Text`, `WheelEvent`.
  - *Likely dead (zero workspace text hits, not requested by active third-party crates):* `CharacterData`, `Comment`, `FileReader`, `HtmlFormElement`, `HtmlHeadElement`, `HtmlImageElement`, `HtmlMediaElement`, `HtmlVideoElement`, `InputEvent`, `TouchEvent`, `TransitionEvent`, `WebSocket`.

**Critical trap to avoid:**
- **NEVER remove a feature that ANY active third-party crate enables in ANY of the 6 wasm targets!** For example, `glow` (only in `renderer`) enables 51 WebGL features, and `wasm-streams` / `dioxus-web` (only in `page`) enable stream/DOM features. Root `Cargo.toml` intentionally pins their union so `renderer`, `page`, workers, and browser crate tests all share **one single `libweb_sys-*.rlib`**! Splitting `web-sys` into 2+ hashes would *add* `+40 s` CPU per extra copy.

**Steps:**
1. Record the baseline `cargo tree --locked --offline --target wasm32-unknown-unknown <target> -e features -i web-sys` feature list across all 6 active targets (`page`, `core-worker`, `cad-worker`, `service-worker`, `renderer`, and `combined_wasm_prebuild` crate+page test targets) and baseline `libweb_sys-*.rlib` byte size (if present).
2. Remove the 12 zero-hit candidates first (`CharacterData`, `Comment`, `FileReader`, `HtmlFormElement`, `HtmlHeadElement`, `HtmlImageElement`, `HtmlMediaElement`, `HtmlVideoElement`, `InputEvent`, `TouchEvent`, `TransitionEvent`, `WebSocket`), plus test each of the remaining 14 candidates by removing them and running `cargo check` across **all 6 wasm targets + all 14 browser crate test targets + `cargo clippy`** under the build lock. Restore every feature that the compiler requires (either for a method signature, trait impl, or `dioxus-web`/`wasm-bindgen-test` target).
   - **Important:** Also check `web/examples/case-settings` and all `#[cfg(test)]` browser test modules (`BROWSER_CRATE_BUILD` + `PAGE_TEST_BUILD`) so no test-only `web_sys` method breaks!
3. **Mandatory unification check:** Run `cargo tree --locked --offline --target wasm32-unknown-unknown <args> -e features -i web-sys` on all 6 targets and assert the resolved `web-sys` feature set is **100% identical across every single target** (ignoring `default`, which is empty/shared) and that after a `wasm-check` build, `ls target/wasm32-unknown-unknown/wasm-check/deps/libweb_sys-*.rlib | wc -l` is **exactly `1`**.
4. **Stop/keep rule:** Keep the pruned list if zero targets split `web-sys` and all checks pass cleanly. Record before/after feature count and `.rlib` size in the Outcome section. If fewer than 1 feature can be safely pruned without splitting, record that finding honestly and move on to W4-3.
5. Commit Slice W4-2 (if changes are kept).

---

### Slice W4-3: Decouple `boardstudio-web-runtime/test-support` from `boardstudio_core`

**Problem diagnosis on `28e08ac7f`:**
- [web/crates/runtime/Cargo.toml:12](../../web/crates/runtime/Cargo.toml#L12) declares:
  ```toml
  test-support = ["dep:boardstudio_core"]
  ```
- Eleven web crates (`case`, `keycaps`, `keymap`, `layout`, `layout-core`, `layout-objects`, `library`, `parts`, `pcb`, `ui-shared`, `web`) enable `boardstudio-web-runtime = { path = "...", features = ["test-support"] }` in `[dev-dependencies]`.
- Because Cargo unifies features across `--workspace` (`cargo test --workspace`) and across `combined_wasm_prebuild`, **enabling `boardstudio-web-runtime/test-support` makes `boardstudio-web-runtime` itself depend on `boardstudio_core` (`engine`, `mirrored-half`, `artifact`, `script`)**. Consequently, **all 14 web crate libraries** (`runtime`, `ui-model`, `ui-shared`, `catalogue`, `keycaps`, `keymap`, `parts`, `case`, `pcb`, `layout-core`, `layout-objects`, `layout`, etc. — ~200k LOC) sit idle behind `boardstudio-core-engine` during `test` and `combined_wasm_prebuild`.
- Inspecting callers confirms they genuinely run real in-process `Session + CoreEngine` edits via `project_name_test_support::{new_runtime, open_document, install, ...}`, `in_process_support::{InProcessCore, InProcessAdapters, board_document}`, `CoreServices::engine()`, and native `runtime_test_stub.rs`.

**Target architecture (Dependency Inversion for Test Wiring):**
Mirror the exact `CoreServices` + `CoreExecutor` injection that W3-4 used for production:

```text
BEFORE (test/browser builds):
  boardstudio-core-engine -> boardstudio_core -> boardstudio-web-runtime (test-support) -> ALL 14 web crate libs -> test binaries

AFTER (test/browser builds):
  Parallel branch A: boardstudio-model + kernel + mechanical -> boardstudio-web-runtime (test-support = []) -> ALL 14 web crate libs
  Parallel branch B: boardstudio-core-mirrored-half + artifact -> boardstudio-core-engine -> boardstudio_core
  Join (thin leaf):  boardstudio-web-test-runtime (~700 LOC; depends on runtime[test-support] + boardstudio_core) -> test binaries only
```

**Concrete design & steps:**
1. **Separate engine-free hooks from `boardstudio_core` adapters in `web/crates/runtime`:**
   - Keep `test-support = []` in [web/crates/runtime/Cargo.toml](../../web/crates/runtime/Cargo.toml) (remove `"dep:boardstudio_core"` from `test-support` and remove `boardstudio_core = { ..., optional = true }` from `[dependencies]`).
   - Keep the **engine-free** test hooks inside `boardstudio-web-runtime` under `#[cfg(any(test, feature = "test-support"))]`:
     - `gate_driver` (`GateDriver`, `OneShotBehavior`)
     - `case_gesture_preview_test_executor`, `keycaps_preview_test_executor`, `held_effects`, `fail_storage_initialization`, `submit_fixed_command`, `run_pending`, `replace_document` (everything that uses only `Session`, `CoreExecutor`, `CoreServices`, `BrowserStore`, `ProjectDoc`)
     - Generic injection hooks on `Runtime` (wasm32): e.g., `Runtime::install_test_session_and_adapters(session, core_executor, persist_port, core_services)` or expose `pub(crate)`→`pub` (under `#[cfg(any(test, feature = "test-support"))]`) the `Runtime` fields/attach methods that `InProcessAdapters::attach(runtime)` needs (without referencing `boardstudio_core` in `runtime`!).
     - For native tests (`#[cfg(all(not(target_arch = "wasm32"), any(test, feature = "test-support")))]`): generalize `runtime_test_stub.rs`'s `Runtime` so it takes `(OpenValidator, Box<dyn FnMut(CoreRequest) -> CoreReply>)` (or `CoreServices` + a trait/closure handler) instead of hardcoding `boardstudio_core::CoreEngine`! That keeps the exact `boardstudio_web_runtime::runtime::Runtime` type name and methods (`model()`, `scope()`, `submit()`, `events`, `operation()`, `observe_operation()`, `core_services()`, gate methods) intact across every native web crate test!
2. **Create dev-only helper crate `web/crates/test-runtime` (`boardstudio-web-test-runtime`):**
   - `edition = "2024"`, `publish = false`, `[lints] workspace = true`, `web-sys = { workspace = true }` (copying exact target-gated deps from `runtime` so zero new `.rlib` hashes appear!).
   - Add `"web/crates/test-runtime"` to root [Cargo.toml](../../Cargo.toml) `members` (and do **NOT** add it to `BROWSER_CRATE_BUILD` launches in `scripts/check.py` because it owns 0 tests — verify `scripts/check-wasm-tests.py` and `scripts/test-check.py` stay green).
   - Move into `boardstudio-web-test-runtime`:
     - `CoreServices::engine()` factory (`pub fn engine_core_services() -> CoreServices`)
     - `in_process_support.rs` (`InProcessCore`, `InProcessAdapters`, `board_document()`, etc.)
     - `project_name_test_support` wrappers that instantiate `boardstudio_core::CoreEngine` (`new_runtime()`, `install()`, `open_document()`, plus re-exporting all the engine-free helpers from `boardstudio_web_runtime` so caller churn is minimal!)
     - Native `pub fn new_native_runtime() -> Rc<boardstudio_web_runtime::runtime::Runtime>` constructing `runtime_test_stub::Runtime::with_core(...)` backed by `boardstudio_core::CoreEngine`.
3. **Update `[dev-dependencies]` and test imports across `web/` and `web/crates/*`:**
   - Replace `project_name_test_support` / `in_process_support` / `CoreServices::engine()` / native `Runtime::new()` calls in tests of `case`, `keycaps`, `keymap`, `layout`, `layout-core`, `layout-objects`, `library`, `parts`, `pcb`, `ui-shared`, `web` to import from `boardstudio_web_test_runtime` (or via a `#[cfg(test)] use boardstudio_web_test_runtime as ...` shim in each crate's `src/lib.rs` to keep individual test files clean and avoid changing any test names!).
   - **Check `web/crates/runtime`'s own unit tests:** `runtime` keeps `boardstudio_core` in its own `[dev-dependencies]` ONLY if its own inline tests (`cad_runtime_borrow_tests.rs`, `runtime.rs`, `source_pcb_export_tests.rs`, `layout_viewer_source.rs`) need `CoreEngine`; better yet, check if `runtime` still compiles its **library** (`rlib`) completely ahead of `boardstudio_core`! (In Cargo, a crate's `[dev-dependencies]` NEVER block its `lib` target from compiling — ONLY `[dependencies]` and feature-activated optional `[dependencies]` do!)
   - Check `web/crates/{case,catalogue,host,layout,layout-core,layout-objects,library,pcb,ui-model}`: keep `boardstudio_core` in `[dev-dependencies]` only where those crates directly reference `boardstudio_core::*` in tests, and remove it wherever `boardstudio-web-test-runtime` is the only consumer.
4. **Verify build-graph & feature invariants (under the build lock):**
   - `cargo tree --locked --offline --target wasm32-unknown-unknown -p boardstudio-web-runtime --features test-support` MUST show **ZERO** `boardstudio_core` or `boardstudio-core-engine`!
   - `cargo tree --locked --offline -p boardstudio-web-runtime --features test-support` (host) MUST show **ZERO** `boardstudio_core` or `boardstudio-core-engine`!
   - `cargo tree --locked --offline -i boardstudio-core-engine -e normal` (host & wasm) still reaches ONLY `boardstudio-web`.
   - `cargo check --workspace --tests --locked` and `python3 scripts/check.py repo tooling lint test build browser` pass cleanly with `libweb_sys` = 1 copy, `libboardstudio_web_host` = 2 copies, `799` browser crate tests across `14` launches, `126` page tests, `2,454` native tests (`0` test name changes!).
   - Verify `scripts/check.py`'s `combined_wasm_prebuild()` still activates (does not fall back to `None`).
5. Commit Slice W4-3.

---

### Slice W4-4: Break the `mirrored-half` Test Cycle & Shard Its 44 s Test Unit

**Evidence from `28e08ac7f`:**
- `core/crates/mirrored-half` is **51,969 LOC** (`~22k` prod + `~30k` tests, **370 `#[test]`s**), compiling as ONE single `--lib --test` binary (~44.5 s warm / ~42.0 s bypass) that cannot even start compiling until `mirrored-half` (lib) → `boardstudio-core-engine` (lib) finishes.
- Exact file-by-file engine-dependency inventory measured on `28e08ac7f`:

| File under `core/crates/mirrored-half/src/` | LOC | `#[test]` | Engine refs | Destination |
|---|---:|---:|---:|---|
| `mirrored_half/derive.rs` | 364 | 3 | **0** | **Keep** in `mirrored-half` lib tests |
| `mirrored_half/inverse/hosted_tests.rs` | 1,310 | 25 | **0** | **Keep** in `mirrored-half` lib tests |
| `mirrored_half/reconcile/outlines/tests.rs` | 1,714 | 15 | **0** | **Keep** in `mirrored-half` lib tests (extract the 3 pure fixture fns it uses from `reconcile/tests.rs` into a tiny `#[cfg(test)] mod test_fixtures` in `reconcile.rs`) |
| `mirrored_half/inverse/tests.rs` | 3,270 | 78 | **1** (`line 3147`) | **Keep** 77 pure tests in `mirrored-half`; move the 1 `apply_for_test` test to `engine/tests/mirrored_misc.rs` |
| `mirrored_half/reconcile/settings_tests.rs` | 508 | 17 | **1** (`line 143`) | **Move** to `core/crates/engine/tests/mirrored_matrix_addition.rs` |
| `mirrored_half/matrix_addition/tests.rs` | 1,175 | 26 | **4** | **Move** to `core/crates/engine/tests/mirrored_matrix_addition.rs` |
| `mirrored_half/matrix_addition/reconcile_tests.rs` | 157 | 1 | **2** | **Move** to `core/crates/engine/tests/mirrored_matrix_addition.rs` |
| `mirrored_half.rs` (inline `#[cfg(test)]`) | ~400 test LOC | 8 | **9** | **Move** engine-using tests to `core/crates/engine/tests/mirrored_misc.rs` (keep any pure ones in `mirrored_half.rs`) |
| `mirrored_half/reconcile/prepare/tests.rs` | 2,972 | 57 | **21** | **Move** to `core/crates/engine/tests/mirrored_prepare.rs` (fix `CURRENT_VERSION` import to `boardstudio_core_kernel::migrate::CURRENT_VERSION`) |
| `mirrored_half/lifecycle/tests.rs` | 6,171 | 62 | **86** | **Move** to `core/crates/engine/tests/mirrored_lifecycle.rs` |
| `mirrored_half/reconcile/tests.rs` | 12,542 | 78 | **58** | **Move & shard** into **2** targets: `core/crates/engine/tests/mirrored_reconcile_a.rs` + `core/crates/engine/tests/mirrored_reconcile_b.rs` (or `mirrored_reconcile.rs` + `mirrored_reconcile_transitions.rs`, ~6k LOC each, sharing a tiny `core/crates/engine/tests/support/mirrored_fixtures.rs` module!) |

**Critical design rules:**
1. **NO new Cargo feature flag on `boardstudio-core-mirrored-half`!** Adding a `test-internals` feature flag to `mirrored-half` would cause `mirrored-half` to compile TWICE in `cargo test --workspace` (once with and once without the feature). Because `boardstudio-core-mirrored-half` is an internal `publish = false` crate behind the unchanged `boardstudio_core` facade, widen only the ~20–30 compiler-demanded internal items (`BaselineKind`, `FinalOwner`, `HostedTransition`, `IndependentPartTransition`, `MatrixLink`, `MembershipContext`, `MembershipTransitionInput`, `PartProjection`, `PlannedAddition`, `Projection`, `TransitionAssociation`, `TransitionState`, `changed_document_fields`, `prepare_membership_transition`, `prepare_membership_transition_kernel`, `reconcile`, `ConstraintLifecycleEffect`, `prepare_constraint_lifecycle_delta`, `retire_replacement_records`, `project_matrix`, `project_part`, `prepare_transaction`, etc.) to `#[doc(hidden)] pub` — following the exact precedent of W3-4a (`core/crates/model` solver frame ops). Verify `core/src/lib.rs` (the `boardstudio_core` facade) exports **zero** new items.
2. **Engine test helpers:** Look at [core/crates/engine/src/lib.rs:1668-1710](../../core/crates/engine/src/lib.rs#L1668-L1710) (`apply_with_ledger_for_test`, `apply_unscripted_for_test`, `apply_for_test`, `CoreEngine::with_document_for_test`). Once `mirrored-half` drops its dev-dependency on `boardstudio-core-engine`, **no other crate in the workspace enables `boardstudio-core-engine/test-support`!**
   - Make those 4 small wrapper functions `#[doc(hidden)] pub` on the internal `publish = false` `boardstudio-core-engine` crate (so `core/crates/engine/tests/*.rs` can call them directly without needing a `test-support` feature on `engine` at all — eliminating any risk of `engine` ever compiling with two feature hashes!), and delete `test-support = []` from `core/crates/engine/Cargo.toml`!
   - Wait: verify via `grep -rn "test-support" core/` that nothing else enables `boardstudio-core-engine/test-support` before removing it.
3. **Remove the dev-dependency cycle completely:**
   - Edit [core/crates/mirrored-half/Cargo.toml](../../core/crates/mirrored-half/Cargo.toml): remove `boardstudio-core-engine` and `boardstudio-script` from `[dev-dependencies]` (also check if `serde` / `serde_json` in `[dev-dependencies]` are redundant with `[dependencies]` — remove duplicate dev-dep entries if identical to keep manifests clean).
   - Delete `pub fn test_dispatch()` and `pub fn apply_for_test()` from `core/crates/mirrored-half/src/mirrored_half.rs:64-84`; place them in `core/crates/engine/tests/support/mirrored_fixtures.rs` instead!
   - Note `reconcile/prepare/tests.rs:12` has `#[path = "../../../../../../tests/support/synthetic_half.rs"]` — adjust its relative path from `core/crates/engine/tests/` to `../../../tests/support/synthetic_half.rs`!
4. **Shared fixtures between moved test targets:**
   - Put shared test fixtures (`fixture_mapping`, `fixture_board`, `fixture_document`, `pair_document`, `create_from`, `halves`, `matrix`, `source_half`, `test_dispatch`, `apply_for_test`) in `core/crates/engine/tests/support/mod.rs` (imported via `mod support;` in each `mirrored_*.rs` integration target — note `scripts/check-core-tests.py` only inspects `core/tests/` and `footprints/tests/`, NOT `core/crates/engine/tests/`, and `core/crates/engine` uses default `autotests = true`, so `tests/support/mod.rs` is automatically a helper module, not a test binary!).
   - Double-check `core/crates/engine/Cargo.toml` has `boardstudio-script` in `[dev-dependencies]` (already present) and `boardstudio-core-mirrored-half` in `[dependencies]` (already present).
5. **1-to-1 Test Conservation Proof (MANDATORY):**
   - Before regenerating the baseline, save the pre-W4-4 baseline:
     `git show HEAD:scripts/check-native-tests-baseline.json > .evidence/w4/baseline-before-w4-4.json`
   - Run `python3 scripts/check.py test --update-native-baseline` under the build lock.
   - Write & run a Python verification script (output saved to `.evidence/w4/w4-4-test-conservation.log`) proving:
     - Total baselined native tests before == after == **`2,454`** (`0` dropped, `0` unintended additions).
     - All `901` `boardstudio_core` integration test names are byte-identical.
     - Every test name removed from `boardstudio_core_mirrored_half` appears **1-to-1** in the new `boardstudio_core_engine` integration targets (`mirrored_reconcile_*`, `mirrored_lifecycle`, `mirrored_prepare`, `mirrored_matrix_addition`, `mirrored_misc`) after stripping the expected module-prefix rename.
6. Verify `cargo tree --locked --offline -p boardstudio-core-mirrored-half` shows **zero** `boardstudio-core-engine` or `boardstudio-script` edges (even with dev-deps!).
7. Commit Slice W4-4.

---

## 4. Final Verification & Reporting Gate

After all 4 slices are committed, run under the shared blocking build lock (`exec 9>"$HOME/.cache/tmp/boardstudio-build.lock"; flock 9; export BOARDSTUDIO_BUILD_LOCK_FD=9 TMPDIR="$HOME/.cache/tmp" PATH="$HOME/.cargo/bin:$PATH"`):

1. **Full gate:**
   ```bash
   python3 scripts/check.py > .evidence/w4/final-check.log 2>&1
   python3 scripts/check.py security deny release > .evidence/w4/final-security-release.log 2>&1
   ```
2. **Verify every count floor and invariant in the logs:**
   - Native baseline: `2,454` executed (`0` dropped, `0` failed)
   - CAD tests: `566` passed / `15` blocks
   - Browser crate tests: `799` passed / `14` launches
   - Page tests: `126` passed
   - Drag smoke: `15` checks passed
   - CAD browser: `364` passed
   - Manifest warnings: `0`
   - `ls target/wasm32-unknown-unknown/wasm-check/deps/libweb_sys-*.rlib | wc -l` == `1`
   - `ls target/wasm32-unknown-unknown/wasm-check/deps/libboardstudio_web_host-*.rlib | wc -l` == `2`
   - `cargo tree -i boardstudio-core-engine -e normal` (host & wasm) reaches ONLY `boardstudio-web`
   - `cargo tree -p boardstudio-web-runtime --features test-support` (host & wasm) has ZERO `boardstudio_core` / `boardstudio-core-engine`
   - `cargo tree -p boardstudio-core-mirrored-half` (including dev-deps) has ZERO `boardstudio-core-engine` / `boardstudio-script`
   - Release `.wasm` sizes compared against `28e08ac7f` (`<= 0.1%` growth on every artifact)
3. **Update this plan document:** Append a `## 5. Implementation Outcome` section summarizing every slice's commits, before/after metrics (web-sys feature count & `.rlib` size, `mirrored-half` vs sharded engine integration test counts, release `.wasm` deltas, and verification results).
4. **Stop before pushing or opening a PR** (report final branch, HEAD SHA, commit list, and verification summary cleanly back to the parent thread).
