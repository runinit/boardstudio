# Outline bridge camera repair evidence

The selected bridge could move the Dioxus Layout camera off the rendered board. The renderer builds its base view box around accepted part-position bounds and interprets `camera.center` as a pan delta from that center. `fit_selected_bridge` supplied the bridge's absolute world-space center as that delta, so the renderer applied the board origin twice. The private helper now subtracts the visible-part bounds center from the selected bridge center before submitting the existing `SetCamera` event. It leaves the existing zoom calculation and state authority intact.

The regression test uses the same private helper that production calls. It expects a bridge at `(270, -30)` in viewport bounds `(0, 280, -80, 0)` to produce offset `(130, 10)`. With the former absolute-center behavior, the test fails on the x offset; with the repair it passes. Logs: `camera-red.log` and `camera-green.log`.

## Browser reproduction

- React oracle: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369` (live verified React page).
- Dioxus candidate: `http://127.0.0.1:34728/boardstudio/`, staged build `layout-bridge-camera-20261002`, from base `c701233ea7bab6a4c794cba38cabf13be3a0b498` plus this isolated camera diff.
- Both origins imported the same archive bytes, `docs/design/evidence/board-outlines/reviung41-original.boardstudio` (SHA-256 `672d5f581e65bf74b9a5df336f734167bba7642865fa7a0dfa12a96347d64a8f`). Browser-local project stores are separate.
- Before repair, selecting the same Bridge 1 occurrence from the Outline and owning `right keys` matrix routes showed an empty Layout canvas (`screenshots/dioxus-before-fix-*.png`).
- With the repair, selecting Bridge 1 from Outline shows its actual geometry around `main-RST` (`dioxus-outline-bridge-selected.png`). Selecting `main-U1` then Bridge 1 from the owning matrix occurrence returns to the visible bridge with the same typed context ID, `main-outline:bridge:cc38e6fa56cda910` (`dioxus-matrix-bridge-selected.png`). The matrix-row click was initiated from Layout.
- React on the same fixture frames the connection around `main-RST` at 461% (`react-outline-bridge-selected.png`). Dioxus displays the selected bridge at 755%; both are visible, but Dioxus crops closer. This repair is limited to the coordinate-frame error that caused the blank canvas; scale parity remains separate.
- Browser page errors: none. The React console showed only Vite connection and React DevTools informational messages.

## Verification

- Expected-red then green targeted test: `cargo test --manifest-path web/Cargo.toml --no-default-features --features page --test outline_camera` (`camera-red.log`, `camera-green.log`).
- Full native web suite passed: `cargo test --manifest-path web/Cargo.toml --no-default-features --features page` (`native-suite.log`).
- Strict supported WASM all-target Clippy passed: `cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings` (`wasm-strict-clippy.log`).
- `cargo fmt --manifest-path web/Cargo.toml -- --check` and `git diff --check` passed.
- Native all-target Clippy was attempted and remains blocked by existing page-only dead-code warnings in native `boardstudio-web` targets; the relevant supported WASM strict gate passes. No lint suppressions or unrelated configuration changes were added.
- Staged full build provenance is in `web/target/builds/layout-bridge-camera-20261002/provenance.json`. The build completed successfully and includes the Dioxus production bundle used for the browser check.

This is source and staged-browser evidence for the bounded camera fix. The integrated `34727` candidate still requires a fresh post-join regression check before any parent or RF ledger closure.
