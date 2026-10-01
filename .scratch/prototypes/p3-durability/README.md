# P3 durability and offline feasibility probe

This isolated prototype answers two questions against the current v1 storage representation: does a real IndexedDB abort preserve the last committed REVIUNG41 snapshot for a same-snapshot retry, and can Rust-owned service-worker cache policy serve the shell offline at both `/` and `/boardstudio/`?

Run the full native, WASM, and Chromium experiment with `./scripts/run.sh`. Every command and exit status is stored under `evidence/logs/<UTC-attempt>/`. Prior generated bundles are moved to `dist-attempt-*` instead of being discarded. The copied fixture is `fixtures/reviung41-original.boardstudio` (SHA-256 `672d5f581e65bf74b9a5df336f734167bba7642865fa7a0dfa12a96347d64a8f`).

The browser page imports that archive through the public Rust archive request, validates the 85-part document and four assets, then stores the document as the existing plain JSON object in `projects` (`keyPath: "id"`) and each asset as `Uint8Array` under its SHA-256 key in `assets` (out-of-line keys). The active project ID uses the existing local-storage preference key. A CoreEngine MoveParts commit produces one retained save snapshot. The probe forces the actual IndexedDB transaction to abort after one successful put request, checks the transaction `abort` event and unchanged committed revision, retries the retained snapshot without another engine edit, and checks transaction `complete` plus a single Undo restoring the original part position. The saved document and bytes are then packed and unpacked through the public archive API and reopened after navigation.

The service-worker policy is Rust/WASM. The only generated JavaScript is a synchronous initializer: it decodes a build-produced base64 representation of the Rust service-worker WASM and calls `initSync`. The Rust `wasm_bindgen(start)` function attaches install, activate, and fetch listeners during the first synchronous module evaluation. Rust owns the asset manifest, cache names, Cache API reads and writes, update cleanup, request routing, network fallback, and failure behavior. Registration paths are hex-encoded byte-for-byte into their cache prefixes so distinct paths cannot collide; fetch matching consults that worker's exact named Cache rather than a global `CacheStorage.match` search.

The P3 WASM build excludes the editor/archive module and `boardstudio_core`; native and page features keep the public archive code. On the final merged build, the release service-worker WASM is 60,828 bytes and its generated initializer is 81,334 bytes. No new JavaScript runtime dependency or paid service is introduced. The test runner uses the app's existing Playwright development dependency and installed Chromium.

## Evidence

- `evidence/browser-storage-results.json` records the actual IndexedDB abort/retry, asset roundtrip, and navigation reopen.
- `evidence/browser-results.json` records the actual Chromium service-worker checks: first root and subpath registration, separate scopes, stale same-URL data in a pre-existing unrelated cache, v1-to-v2 root update that preserves the subpath cache, offline shell navigation, and an uncached lazy asset failing with a network error.
- The fresh-context case starts with no registrations and no CacheStorage entries, then proves that an offline root navigation fails with `ERR_INTERNET_DISCONNECTED`.
- The coordinator's separate `p3-reference-check` executes the copied IndexedDB representation through `app/src/storage.ts` and the public archive exchange; this probe alone does not claim complete React workflow parity.

## Runtime findings and limits

The initial module-worker loader used top-level `await` before Rust registered its listeners. Chromium 153.0.8010.52 rejected even a minimal module worker with top-level `await` as “ServiceWorker cannot be started.” A separate synchronous initializer was tested with the original 5,235,225-byte worker WASM; its 6,980,519-byte generated script failed startup with “Failed to access storage.” A comment-only script of the same size failed similarly, while a small worker registered. This is recorded as an observed packaging failure for this Chromium build, not a general browser size limit. Gating editor/archive code reduced the worker WASM to 53,981 bytes, allowing a synchronous initializer and Rust listener registration without an asynchronous JavaScript policy bridge.

Offline shell reopen is verified in Chromium after ordinary navigation with the worker controlling the page; the harness does not forcibly terminate Chromium's service-worker process between the online install and offline navigation. The fresh-context offline failure is separately verified. Missing lazy assets fail honestly when offline; they are not silently claimed as cached. This prototype is feasibility evidence, not a production integration or a completion claim for the wider host/editor migration.

## Standards and pinned tools consulted

- [W3C Service Workers](https://w3c.github.io/ServiceWorker/) for first-evaluation event-listener registration and install/activate/fetch lifetime behavior.
- [W3C Indexed Database API](https://w3c.github.io/IndexedDB/) and [MDN `IDBTransaction` complete event](https://developer.mozilla.org/en-US/docs/Web/API/IDBTransaction/complete_event) for transaction-level success semantics.
- [MDN `CacheStorage.match`](https://developer.mozilla.org/en-US/docs/Web/API/CacheStorage/match) for the cross-cache search behavior that motivated using a cache-specific `Cache.match`.
- The package pins `wasm-bindgen 0.2.129`, `js-sys 0.3.106`, `web-sys 0.3.106`, and `wasm-bindgen-futures 0.4.79`; `Cargo.lock` is committed. The generated loader uses the installed `wasm-bindgen` API matching that build.
