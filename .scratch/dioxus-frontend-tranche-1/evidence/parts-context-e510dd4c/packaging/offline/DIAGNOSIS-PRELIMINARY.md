# Offline registration diagnosis

## Compared artifacts and browser

The served candidate at port 34659 is the build directory `web/target/builds/frontend-parts-context-aria-20261002`, provenance commit `f65b0c833694d1980ac71caa21d2f932cf785d53`. The known baseline at port 34651 is `web/target/builds/frontend-panels-47cf655b-20261002`, provenance commit `47cf655baabde025628f15cae94e5b34e6dabaf4`. Both were served by the same `serve-release.mjs` from the integration worktree and tested with `agent-browser 0.38.1` / headless Chromium 154 on localhost. Candidate and baseline provenance JSON plus root offline manifests are in the worktree build directories; their SHA-256 values are in the session report sent to the parent.

## Source-backed expected flow

`web/src/presentation.rs` calls `Runtime::new()` from the public `App`. `web/src/runtime.rs:97-105` creates the runtime, then asynchronously calls `boardstudio_web::host::register_offline(prefix)` automatically and reports a user-visible status if registration returns an error. In `web/src/host/offline.rs:7-18`, the registration uses `{scope}service-worker.js`, sets the registration scope and module type, and awaits `ServiceWorkerContainer.register`. The packaged worker's `web/src/service_worker.rs` install handler opens its versioned cache and runs `Cache.add_all` over the generated manifest, then claims clients on activation. Thus the raw offline reload test did not need a user activation button; automatic startup registration is the intended path.

The f65 provenance lists only presentation/CAD/CSS/main inputs as changed from its base. Its built page WASM contains the literal “Offline setup failed; this keyboard still needs an online connection”, and the packaged `service-worker.js` is present. This confirms the compiled binary includes the failure-report string, but does not by itself prove that the registration future ran.

## Observed A/B behavior

- **Candidate f65 / port 34659:** In a fresh browser session, `isSecureContext` was true and `navigator.serviceWorker` existed. After the app settled, `getRegistrations()` returned an empty list and `navigator.serviceWorker.controller` was false. A direct read-only `fetch("service-worker.js")` returned 200 `text/javascript`. The public app loaded normally; browser `errors` and `console` captures were empty. The root and subpath sessions had no active worker before the earlier offline reloads; both reloads ended at `chrome-error://chromewebdata/` with `ERR_INTERNET_DISCONNECTED`.
- **Baseline / port 34651:** Under the same browser automation and localhost secure context, a fresh session had an activated registration for `http://127.0.0.1:34651/` using `/service-worker.js`, and its versioned CacheStorage contained 51 entries. With browser offline enabled, reload remained at `/` and rendered the app’s “Open a keyboard” landing screen. The registration, cache contents, offline page and browser diagnostics are captured in `baseline-*.json` / `baseline-offline-*.txt`.
- Candidate and baseline manifests each list 51 assets. A read-only check found all 51 files present in each `site-root` output and all 51 paths returned successful HTTP HEAD responses from their respective static servers. This reduces the likelihood of a simple missing-file or route-availability cause; HEAD checks do not establish that every service-worker fetch/install succeeded.

The initial candidate `*-before.json` probes opened the named version cache with `caches.open()` to inspect its keys, which can create an empty cache. Therefore an empty candidate cache is not attributed to the application. The stronger observation is that the app had no service-worker registration/controller; baseline had an activated worker and a populated cache.

## Ranked hypotheses and limits

1. **Most consistent with the A/B evidence:** candidate startup does not complete automatic service-worker registration/installation, while the baseline and browser harness do. This is a candidate regression in offline availability relative to the known baseline. The exact stage (whether the runtime registration call is skipped/rejected or worker installation fails) is unresolved.
2. **Possible but not demonstrated:** the candidate worker's module initialization or `install` / `Cache.addAll` path fails despite the served script and all manifest asset paths being available. `agent-browser` page-target network/console diagnostics did not expose a worker exception, and the service-worker target's internal logs were not directly captured.
3. **Less likely:** browser automation, secure-context, or generic server availability prevents registration. The same named-browser tooling registered and activated the baseline worker on localhost; candidate reports secure context/API availability and a 200 response for its worker script. This does not rule out a candidate-specific server response difference beyond those checks.

There is no public “make available offline” control in the inspected UI, but that absence is not the explanation: source calls registration automatically. The candidate reload failures should be treated as a real observed regression against this baseline, while the cause remains undiagnosed. No registration was injected or manually invoked, no source/config was changed, and no compiler was run.
