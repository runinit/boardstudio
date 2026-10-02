# F6K.4b browser evidence — 2026-10-02

## Baseline and fidelity

The same Sofle v2 demo and Left PCB were opened through the public UI in the TypeScript reference (`http://127.0.0.1:5175/`) and release-built Dioxus page (`http://127.0.0.1:34817/`). The TypeScript app uses the production `FirmwareKeymapPanel` and CSS. The Rust page uses the new private Dioxus component and matching local CSS. The key row order, part-reference labels, choice labels, `1/30 assigned` count, edit placement in PCB > Inspect > Electrical wiring, compact two-column grid, and preview match in the paired screenshots.

- [TypeScript PCB inspector](typescript-pcb-firmware-keymap.png)
- [Dioxus PCB inspector](dioxus-pcb-firmware-keymap.png)

## Paired user journey

On both pages, set `left-keys-SW1` to `A`, verify the assigned count/preview, Undo to `0/30`, Redo to `1/30`, reload, reopen PCB > Firmware keymap, and confirm the persisted control shows `A` (`&kp A`). The Dioxus select control is explicitly validated after reload; an initial mounted check found that its rendered option defaulted to Unassigned while the accepted preview showed `&kp A`. Rendering the accepted option as selected fixed that mismatch.

Dioxus also received a second-level mounted async check using an ephemeral browser-only IndexedDB `oncomplete` delay: while a real project write was pending, the control reported `Saving firmware position…`; after the delayed completion it reported saved and projected the accepted binding. No application code or storage records were patched by this timing harness. A separate ephemeral browser-only IndexedDB transaction-abort injection reached the mounted failure feedback and `Save failed` state. Reload restored the last durable snapshot, and a fresh edit then saved successfully. The browser injection was removed by page reload.

## Build and checks

- `cargo fmt --manifest-path web/Cargo.toml --check` — passed.
- `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page` — passed (12 library, 37 page tests).
- `cargo check --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown` — passed.
- `cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown -- -D warnings` — passed.
- `dx build --web --release --base-path / --no-default-features --features page --cargo-args=--locked` — passed. Existing unrelated unused-variable warnings remain in the Keymap encoder editor and matrix inspector.
- `git diff --check` — passed before integration merge.

## Scope boundary

This browser evidence covers F6K.4b's mounted control and exact edit/undo/redo/persist/reopen behavior. It does not close F5.2, F8.2, or the F6K.4 parent joins. Browser error injection is regression evidence only, not a production feature. No new architectural takeaway beyond the already documented owner boundaries was established in this slice.

## Final integration tip

After the paired journey, the implementation branch merged integration tip `bd671ae8db8897388d26ebf973380c69efb9bffd`; combined HEAD is `039cc612962821e0d8b64c26b05256d7beca5ffd`. Checks were rerun on that merged tree: native suites passed (12 library + 37 page + 6 Keycaps lifecycle integration tests), WASM `cargo check` passed, strict WASM all-target Clippy passed, and WASM test targets compiled with `cargo test --no-run` (3 executables). The merged release build was served and the final reload still showed `left-keys-SW1 = A`, `1/30 assigned`, revision 12 Saved.
