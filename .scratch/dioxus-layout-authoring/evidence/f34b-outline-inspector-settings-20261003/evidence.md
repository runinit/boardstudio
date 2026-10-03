# F3.4b outline settings and version controls

This packet implements the current accepted Board outline Inspector in the isolated F3.4b worktree, based on `813859071b31b129fa53631fb29c85e4a79e4553`. The private adapter maps UI intents to existing `SelectOutline`, `RenameOutline`, `SetOutline`, and `ReplaceDocument` operations. It keeps admission and exact outcome handling in the existing outline lifecycle owner.

The UI now includes accepted version selection and rename, automatic feature creation when missing, source-backed settings and gap controls, and a projected preview. Gap repair only appears for Generated when accepted gaps exist. Cleanup/clearance controls and protected-gap removal follow React's Advanced cleanup section. Fixed versions expose corners/size and general clearances; pinned React only exposes bridge width for Generated. Numeric drafts commit on blur/Enter and Escape restores the accepted value. Blank numeric/name drafts expose `aria-invalid`.

Validation on this packet:

- `cargo test --manifest-path web/Cargo.toml --locked --no-default-features --features page --bin boardstudio-web outline_settings -- --nocapture`: **4 passed**.
- `cargo test --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --locked --no-default-features --features page --bin boardstudio-web presentation::outline_lifecycle::browser_tests -- --nocapture`, with the reused `f73b-layout-layer-controls-20261002/web/target`, `wasm-bindgen-test-runner`, and headless Chrome: **2 passed**. The tests cover the mounted production Generate action reaching the current owner and existing document edit, plus Enter commit/Escape restore on the mounted dimension control.
- `cargo fmt --manifest-path web/Cargo.toml --all -- --check` and `git diff --check`: **passed**.

The native adapter suite was rerun after the JavaScript-compatible UTF-16 version-name limit change. The browser suite passed before removing one unnecessary `mut` from the test fixture; that final warning-only edit does not change behavior. Combined strict WASM Clippy and the public paired multi-version/Undo/Redo/save-reopen journey remain integration gates. F3.4b does not close F3.4a, F3.4, T1-11, or the 62-parent graph.

RF-001/RF-006/RF-009 remain the relevant carry-forward entries. **No new refactoring takeaway observed.**

## Same-context Undo refresh follow-up

The integrated paired settings journey found a focused Inspector bug: after changing the fixed outline's Fillet radius from 2 to 3.5 and pressing Undo, the accepted document returned to 2 and advanced to a saved revision, but the mounted dimension field continued to render 3.5 until the user changed selection away and back. Red: integrated candidate source `04c85b88eae2894b416979f785a1f0060d2eb27d` at `34759` (the root-confirmed `34760` run used the current integrated source). The accepted projection was current; the private `OutlineDimension` local draft baseline was not refreshed when its accepted `value` prop changed.

The bounded source correction adds the same value-baseline reconciliation already used by `OutlineCoordinate`: when the accepted value changes (including Undo/Redo), reset the local draft to that value. It does not clear a draft on unrelated rerenders. One integrated browser GREEN on the next candidate remains required. No additional tests or build were run in this isolated source leaf. F3.4b, F3.4, F3.7 and all parent joins remain open; RF-001/RF-006/RF-009 remain applicable and no new refactoring takeaway was observed.
