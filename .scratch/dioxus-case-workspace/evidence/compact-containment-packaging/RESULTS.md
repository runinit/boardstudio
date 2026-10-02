# Compact Case containment: served package and controls

This bounded check uses build `frontend-case-compact-containment-20261002`, source commit `b6d2af49e519b282abaf8e026dcaca1232a0fd19`, with 968 verified source hashes. The served routes were `http://127.0.0.1:34679/` and `http://127.0.0.1:34679/boardstudio/`.

## Package parity

`served-assets.json` records HTTP status, content type, served SHA-256, build-provenance SHA-256, and local byte equality for all 51 route assets plus `index.html`, `service-worker.js`, and `boardstudio_offline_worker.js` on each route. All 54 checks per route passed. The served `assets/m1.css` is 46,289 bytes on both routes with SHA-256 `ed3fcce8677e261167f247acf5304205f309cca62b593444d934bd089a0da009`, matching provenance and the local build output.

## Natural offline behavior

Two fresh browser profiles launched with distinct `/var/tmp` temporary directories. Both workers registered through normal app startup and became active and controlling at their matching route scopes. Each route had 51 cached entries. After browser-offline reload, the editor rendered and worker-mediated fetches for the route JavaScript and page WASM succeeded.

## Desktop UI containment

At 1280×577 on both routes, the document and body dimensions remained 1280×577 with no horizontal or vertical overflow. The Case workspace showed the Case assembly region with Generate case and disabled Cancel generation controls. Switching to Keymap showed the no-selection state while keeping the Macros section and Add macro action available, confirming the Case-only CSS did not hide those controls.

The built-in REVIUNG41 sample was opened in each fresh profile through the UI; the app showed its normal “Saved locally” status. No Case generation, macro edits, direct browser-storage API operations, manual worker/cache operations, or fake providers were used. The mobile Case layout matrix is owned by the separate UI verifier and is not covered here.

Evidence files:

- `served-assets.json`: all route asset hashes, status, and byte equality.
- `browser-checks.json`: worker, offline reload, and UI checks.
- `root-case-desktop.png`, `subpath-case-desktop.png`: desktop Case workspace captures.
