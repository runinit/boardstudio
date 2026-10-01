# P3 durability feasibility evidence

The final matrix passed against integration base `dfbfcb16` after merging it into the P3 branch. The P3 source commit is `efa2bc4a`; merge commit is `9efd6711`. The exact-source command was `./scripts/run.sh`, with command-by-command output and exits in `evidence/logs/20261001T202535Z/`. It includes native fmt/test/strict Clippy, page and service-worker WASM strict Clippy, locked page and worker-v1/v2 WASM builds, and the actual Chromium matrix. The updated CoreReply scene document is boxed on that base; P3 callers use serde serialization and field autoderef, with no caller API change required.

Chromium evidence is in `browser-results.json` and `browser-storage-results.json`. The imported REVIUNG41 document starts at revision 3. The forced IndexedDB transaction fires `abort` after one successful request and leaves revision 3 committed. Retrying the same retained snapshot reaches `complete`, reports five successful project/asset writes and revision 4, while the CoreEngine commit count remains one. Undo restores the original part coordinate at revision 5. Archive repacking preserves the project and all four asset hashes. Navigation away and reopen returns the saved revision 4 and all four assets.

The actual service worker synchronously registers Rust handlers during first module evaluation. A pre-existing unrelated CacheStorage cache contains stale bytes for the same `version.txt` URL; the worker still returns its own cache's `p3-cache-v1` value. Hex-encoded scope paths distinguish `/` and `/boardstudio/`; the pure test also distinguishes `/a/b/` from `/a_b/` and `/` from `/_/`. Root and `/boardstudio/` caches coexist; updating only root removes only root v1, keeps the subpath v1 cache and unrelated cache, serves root v2, and both roots reopen offline. Missing lazy chunks fail with a network `TypeError`. A separate fresh browser context reports no registrations or caches and correctly fails offline navigation with Chromium `ERR_INTERNET_DISCONNECTED`.

Offline cached navigation was tested after ordinary page navigation while the service worker controlled the page. The harness does not force-stop Chromium's service-worker process before going offline. The no-worker/no-cache offline failure is independently tested in a fresh context. The sibling coordinator harness at `.scratch/prototypes/p3-reference-check/` also reports a successful JS storage adapter/archive roundtrip on the copied v1 representation, including revision 4, 85 parts, four unchanged asset hashes, and invalid-import preservation. Neither prototype claims full React workflow parity or completion of the broader host/editor migration.

## Versions

- Rust `1.98.0`; Cargo `1.98.0`; wasm-pack `0.15.0`.
- Chromium `153.0.8010.52`; Playwright `1.63.0`; Node `v26.10.0`.
- Locked Rust/WASM bindings: wasm-bindgen `0.2.129`, js-sys/web-sys `0.3.106`, wasm-bindgen-futures `0.4.79`, serde-wasm-bindgen `0.6.5`.

## Standards and API references

- [W3C Service Workers](https://w3c.github.io/ServiceWorker/) — service-worker event dispatch, event lifetime, and registration behavior.
- [W3C Indexed Database API](https://w3c.github.io/IndexedDB/) and [MDN transaction complete event](https://developer.mozilla.org/en-US/docs/Web/API/IDBTransaction/complete_event) — transaction terminal success semantics.
- [MDN CacheStorage.match](https://developer.mozilla.org/en-US/docs/Web/API/CacheStorage/match) — global search across caches; implementation instead opens its scope/version cache and calls `Cache.match`.
- Installed `web-sys 0.3.106` and generated wasm-bindgen glue were checked for `ServiceWorkerGlobalScope`, cache, event and `initSync` signatures.

## Content hashes

The copied archive fixture is SHA-256 `672d5f581e65bf74b9a5df336f734167bba7642865fa7a0dfa12a96347d64a8f`. On the final merged build, the policy-only v1 worker WASM is 60,828 bytes and its generated initializer is 81,334 bytes. Exact fixture, page/worker WASM, initializer, result, and source hashes are in `artifact-hashes.sha256` and `source-hashes.sha256`.
