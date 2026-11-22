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
  or runtime invariant carries a comment naming it so later cleanup does not undo it.
- **Audit the full corpus before writing custom parsers or workarounds.** Before extending a custom
  format reader or repair layer (STEP, KiCad, etc.), measure entity/feature coverage across the
  entire asset corpus first; prefer a small permissive crate or general algorithm over per-fixture
  special cases.
- **Delete on cutover—never leave dual paths.** When replacing a subsystem (e.g., sync edits
  replacing queued tickets, derived symmetry replacing post-hoc reconciliation), delete the retired
  path in the same change or immediate follow-up rather than keeping both alive.

## Cargo & build performance

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
  guards (`MountedFixture`, `SavedLocalStorage`).
- **Keep one leaf helper per tooling contract.** Share leaf modules (`tailwind_tools`, `cargo_json`,
  `browser_tools`) instead of duplicating CLI discovery, Cargo JSON parsing, shallow `TMPDIR`/Chrome
  env setup, or asset staging across scripts. Avoid circular imports into `check.py` and keep
  launcher `exec`/output contracts aligned with `scripts/test-check.py`.
