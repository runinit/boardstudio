# Dependency replacement research

Research date: 2026-10-15. Scope: replacement options for the four unmaintained-crate notices observed in BoardStudio's Cargo audit. These are maintenance advisories, not four active vulnerabilities. The first two candidates have since been trialed in the working tree; other replacement candidates remain research recommendations.

## Recommendations at a glance

| Advisory path | Candidate | Recommendation |
| --- | --- | --- |
| `cgmath 0.18.0` (vendored patch), through `three-d` | `cgmath-next 0.18.1` | Trial uses a locally repackaged source under Cargo's required `cgmath` package identity. Native/WASM checks and the 36-test Miri soundness suite pass with no cgmath compiler warnings. |
| `instant 0.1.13` through `three-d 0.19.0` | `three-d 0.20` using `web-time` | Trial pins development commit `0e5ece5`; native and WASM renderer checks pass. The source remains an unreleased development snapshot. |
| `smartstring 1.0.1` through `rhai 1.26.1` | Rhai `compact_str` feature | Wait for a Rhai release containing the feature, then enable it on both native and WASM dependency declarations. Rhai documents the feature as changing `Map` key type. |
| `ttf-parser 0.25.1` in `cad/wasm` | Google Fonts `skrifa` (`fontations`) | Migrate the small lettering adapter to Skrifa's glyph-outline APIs. Upstream recommends it for new projects; it is not a drop-in API replacement. |

## `cgmath` — [RUSTSEC-2026-0196](https://rustsec.org/advisories/RUSTSEC-2026-0196.html)

BoardStudio patches crates.io `cgmath` to a local package built from `cgmath-next` 0.18.1 so `three-d` and `three-d-asset` retain their expected package identity. The previous swap-only vendor tree is retained but no longer resolved. Renderer code uses the math types re-exported by `three-d`, rather than importing `cgmath` directly ([workspace manifest](../../Cargo.toml), [vendor security notes](../../renderer/vendor/README.md), [renderer manifest](../../renderer/Cargo.toml)).

