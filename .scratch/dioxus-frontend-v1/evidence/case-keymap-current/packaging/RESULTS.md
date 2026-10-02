# Case-keymap current build: offline root/subpath verification

Browser tooling was agent-browser 0.38.1 with HeadlessChrome 154.0.0.0. This packet records a browser-level packaging check against build `frontend-case-keymap-20261002` (`dc81237c51c9d4f43ddb7f0d3f6f7eda9cf685fc`) served at `http://127.0.0.1:34663/`. The source build `provenance.json` declares `status: complete`, 51 assets at both prefixes, and is preserved verbatim in `artifacts/provenance.json`.

The unchanged `natural-install-probe.mjs` helper from the retained `parts-context-e510dd4c/packaging/offline/environment-controls/` folder was run with new, independent Chromium profiles under `/var/tmp/frontend-case-keymap-offline/profiles/` (not `/tmp`). Each run enabled CDP ServiceWorker lifecycle events, waited for this build's worker to reach `activated` or `redundant`, then enabled browser offline emulation and reloaded only after an activation verdict. No page script or service-worker registration was injected. Browser result JSON, including the lifecycle events and post-offline accessible snapshot, is preserved under `browser-results/`.

| Prefix | Worker lifecycle | Registration/controller | Cache | Offline reload | Result |
|---|---|---|---|---|---|
| `/` | `activated` | scope `/`, active `activated`; controller true | one app cache, 51 entries | URL remained `/`; snapshot contained the `Open a keyboard` landing and keyboard list | pass |
| `/boardstudio/` | `activated` | scope `/boardstudio/`, active `activated`; controller true | one app cache, 51 entries | URL remained `/boardstudio/`; snapshot contained the `Open a keyboard` landing and keyboard list | pass |

The root and subpath cache names are prefix/build scoped (`boardstudio-m1-offline-path-2f-frontend-case-keymap-20261002-root` and `boardstudio-m1-offline-path-2f626f61726473747564696f2f-frontend-case-keymap-20261002-subpath`). The 51 count is read from the actual browser Cache Storage after app registration; each test also verifies the activated registration controls the app page before treating offline reload as meaningful.

For artifact provenance, the captured root and subpath service-worker scripts and `boardstudio_offline_worker.js` are the built files. Their SHA-256 values match the respective entries in `provenance.json` and the bytes fetched from the live server (`artifacts/served-*`). Root service-worker hash is `3d24c25674228bed67ca438a1f1591dba588a44d359a8cb18308a34355e48a52`; subpath hash is `65599ff0fe5820568c2bf62d749ad72ef7c7f8e3a654f7c601bbca75f88ad34d`; offline worker hash is `1c02dac3443cbc9b8c312b6c0d78181f6e76c53077af235a3d92f67cc9842c61`.

This establishes fresh-profile browser install/control/cache/offline-landing behavior for root and subpath packaging in this Chromium test environment. It does not claim cross-browser coverage, longer-term persistence across browser restarts, or a network-isolated physical device test. No production source, build output, server, Cargo target, or existing browser profile was changed by this verification.
