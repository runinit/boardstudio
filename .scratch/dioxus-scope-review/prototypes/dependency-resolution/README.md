# Throwaway dependency-resolution probe

Question: can released Dioxus 0.7.10's minimal web/mounted feature graph and the existing
CoreEngine dependency resolve with wasm-bindgen 0.2.129, js-sys/web-sys 0.3.106
and serde-wasm-bindgen 0.6.5? This standalone package is outside production
packages. It contains no application implementation and changes no existing
manifest or lockfile.

Run from the assessment worktree:

```sh
cargo generate-lockfile --manifest-path .scratch/dioxus-scope-review/prototypes/dependency-resolution/Cargo.toml
```

Scope: dependency resolution only. This does not compile WASM, test the CLI,
worker URLs/initialization, CAD/OCCT linking, browser events or performance.
Use exact tagged official sources for API decisions. Retain the probe here
as evidence; do not copy its manifest or lockfile into production automatically.
No commit, merge or publication is authorized by this probe.

Verdict on 2026-10-01: **pass for dependency resolution only**. The command exits
0 and reports 348 dependency packages against Rust 1.98.0 compatibility; the
lockfile has 349 entries including this standalone probe. All
resolved Dioxus packages are 0.7.10; wasm-bindgen is 0.2.129, js-sys/web-sys
0.3.106, serde-wasm-bindgen 0.6.5 and wasm-bindgen-futures 0.4.79. Lockfile entries
include optional dependencies; their presence is not evidence that a server or
every optional feature is active. No compilation, CLI install or runtime
verification was attempted. Existing production lockfiles remain unchanged.

Exact tagged source requires Dioxus's `mounted` feature for mounted-element
events; `minimal` plus `web` alone does not enable it. The probe was rerun with
`mounted` after verifying that requirement. This proves feature-graph resolution,
not successful canvas event conversion.

The mounted-feature offline resolution attempt exited 101: the available cache
selected wasm-bindgen-futures 0.4.67, requiring js-sys 0.3.94, which conflicts
with the existing 0.3.106 pin. The matching 0.4.79 archive was absent. Repeating
online, without changing any pins, exited 0. This is an unresolved offline
provisioning limit, not proof of Dioxus incompatibility or a passing offline
build. Preserve this failed attempt alongside the successful resolution.
