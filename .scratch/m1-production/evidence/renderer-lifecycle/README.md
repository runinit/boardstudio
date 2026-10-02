# Delayed renderer import and scope replacement

The retained JSON records a correctness-only browser probe against immutable release `8f509433`. It delayed the real renderer JavaScript response for 12 seconds, clicked the public **REVIUNG41 copy** action while the earlier Sofle case mount was still awaiting the import, then checked the page after that old response completed.

The ordering markers are the owned server's module request/release timestamps plus the browser snapshot taken immediately after the REVIUNG41 copy action: request arrived at Unix `1790899439.259926`, response released at `1790899451.2600067`; before release, the visible page already showed the REVIUNG41 document and Keyboard PCB. The final DOM had no generated-case canvas. Instrumentation saw no old-canvas `getContext`, canvas listeners, resize observer, or animation frame. The module request returned HTTP 200 and the browser reported no errors.

The harness returns 404 only for the service worker so the actual lazy dynamic import cannot be satisfied by a prefetched cache entry. It does not modify the staged assets or application code. This probe covers root-prefix stale-mount cancellation only. It does not test offline behavior, subpath deployment, tab closure, or broad app resource limits. The browser session and temporary HTTP server were closed after capture.

Run details, release asset hashes, exact observations, and limitations are in `delayed-import-scope-replacement.json`. `instrument.js` is the browser-side observer and `delayed-renderer-server.py` is the static server with the one delayed asset route.
