# Macro editor build: served assets and offline shell

This evidence checks the completed `frontend-keymap-macros-20261002` build from source commit `9ef5bc5cad09ab3064711b45fc348b4cc09a74d4` served at `http://127.0.0.1:34677/` and `http://127.0.0.1:34677/boardstudio/`. Build provenance reports 967 verified source hashes.

## Served bytes

`served-page-assets.json` records HTTP status, content type, SHA-256, and byte equality for each route's HTML, service worker, offline loader, route JavaScript, and page WASM. The served bytes match both the build provenance and the route's local static-site file for every recorded resource. `offline-manifest-http.json` records a separate fetch and byte comparison for every offline asset: all 51 root-route assets and all 51 subpath assets succeeded and matched the local static site.

The shared page WASM hash is `b9dde4a01542ad653d18adb369787d5c3af8948bd5e4a31ccb631048f53a900f`. The binary was deliberately not copied into this evidence directory; the local build retains it.

## Natural service-worker install and offline reload

Two fresh, isolated browser profiles were launched with separate `/var/tmp` browser temp directories. The app registered its worker through its normal startup path; no manual registration or cache writes were performed. Before going offline:

- `/` had an activated, controlling worker scoped to `/` and 51 cached entries.
- `/boardstudio/` had an activated, controlling worker scoped to `/boardstudio/` and 51 cached entries.

Each page was then reloaded using the browser's offline mode. The editor shell rendered from both routes, and worker-mediated fetches for the route JavaScript and page WASM returned successfully. The post-reload worker remained active with all 51 entries.

The browser initially used a constrained default temporary filesystem and showed no worker registration. Re-running with isolated `/var/tmp` directories allowed the natural install to complete. This was an environment setup issue; no application or server bytes were changed.

## Shell and Keymap checks

At 1280×577, the document and body widths were 1280 pixels on both routes, with no horizontal overflow. At 390×844, the document/body widths were 390 pixels on both routes; content height extended to 1323 pixels and remained vertically scrollable, with no horizontal overflow.

In each fresh profile, the built-in REVIUNG41 sample was opened and the Keymap workspace inspected without selecting a key. The inspector showed “Select a key”, an empty-result key search, the “Macros” section, and an available “Add macro” action at the same time. Filling search with `NO_SUCH_KEY_999` filtered the selected-key chooser to its placeholder while leaving the Macros section/action present. No macro was created or edited, so macro CRUD and persistence are not claimed by this packaging/layout check.

Screenshots cover the editor and this unselected/search state for each route and viewport. The `*-keymap-mobile-macro-section.png` captures are scrolled to show the filtered empty key chooser beside the still-available Macros section and Add macro action. They are browser captures from the isolated profiles; no DOM or CSS was altered by the test.

The editor was entered by opening the built-in REVIUNG41 sample in each fresh profile. The application showed its normal “Saved locally” state after opening it. No macro or keymap edits were submitted, and no browser storage APIs, fake providers, manual worker/cache operations, or global cleanup were used.

## Evidence files

- `served-page-assets.json`: exact served/provenance/local asset hashes and equality.
- `offline-manifest-http.json`: full 51-asset per-route HTTP and local byte comparison.
- `browser-checks.json`: natural worker install, offline reload, and viewport results.
- `root-online-first-load.har`: initial root-route request capture.
- `*-editor-desktop.png`: editor at 1280×577.
- `*-keymap-no-selection-search.png`: Keymap inspector with no selected key and a nonmatching search.
- `*-keymap-mobile.png`: Keymap at 390×844.
- `*-keymap-mobile-macro-section.png`: scrolled mobile view showing the filtered chooser and independently available Macros action.
- `*-offline-desktop.png`: editor after ordinary offline reload at 1280×577.
- `root-landing-1280x577.png`: root-route landing shell.
