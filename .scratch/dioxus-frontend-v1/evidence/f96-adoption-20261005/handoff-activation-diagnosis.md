# F9.6 activation/navigation dependency correction

The preserved-profile `rehearsal/run-worker-handoff-20261005` preflight still had React's root `/sw.js` controller, its saved project and database/cache. After reload the proxy saw only bridge `/sw.js` 200 at 17:08:29.810 and 304 at 17:08:59.320. Both page wait and subsequent page evaluation timed out in `Runtime.evaluate`; there was no next document or module-worker network request. This is an actual stopped navigation, not proof that a new Dioxus page loaded. No live worker `activating` state was observed by this author, and a stale/hung CDP target remains a possible contributor to those tooling timeouts.

## Cause and discrimination

Initial ranked alternatives were (1) an activation/navigation dependency, (2) a stale/destroyed/hung page execution target, (3) bridge installation error. Read-only ServiceWorker version state/errors and target identity distinguish these without reload or storage changes. The coordinator did not hold the demonstrated correction on unavailable metadata.

The [activation algorithm](https://www.w3.org/TR/service-workers/#activation-algorithm) settles lifetime promises before entering activated state. Although the spec's fetch algorithm can skip absent handlers, [Chromium's navigation loader](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/content/browser/service_worker/service_worker_controllee_request_handler.cc) waits for ACTIVATED in `ContinueWithRegistration` before reaching `ContinueWithActivatedVersion` and its no-handler network fallback. The primary source checked here has blob `40e1d14bf8d37532f85700a93aad85f067ec01ea`. Our bridge made activation await navigation completion, creating the inverse dependency. [GoogleChrome's navigate sample](https://raw.githubusercontent.com/GoogleChrome/samples/gh-pages/service-worker/windowclient-navigate/service-worker.js) initiates client navigation but returns the array rather than awaiting its promises.

## Meaningful regression and repair

The original unit mock completed `navigate` immediately, so it did not model the browser dependency. The regression now runs the actual generated forward/reverse handoff with navigation promises held pending until activation settles. It demands activation completion before releasing that barrier, then cleans up every promise. Against the unchanged bridge all three existing cases failed at the exact assertion `activation must settle while navigation is pending` (root, subpath, reverse overlay); `handoff-activation-red.log` retains raw output.

The only production change replaces awaited `Promise.allSettled` with non-awaited initiation. Claim and controlled-client enumeration still finish inside the activation lifetime; exact origin/scope filtering stays unchanged. Navigation is started once, without timer, message, retry, or reload loop. `Promise.allSettled` still handles rejected navigation promises. The reverse test also rejects a navigation after enumeration to exercise a closed/replaced client without unhandled rejection.

`node --test scripts/web/test-service-worker-handoff.mjs` passed **3/3**, raw `handoff-activation-green.log`; `git diff --check` passed. The builder and reuse tests are unchanged, so their prior **41/41** result remains sufficient. Two frozen source hashes are in `handoff-activation-source-manifest.json`; exact delta is `handoff-activation.patch`.

No build, live-browser mutation, public cutover retry, storage deletion, or registration reset was performed by this author. The owning packaging regression establishes and fixes the concrete cycle; the final packaged preserved-profile forward → rollback → forward journey, final worker control and durable store readback remain required. This note does not invent an observed live ACTIVATING status.

## Preserved-profile metadata corroboration

The Case author then reported Chrome/154.0.8037.92, still-attached page target `92A555F4EDC6F7662D255B8B8894656E` titled Board Studio v2 at the proxy origin, and attached service-worker target `C6BE9DE551115865FB4D4785E5824D6D` at `/sw.js`. Browser-level `Target.getTargets` succeeded. `ServiceWorker.enable` was unavailable on that endpoint (-32601), so no lifecycle state was obtained. The page target had not disappeared; this narrows the tooling alternative but does not by itself prove the worker state. No further CDP mutation was requested.
