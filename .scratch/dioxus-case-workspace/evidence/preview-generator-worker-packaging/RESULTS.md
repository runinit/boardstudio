# Preview-generator worker package and compact Case smoke

## Build and asset provenance

Final build: `frontend-case-focus-resize-final-20261002`, source commit `e2a84d8b00418eab4d1ec47e0bf5b6957da9ffe3`, with 971 verified source hashes. It was served at `http://127.0.0.1:34683/` and `http://127.0.0.1:34683/boardstudio/`.

`final-served-assets.json` records every one of the 56 route assets on each route. All 112 served responses matched both the local static output and build provenance. The report includes the app CSS, route JS/WASM, service workers, offline loader, and preview-generator files.

`final-static-graph.json` validates the exact five-file graph: `worker.mjs`, `kicad.mjs`, `ergogen.mjs`, `catalogue.mjs`, and `provenance.json`. Every generated module matches its package provenance hash; static imports remain route-relative: worker→Ergogen/KiCad, KiCad→Ergogen, Ergogen→catalogue, catalogue→none.

## Worker execution lineage

A real `type: module` Worker was exercised on both routes in build `frontend-case-focus-preview-worker-20261002` (source `cf67a388acbd90e52665185dfb4dc3bd8eab31b8`) from a same-origin static directory listing rather than the app. Each route returned one generated switch-footprint job and echoed its worker request, owner, batch, and plan identities. The harness had no app shell and loaded no React or Core worker/WASM.

The five preview-generator files, including their package provenance file, are byte-identical between that worker-tested build and the final build on both routes. `build-lineage.json` records the hashes and equality, so the real Worker test applies to the final graph without rerunning it.

## Final build browser checks

Two fresh browser profiles naturally registered active workers at `/` and `/boardstudio/`, each with 56 cache entries. Offline reload rendered the editor on both routes; cached route JavaScript and page WASM fetches succeeded. Browser page-error checks returned no errors.

At desktop 1280×577, both routes had 1280×577 document dimensions. Case retained its Case assembly region and Generate case / disabled Cancel generation controls. Switching to Keymap retained the no-selection state with the Macros section and Add macro action. No Case generation or macro mutation was submitted. Mobile Case coverage remains with the separate UI verifier.

The built-in REVIUNG41 sample was opened through normal UI controls in the isolated profiles and the app displayed its normal “Saved locally” status. No direct browser-storage APIs, fake providers, or manual worker/cache operations were used.

Evidence files:

- `served-assets.json` and `static-graph.json`: the 34681 worker-tested build’s served asset and static graph checks.
- `root-generator-worker.har`, `subpath-generator-worker.har`, and `worker-execution.json`: route-relative module Worker requests and correlated output.
- `final-served-assets.json`, `final-static-graph.json`, `build-lineage.json`: exact final build parity and reuse basis.
- `final-browser-checks.json`: final route worker/offline and desktop UI checks.
- `final-*-case-desktop.png`, `final-*-keymap-desktop.png`, `final-*-offline.png`: final build screenshots for both routes.
