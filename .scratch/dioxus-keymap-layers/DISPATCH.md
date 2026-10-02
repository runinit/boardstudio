# Dispatch notes — F6K.1 Keymap projection/layers

## Ownership and seam

INT.1 is the parent start edge only; its reviewed extraction does not provide a Keymap mount, read model, canvas or selection callback. Coordinator must prove/own the page-crate Keymap mount, accepted document and board/scope identity, shared selected-part callback, edit callback and owner files. F6K creates private Keymap projection/panel modules and its actual mount integration must be explicitly handed off; do not widen library visibility or edit coordinator-owned `web/src/presentation.rs` concurrently. Parent F3.1 is an acceptance join and cannot be inferred from fixture work.

## Source and behavior

Parent spec is `.scratch/dioxus-frontend-v1/issues/06-keymap-keycaps.md`; graph row is F6K.1 in `tasks.json`. React responsibilities: `app/src/ui/createKeymapWorkspace.tsx`, `KeymapPanel.tsx`, `KeymapLayout.tsx`, `KeyBindingEditor.tsx`, and shared selection controller. The reference creates a virtual Base map when document keymap is absent; active layer resolves from stable ID with first-layer fallback. Legacy base `keyBinding` maps to key-press/transparent/none as supported; absent non-base entries are transparent. Key set includes switches and supported direct matrix keys and filters matrix companions. KeymapPanel's “Selected key” is a native select filtered by key reference and binding title; canvas highlights shared selected-part selection.

Candidate currently routes Keymap to a placeholder in `web/src/presentation.rs`. Accepted types are `KeymapConfiguration`/`KeymapLayer` in `core/src/keymap.rs`, edit changes in `core/src/model.rs` and apply/validation in `core/src/keymap.rs`. Existing events/edit callback must be used. RenameLayer permits Base rename; RemoveLayer protects the first layer. UI allows rename any selected layer, rename on blur, adds up to 32 layers and removes only non-base. Keep F6K.2's binding editor, macros, encoder/firmware controls and keycaps out of these tickets.

## Checks and profiles

Run `cargo fmt --manifest-path web/Cargo.toml -- --check`; `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings`; `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web`; applicable Dioxus page build with source provenance; and public Dioxus browser evidence. Existing React tests are only reference oracles, not Dioxus compilation/runtime evidence. Include themes/desktop/compact, keyboard/focus/axe, legacy and layered fixtures, scope changes, invalid edits, history and storage. No new gate. Keep F3.1 join open until verified. Reconcile parent RF-009 at publication.

F6K.1a: Luna High author/verifier due to new canvas/selection/scope/projection; Astra High independent reviewer; M. F6K.1b: Luna High unless a proven edit callback and ID-selection controller are supplied, then Medium; Astra High reviewer; S/M. A child cannot close parent acceptance beyond its bounded outcome.
