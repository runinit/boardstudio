# M1 service-worker policy slice

The maintained `boardstudio-web` crate now has a `service-worker` feature that compiles the Rust policy without `boardstudio_core`, `boardstudio-application`, or Dioxus. The page feature exports `boardstudio_web::host::register_offline(scope)`; pass `/` or `/boardstudio/`. It registers `${scope}service-worker.js` as a same-origin module worker with that exact scope.

`web/build.rs` consumes `BOARDSTUDIO_OFFLINE_MANIFEST`, a JSON object `{ "version": "<release-id>", "assets": ["index.html", "assets/...", ...] }`, and emits validated Rust constants. Paths must be unique, normalized, scope-relative and include `index.html`. The service worker precaches every listed required asset before activation, uses a cache name containing an exact hex encoding of the registration path plus the build version, looks up only in that registration's cache, deletes only old versions for the same path, and claims clients after activation. Document navigations fall back to the cached shell. Other missing paths use network fetch and reject offline when unavailable.

`node scripts/web/embed-worker-wasm.mjs <wasm-pack-dir> <manifest.json> <site>/service-worker.js` embeds the release WASM into a generated synchronous initializer and places the wasm-bindgen module alongside it. Both `service-worker.js` and `boardstudio_offline_worker.js` belong in the staged release manifest. The initializer contains no cache, routing, lifecycle or fallback policy.

## Verification

- `cargo test --manifest-path web/Cargo.toml --locked`: 6 passed, including path namespace and strict manifest validation.
- `cargo clippy --manifest-path web/Cargo.toml --locked --all-targets -- -D warnings`: passed.
- `BOARDSTUDIO_OFFLINE_MANIFEST=/tmp/m1-offline-manifest-test.json cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features service-worker -- -D warnings`: passed.
- `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --features page -- -D warnings`: passed.
- `BOARDSTUDIO_OFFLINE_MANIFEST=/tmp/m1-sw-test/manifest.json wasm-pack build web --target web --release --out-dir /tmp/m1-sw-test/pkg --out-name boardstudio_offline_worker --no-typescript --no-pack -- --no-default-features --features service-worker --locked`, followed by the embed script: passed; optimized WASM was 64,773 bytes.
- Actual Chromium 153.0.8010.52 probe results: [`browser-probe.json`](browser-probe.json). It verifies root and `/boardstudio/` install, cached offline document reopen, uncached offline fetch rejection (`TypeError`), isolation from a same-URL response in an unrelated cache, and a root v2-to-v3 update that removes only the root v2 cache while preserving the subpath v1 cache.

The worker-only dependency graph contains no `boardstudio_core`, `boardstudio-application`, Dioxus, `serde_json` runtime or SHA-256 dependency. The browser probe uses a minimal temporary static shell, not the integrated Dioxus production bundle. The parent integration build still needs to provide a complete root and subpath manifest, stage the worker artifacts, call `register_offline`, and rerun the checks against its exact production candidate. The test did not force-kill a service-worker process, perform full application archive exchange, or establish cold offline success before first online install.
