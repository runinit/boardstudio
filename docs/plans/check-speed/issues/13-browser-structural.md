# 13: Browser step structural change (only if 11 shows it is worth it)

Status: needs-info
Type: build
Blocked by: [12](12-browser-quick-wins.md)
Map: [map.md](../map.md)

## Request

If measurements show the per-launch overhead and the triple core build still dominate after 12, choose from:

- **A. One cargo build, direct runner pool.** `cargo test --no-run --message-format=json` across all web crates once (unifies feature sets, one core build,
  no wasm-pack double cargo run), then call `wasm-bindgen-test-runner <wasm> [filter]` directly from the Python pool.
- **B. Pin feature sets** (a hakari-style crate) so the crates unify without changing the launch model.
- **C. One combined test binary with N filter shards**, via a `browser-tests` feature in place of `cfg(test)`. Risks: crate-stripped name collisions and DOM state leaking
  between crates in one page. Keep the `setup_guide::` viewport and the `panels::` / `case_workspace` isolation until cleanup is proven.
- **D. Native or SSR runs** for the roughly 18 headless-VirtualDom tests only; the deep dive found the real `Runtime` is wasm-only, so little else qualifies.

A decision ticket with the user once 11 and 12 have numbers. Mutation checks required for any option: the executed-name multiset equals the baseline; a seeded duplicate name and a leaked DOM node fail the run;
a deliberate panic keeps its location; `cfg` paths that change under unified features are diffed per crate.
