# Final physical-instance STEP delivery check

This changed-path browser check uses maintained release `m1-release-20261002-a49bb798` at source `a49bb798cf751d0f111c265c29d7276841e513b9`. The release manifest and asset hashes are recorded in `../release-a49bb798-provenance.json`. A dedicated agent-browser session and an owned test server served the immutable `site-root` output. The owned origin used `--no-sw` so that the second export could be held at the real CAD worker WebAssembly response; this was not an offline test. The pristine demo origin remained untouched.

From the public editor, the run opened a Sofle v2 copy, selected Left PCB and the `left half` physical instance, used **Add case settings**, generated the case, and exported STEP. The browser downloaded `downloads/sofle-left-half-instance.step` (6,417,795 bytes; SHA-256 `9b697043af32fc028cd7e52cfb93b9c778747318cf411f8555d9652646a4adec`). It begins with `ISO-10303-21;` and the header. The public CAD worker returned an `export-step` result with `revision` and `step`, also 6,417,795 bytes. Browser instrumentation observed one Blob URL creation, one revocation, and one download-anchor click.

For cancellation, the QA server held the next request for the actual 6,097,242-byte CAD worker WASM. Export was started while Left PCB / `left half` was selected. While the worker response was pending, the public Physical instance selector changed to Canonical board. The renderer observer saw its ResizeObserver target disconnect and the CAD worker targets terminate. After releasing the unchanged staged WASM bytes, the page remained Canonical, the Blob URL totals remained 1 created / 1 revoked with none active, and the anchor download total remained one. The page error log was empty. This confirms that the pending physical-scope export did not produce a stale download after the public scope change.

Raw browser/server snapshots and the downloaded STEP are retained alongside `physical-left-step-acceptance.json`. The resource observer is `../../cad-jobs/renderer-resource-probe.init.js`; the anchor observer is `download-click-probe.init.js` in this directory. Rerun instructions follow.

```sh
node .scratch/m1-production/evidence/integration/qa-observers/serve-startup-restore-qa.mjs \
  web/target/builds/m1-release-20261002-a49bb798 44126 --no-sw
```

In another terminal, use a fresh named `agent-browser` session. Open `http://127.0.0.1:44126/` with both `renderer-resource-probe.init.js` and `download-click-probe.init.js` as init scripts. Open the Sofle copy, select Left PCB / `left half`, add settings, generate, and use the public Export STEP control. For the cancellation half, `POST /__qa/arm-cad`, start Export STEP, wait until `/__qa/status` reports a pending CAD request, switch to Canonical board, then `POST /__qa/release-cad`. Capture resource marks and anchor clicks before and after release. Close the owned browser and server when finished.

The check does not claim IndexedDB denial behavior, Rust `Drop` execution, GPU memory reclamation, or a CAD material-oracle result. Offline behavior is covered by the full release QA record, not this no-service-worker origin.
