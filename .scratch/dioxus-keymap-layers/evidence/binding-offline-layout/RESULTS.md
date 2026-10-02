# Binding candidate offline and shell-layout verification

## Result

The fresh candidate at `http://127.0.0.1:34673/` and `/boardstudio/` naturally registered and activated its route-scoped service worker in separate fresh task-owned Chrome profiles. After opening the public built-in REVIUNG41 copy, browser network emulation was switched offline and each route was reloaded. Both reloads remained on the actual editor with the demo present; the offline navigation timing reports a service-worker response (`workerStart` about 1.1 ms and `transferSize: 0`). The route-scoped caches each held 51 resources. No internal registration, cache writes, injected storage, provider, or feature flag was used.

At both routes the editor shell mounted at desktop 1280×577 and mobile 390×844. At each size, document and body scroll dimensions exactly matched the viewport (no horizontal or document-level vertical scroll). The mobile shell used the compact workspace selector and panel controls and displayed the rendered REVIUNG41 layout. The same screenshots were obtained before and after offline reload at each compared size.

## Provenance and method

- Candidate server: existing task server on port 34673; no server or source changes were made.
- Fresh isolated profiles: `/var/tmp/frontend-run/binding-offline-layout/root-profile` and `/var/tmp/frontend-run/binding-offline-layout/subpath-profile`, created 2026-10-02 06:51 local time. Browser sessions: `binding-root-offline-7dd31abc1fc4` and `binding-subpath-offline-7dd31abc1fc4`; `agent-browser 0.38.1`, Chrome.
- **Page build identity:** `web/target/builds/frontend-keymap-bindings-20261002/provenance.json` identifies the served candidate build as `frontend-keymap-bindings-20261002`, source commit `6509f557c2a16df0a1f5ce19296eeb635df68256`; its 961 source-file hashes are all marked verified. The manifest SHA-256 is `e7babf238b38de15041a37a7f543c2fbf9834ad40ade1056e3d4ac57b036fed4`. A copy is retained here as `build-provenance.json`.
- The separate served `assets/fixtures/provenance.json` is fixture-generation/static baseline provenance from the reused provider/fixture bundle: source commit `f7449fe37432bec9bbf472574f82d8d6ba009ad7`, Rust/WASM hash `bc3a091122da2ebcb4eb55d3f0967d2b1943611105068f287c2fc6f0e17aa186`. It does **not** identify the candidate page build. Its local copy is `fixtures-provenance.json` (SHA-256 `d422ef9e10c6565b2cbe219c25ce404f6f17d5ec4ab1b55129d843b1b0f17184`); the build manifest lists this fixture-provenance file hash as part of its static asset inventory.
- `served-page-assets.json` records byte-for-byte comparisons for both routes between the build's root/subpath asset hashes, files in `site-root`/`site-subpath/boardstudio`, and HTTP responses from port 34673. Index HTML, service worker, offline-worker loader, route-specific page JS, and shared page WASM all match the build manifest for both routes. The check output SHA-256 is `5e02163cb0eb1cd98bbf2f589ab47d8944911d692fcd8b408d95509905677dc3`.
- Copied offline manifests are retained as `offline-manifest-root.json` (SHA-256 `66913f3a52ffbad1498f71343fe377460d7d530649d4cdc35972482f1cbe5f7f`) and `offline-manifest-subpath.json` (`d67695a2725d29cae814a2c5fbc839fc547005a608547a57b0b495a66510713f`).
- Root and subpath HTML each returned HTTP 200 (`text/html`; 531 and 551 bytes). Captured bodies: `root-index.html` SHA `5f66085534dc7e39aae9561fc696da0d70d28ffb66d4fd2afc2849c3f9308828`; `subpath-index.html` SHA `cb6ca85da5abd04a7f29083535deef1ca8fb6bb64b93c5f7379dcfd57ed051c6`. Root loads `/assets/boardstudio-web-dxhaf4d314b29c093d0.js`; subpath loads `/boardstudio/assets/boardstudio-web-dxh64dd33736c8654ae.js`.
- Captured route service-worker script bodies: `root-service-worker.js` SHA `4e45330bfd3d15bbed1b01b6d1b97fdfabbd56a93fa321bfeffe272b37e96141`; `subpath-service-worker.js` SHA `d47c619a5850530c2ecb36ae753f0f67045c389fdb1605c01b4f98b0edd3204d`.
- Service-worker/cache inspection was read-only (`navigator.serviceWorker.getRegistration()`, `navigator.serviceWorker.controller`, CacheStorage keys/entry count). Offline was enabled through `agent-browser set offline on` for each task-owned browser session, followed by an ordinary page reload. `navigator.onLine` was false during both successful reload checks.

## Route results

| Route | Natural active worker / scope | Cache | Offline reload | Viewports |
|---|---|---|---|---|
| `/` | `http://127.0.0.1:34673/service-worker.js`; scope `/` | `boardstudio-m1-offline-path-2f-frontend-keymap-bindings-20261002-root`, 51 entries | editor and REVIUNG41 remained visible; offline navigation `workerStart≈1.1`, `transferSize=0` | 1280×577, 390×844 |
| `/boardstudio/` | `http://127.0.0.1:34673/boardstudio/service-worker.js`; scope `/boardstudio/` | `boardstudio-m1-offline-path-2f626f61726473747564696f2f-frontend-keymap-bindings-20261002-subpath`, 51 entries | editor and REVIUNG41 remained visible; offline navigation `workerStart≈1.1`, `transferSize=0` | 1280×577, 390×844 |

On both online first loads, `isSecureContext` and service-worker support were true, and the page was controlled after registration. Browser `console` and page `errors` checks returned no messages during the captured checks. The desktop editor workspace at 1280×577 occupied x=235..960 and y=44..515; the shell used the available 577 px height without document overflow. On mobile, the workspace filled 390 px width, the compact panel controls occupied y=44..87, and the document remained 390×844 with the editor canvas visible.

## Screenshots and captured artifacts

- `root-landing-1280x577-online.png`, `root-editor-1280x577-online.png`, `root-editor-390x844-online.png`, `root-editor-offline-1280x577.png`
- `subpath-landing-1280x577-online.png`, `subpath-editor-1280x577-online.png`, `subpath-editor-390x844-online.png`, `subpath-editor-offline-390x844.png`
- HTML, service-worker script, and fixture provenance captures are in this folder. The root and subpath rendered screenshots match byte-for-byte at the same viewport; offline screenshots match their corresponding online screenshots.

This verifies packaging, natural SW installation/control, cache availability, offline document navigation, and responsive shell mounting for this served candidate. It does not verify binding interactions (covered by the separate public interaction QA), assistive-technology behavior, performance, or full application workflows.
