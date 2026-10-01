# P3 durability feasibility evidence

The full matrix passed on the initial cache naming, then reviewer feedback led to byte-hex scope keys and an injectivity regression test. This source hardening is ready for a candidate commit, with final merged-base validation pending; logs from the earlier full matrix are retained in `evidence/logs/20261001T201624Z/`, and the final merged source must be rerun with `./scripts/run.sh` before acceptance. The earlier full matrix and fresh-context browser rerun both passed. The coordinator's updated base will be merged after the source commit; exact-source results will be added to this summary and hashes after that rerun.

Chromium evidence is in `browser-results.json` and `browser-storage-results.json`. The imported REVIUNG41 document starts at revision 3. The forced IndexedDB transaction fires `abort` after one successful request and leaves revision 3 committed. Retrying the same retained snapshot reaches `complete`, reports five successful project/asset writes and revision 4, while the CoreEngine commit count remains one. Undo restores the original part coordinate at revision 5. Archive repacking preserves the project and all four asset hashes. Navigation away and reopen returns the saved revision 4 and all four assets.

The actual service worker synchronously registers Rust handlers during first module evaluation. A pre-existing unrelated CacheStorage cache contains stale bytes for the same `version.txt` URL; the worker still returns its own cache's `p3-cache-v1` value. Root and `/boardstudio/` caches coexist; updating only root removes only root v1, keeps the subpath v1 cache and unrelated cache, serves root v2, and both roots reopen offline. Missing lazy chunks fail with a network `TypeError`. A separate fresh browser context reports no registrations or caches and correctly fails offline navigation with Chromium `ERR_INTERNET_DISCONNECTED`.

Offline cached navigation was tested after ordinary page navigation while the service worker controlled the page. The harness does not force-stop Chromium's service-worker process before going offline. The no-worker/no-cache offline failure is independently tested in a fresh context. This probe does not claim production integration, full React parity, or completion of the broader host/editor migration; the coordinator has a separate app-storage adapter exchange harness for the copied v1 representation.

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

The copied archive fixture is SHA-256 `672d5f581e65bf74b9a5df336f734167bba7642865fa7a0dfa12a96347d64a8f`. The last pre-hardening policy-only v1 worker WASM was 53,981 bytes and its initializer was 72,206 bytes. Exact fixture, WASM, initializer, result, and source hashes are in `artifact-hashes.sha256` and `source-hashes.sha256`; final merged-build hashes will replace those measurements.
