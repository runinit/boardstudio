# Encoder mount packaging, layout, and editor regression check

This packet covers the fresh `frontend-encoder-select-fixed-20261002` candidate served at `http://127.0.0.1:34687/` and `http://127.0.0.1:34687/boardstudio/`. Build provenance identifies source commit `868edfcbdf93315e866962c9c57543d26672f379`, 973 verified source hashes, and eight successful build steps.

## Served assets and offline routes

For each route, all 56 served assets match the build provenance and its local static-site bytes; all 56 offline-manifest paths resolve to those same bytes. The details are in `served-assets-final.json`, `build-provenance-final.json`, and the copied route manifests.

Each fresh route-specific browser profile naturally installed and activated its scoped worker, which controlled the page and cached 56 assets. After switching browser network emulation offline and reloading, the editor rendered from both routes. Navigation and WASM performance entries showed a service-worker response (`workerStart` 1 ms for navigation, 12–19 ms for WASM) and zero transferred network bytes. No page errors were reported.

At desktop 1280×577 and compact 390×844, both routes had document/body widths equal to the viewport. Compact Keymap content was vertically scrollable (height 1852 px at root and 3400 px at the subpath); this packet makes no claim that the Keymap page has no vertical scroll.

## Corrected select and editor smoke

I imported the unchanged layered Sofle fixtures through the visible `.boardstudio` import control in isolated profiles. In `imported-layered-sofle.boardstudio` (SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`), both clockwise and counterclockwise encoder controls displayed `Unassigned`, matching the source fixture’s absent sensor entries. An untouched normal key likewise displayed `Unassigned`.

In a separate profile, I imported `reference-layered-bindings.boardstudio` (SHA-256 `c9aa6a4e387fc206fd3a4a0e944b140f095ba8774a8ab5bdc93a99d74e19c9ea`), added `Macro 2`, and exercised ordinary binding selections on separate keys: SW3 selected the second layer, `Function`; SW5 selected the last hold modifier, `RGUI`; and SW6 selected the second macro, `Macro 2`. After a full page reload and selecting each key again, the correct behavior and non-first choice remained selected. SW4’s initial `Unassigned` value also remained correct. Filtered post-reload snapshots are retained as `final-binding-none-snapshot.txt`, `final-layer-nonfirst-snapshot.txt`, `final-hold-nonfirst-snapshot.txt`, and `final-macro-nonfirst-snapshot.txt`.

The prior `frontend-encoder-bindings-20261002` build at port 34685 is retained only as regression lineage: before the select fix, its true-None encoder controls rendered `Key press`. The corrected candidate renders `Unassigned` for both. `served-assets.json` and the earlier screenshots record the prior candidate’s route verification; final checks are isolated in `served-assets-final.json`, `browser-checks-final.json`, and the `final-*` screenshots.

This is a focused packaging/layout and ordinary editor regression check. It does not close F6K.4, F5.2, F8.2, F6K.2, or F6K.3 acceptance. It does not test an attached VIK module, firmware-provider output, a full accessibility audit, or assert that the Keymap page has no vertical scrolling.

## Evidence

- `served-assets-final.json`: exact HTTP/provenance/local equality for both routes.
- `browser-checks-final.json`: worker scopes/cache counts, offline navigation/WASM timing, dimensions, and UI checks.
- `final-encoder-none-snapshot.txt` and the four `final-*-snapshot.txt` files: filtered DOM snapshots after import/reload.
- `final-root-keymap-offline-1280x577.png`, `final-root-keymap-offline-390x844.png`, `final-subpath-keymap-offline-1280x577.png`, `final-subpath-keymap-offline-390x844.png`: screenshots from the corrected candidate.
- `build-provenance-final.json`, `offline-manifest-root-final.json`, and `offline-manifest-subpath-final.json`: copied candidate provenance/manifests. The built WASM remains in the ignored build output and is not duplicated here.
