# Coding standards

Read when authoring or reviewing code. Mechanical rules live in `python3 scripts/check.py`; this
file holds the judgement calls that no check can make.

## Code & contracts

- **Reuse the helper.** When the module or its neighbours already implement the geometry or
  arithmetic (`rotate` in `matrix/splay.rs`), call it and widen visibility if needed. A second
  hand-rolled copy drifts the first time one of them changes.
- **Keep the operation order of saved arithmetic.** Poses are persisted and hashed. Preserve
  floating-point evaluation order (`a + (b - c)` can differ from `(a + b) - c` in the last bit)
  and pin it with an exact-equality test.
- **Say why a pinned contract is pinned.** Any variant, flag, `debug-assertions` exception
  (`dioxus-cli-config`), or branch kept for a golden fixture (`core/tests/fixtures/boundary/transcript.json`)
  or runtime invariant carries a comment naming it so later cleanup does not undo it. Never delete
  invariant or "why" comments to pad LOC reductions (`check_reduction.py` excludes comments/blank lines).
- **Audit the full corpus before writing custom parsers or workarounds.** Before extending a custom
  format reader or repair layer (STEP, KiCad, etc.), measure entity/feature coverage across the
  entire asset corpus first; prefer a small permissive crate or general algorithm over per-fixture
  special cases.
- **Delete on cutover—never leave dual paths.** When replacing a subsystem (e.g., sync edits
  replacing queued tickets, derived symmetry replacing post-hoc reconciliation), delete the retired
  path in the same change or immediate follow-up rather than keeping both alive.
- **A fold is not a simplification.** Merging files verbatim or renaming a controller into its view
  removes zero complexity; simplify by deleting redundant state layers, prop-bags, and obsolete
  indirection.
- **Synchronous `Runtime::commit` needs no async settlement ceremony.** Handle
  `Result<AcceptedSnapshot, String>` inline at the call site; do not keep `PendingSettlement`,
  polling `use_effect`s, `committing` flags, generation/request-ID guards, or `ProjectionIdentity`
  memo mirrors for synchronous edits. Reserve a minimal counter/token strictly for genuinely async
  I/O (`DefinitionLoader`, worker jobs, storage).
- **Keep UI event adapters thin and one-way.** Normalize DOM events into pure `CanvasInteraction`
  events without cloning large `*Input` prop-bags or duplicating state in parallel `RefCell`s;
  dependencies flow downward (`workspace -> editor/canvas_adapter`, never
  `workspace -> workbench/editor_session`).
- **No panicking unwraps in production core or runtime.** Non-test code across `core/crates/*` and
  `web/crates/runtime` enforces `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]`;
  propagate typed errors cleanly rather than trapping the browser WASM instance.
- **RAII DOM listeners only in UI code.** Never leak `Closure::wrap` + `.forget()` listeners in
  Dioxus components; bind window/document listeners through `ui_shared::DomEventListener` (or
  `host::renderer_host::Listener`-style RAII guards). Only permanent worker globals
  (`service_worker.rs`, `cad_worker.rs`) and self-releasing `Closure::once_into_js` are exempt.
- **Shared 3D viewport lives below workspaces.** `<Viewport3D />` and shared camera/scene helpers
  live in `web/crates/ui-shared/src/viewport_3d/`. Workspace crates (`case`, `parts`, `layout`,
  `pcb`, `keycaps`) must not depend sideways on `case`.
- **Keep pre-paint theme and WCAG AA contrast in sync.** Any palette edit in `web/tailwind.css`
  must pass `scripts/check-theme-contrast.py` across all four Theme × Appearance combinations and
  keep `FIRST_PAINT_THEME_HEAD` (`--canvas`) aligned in `scripts/build-web.py`.

## Cargo & build performance

- **Avoid redundant generic/component monomorphizations in release WASM.** Prefer concrete helpers
  and non-generic inner views over deeply parameterized Dioxus components so release `.wasm` stays
  lean alongside `strip = "debuginfo"` and `lto = "thin"`.

- **Keep WASM dependency features unified.** Never add a private `web-sys` feature list (use
  `web-sys = { workspace = true }`). Before changing features or dependencies in `web/`,
  `web/crates/*`, or `renderer/`, verify with `cargo tree --locked --offline --target wasm32-unknown-unknown`
  across `page`, workers, and `renderer` that no leaf or host proc-macro feature diff duplicates
  Core or `web-sys` builds.
- **Avoid contagious features for local needs.** Do not enable graph-wide Cargo features
  (`serde_json/preserve_order`, broad codec defaults, oversized protocol crates) when a local
  helper or explicit sort suffices; Cargo unifies features across the build graph.
- **Audit user-facing surfaces when stripping crate features.** Passing existing tests is not proof
  a disabled feature is unused. Check what user syntax, formats, or saved files a feature gates and
  add a regression test for the boundary kept.
- **Benchmark build, profile, and sharding changes end-to-end.** Measure compile CPU, test/browser
  wall time, and `check.py release` `.wasm` sizes together. Reject changes that trade compile time
  for worse runtime or reload overhead.

## Tests & tooling

- **Native-first tests.** Pure controller, view-model, projection, and Dioxus `VirtualDom` tests
  belong in native `#[test]`. Reserve `#[wasm_bindgen_test]` and headless Chrome for genuine
  DOM/pointer/WebGL/worker/IndexedDB boundaries.

- **Match surrounding tests and isolate shared browser state with RAII.** Follow sibling fixture
  shape and naming. Because `wasm-bindgen-test-runner` shares one document and WASM memory per
  crate, own mounted DOM roots, injected `<style>` nodes, and modified `localStorage` through RAII
  guards that tear down the render task and restore the shared state.
- **Keep one leaf helper per tooling contract.** Share leaf modules (`tailwind_tools`, `cargo_json`,
  `browser_tools`) instead of duplicating CLI discovery, Cargo JSON parsing, shallow `TMPDIR`/Chrome
  env setup, or asset staging across scripts. Avoid circular imports into `check.py` and keep
  launcher `exec`/output contracts aligned with `scripts/test-check.py`.