The strongest low-scope candidate is [`cgmath-next 0.18.1`](https://github.com/kent-tokyo/cgmath-next). Its project documents a stable published release, unchanged `cgmath` library/import name, API parity with upstream 0.18.0, and fixes for both the unmaintained notice and the swap undefined behavior. It is an independently maintained community successor, not an official continuation. It also documents remaining unsafe tuple conversions guarded by runtime layout checks; Rust still provides no tuple layout guarantee. ([release notes](https://github.com/kent-tokyo/cgmath-next/releases/tag/v0.18.1), [unsafe audit](https://github.com/kent-tokyo/cgmath-next/blob/v0.18.1/docs/unsafe-audit.md)).

Cargo detail: the crates.io package is named `cgmath-next` while its library target is named `cgmath`. A direct dependency alias preserves imports, but Cargo patches must retain the dependency package name; `three-d` and `three-d-asset` both request package `cgmath`. ([Cargo patch rules](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html)).

**Trial result:** the pinned 0.18.1 source is locally repackaged with package name `cgmath` and compatibility version `0.18.0`, while its upstream manifest and release hashes are recorded. No-op macro-call `#[inline]` attributes and unreachable undeclared-feature cfgs were removed; native and WASM renderer checks emit no warnings. The upstream soundness suite passes all 36 tests under Miri. The tuple conversion guard panics on measured layout mismatch, but Rust still does not guarantee tuple layout; that remains a documented safety limitation. An upgrade of `three-d` alone does not remove the dependency: its development manifest still declares `cgmath = "0.18"` ([three-d manifest](https://github.com/asny/three-d/blob/master/Cargo.toml)).

## `instant` — [RUSTSEC-2024-0384](https://rustsec.org/advisories/RUSTSEC-2024-0384.html)

The local dependency chain is `boardstudio-renderer -> three-d 0.19.0 -> instant`; BoardStudio pins `three-d` and disables its default features ([renderer manifest](../../renderer/Cargo.toml)). The published 0.19.0 dependency list includes `instant` ([three-d 0.19.0 package metadata](https://docs.rs/crate/three-d/0.19.0)). The upstream development manifest for three-d 0.20.0 uses `web-time` for WASM time instead ([three-d manifest](https://github.com/asny/three-d/blob/master/Cargo.toml)); however, the official released-version listing still identifies 0.19.0 as the published line ([three-d releases](https://github.com/asny/three-d/releases)). The project warns that `master` is work in progress and recommends releases to avoid breakage ([three-d project status](https://github.com/asny/three-d#state-of-the-project)).

**Trial result:** `renderer/Cargo.toml` pins the current development commit `0e5ece5` directly, and `Cargo.lock` records the exact revision. It removes `instant` from the resolved graph in favor of `web-time`; native and WASM renderer checks pass. BoardStudio uses three-d's core/math/rendering APIs in several renderer modules, so this compile result does not cover runtime rendering behavior ([renderer source](../../renderer/src/)). Upstream marks the development branch as work in progress and warns about breaking changes ([three-d project status](https://github.com/asny/three-d#state-of-the-project)).

## `smartstring` — [RUSTSEC-2026-0249](https://rustsec.org/advisories/RUSTSEC-2026-0249.html)

The chain is `boardstudio_core -> rhai 1.26.1 -> smartstring`; BoardStudio's direct Rhai use is currently limited to `Engine` construction in [`core/src/script.rs`](../../core/src/script.rs), while both native and WASM declarations pin the same Rhai release ([core manifest](../../core/Cargo.toml)). Rhai's current upstream manifest has an optional `compact_str` dependency and a `compact_str` feature described as replacing SmartString for identifiers and object-map keys. Its documentation explicitly says this changes the key type of `Map` ([Rhai manifest](https://github.com/rhaiscript/rhai/blob/main/Cargo.toml)). That feature is absent from the checked-in 1.26.1 manifest ([Rhai 1.26.1 manifest](https://github.com/rhaiscript/rhai/blob/v1.26.1/Cargo.toml)); therefore merely adding a Cargo feature today will fail resolution.

**Recommendation:** when Rhai publishes a release with this feature, enable it in both target-specific dependency declarations and update the lockfile. This keeps Rhai and its scripting language while changing its internal compact-string implementation. Exercise saved/user scripts, object-map behavior, and any serialized Rhai values: the documented `Map` key type change is the compatibility edge to verify. Do not replace Rhai with another interpreter for this advisory; that would broaden the migration to script semantics.

## `ttf-parser` — [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html)

`cad/wasm` pins `ttf-parser = 0.25.1` ([CAD manifest](../../cad/wasm/Cargo.toml)). Its sole direct call site is [`cad/wasm/src/model/keycaps/lettering.rs`](../../cad/wasm/src/model/keycaps/lettering.rs): parse the configured font, map characters to glyph IDs, obtain outlines and horizontal advances, and forward outline segments to a local builder. The upstream repository describes `ttf-parser` as maintenance mode and specifically recommends Google's Fontations (`read-fonts` and `skrifa`) for new projects, citing active development and broader table support ([upstream README](https://github.com/harfbuzz/ttf-parser)).

Skrifa is the direct Fontations candidate for this use: it exposes glyph outlines through its own font/glyph APIs and pen interface ([Skrifa README](https://github.com/googlefonts/fontations/blob/main/skrifa/README.md), [outline API](https://github.com/googlefonts/fontations/tree/main/skrifa/src/outline)). It does not preserve `ttf_parser::Face` or the `OutlineBuilder` trait, so this is a small but genuine adapter migration rather than a version bump. Preserve current missing-glyph handling, advance fallback, curve conversion, and downstream tessellation behavior. Check the crate's MSRV and target features against the separately built CAD WASM toolchain before adopting it.

**Recommendation:** plan a focused `ttf-parser` to Skrifa migration around `lettering.rs`; keep the existing input/output contract of the lettering helper and validate bundled fonts, unsupported characters, and generated outlines. A newer `ttf-parser` release alone is not a durable answer to a notice that upstream itself labels unmaintained.

## Suggested order

1. Trial `cgmath-next`: lowest expected source migration, and it also removes the local vendor patch that exists solely to bridge the fixed swap behavior into `three-d`'s dependency graph.
2. Migrate `ttf-parser` to Skrifa: isolated to the lettering adapter, but requires behavior-focused tests and checking the CAD WASM toolchain.
3. Adopt Rhai's `compact_str` feature when published; no replacement is currently declared in the pinned Rhai manifest.
4. Upgrade `three-d` when its `web-time` change ships in a release; until then, retain the working version rather than consuming development branch code.

The replacement choices should be rechecked against current upstream releases at implementation time. This report does not assess unrelated advisories, run alternative builds, or claim that any candidate has been validated in BoardStudio.
