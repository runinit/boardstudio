# Parts04 custom definition source handoff

This is a reviewed-start implementation candidate for `.scratch/dioxus-parts-catalogue/issues/04-custom-footprint-authoring.md`, based on planning packet commit `e056a8f8f68d9542a6b94b4534a90a74f7d7bfab` (Sol planning report SHA-256 `68cd4cc48d0b1270277221ca873e5c67b967ddc1aca5c3f3baebf7b2c5e3739d9`). It does not close F4.1/F4.2, INT.2, or the public acceptance joins.

## Source boundary

The new private leaf `web/src/parts_custom_definition.rs` prepares one scoped accepted `ReplaceDocument` edit per Definition kind, courtyard, or pad action. Admission checks the accepted snapshot token, session epoch, document identity/revision, Parts scope, and selected project-definition ID. Text and numeric drafts stay local until blur; Enter blurs and Escape restores the accepted value. Invalid dimension/identity edits report recoverable errors and retain the accepted document. Blank X/Y becomes zero, blank drill removes the optional drill, and blank positive dimensions fail validation. Courtyard resizing preserves the prior bounds center and marks the envelope authored. KiCad-source pads remain read-only.

Pad ID edits remap matching pad references for every placed instance using that definition; removal drops only references to the removed pad on those instances. Add pad keeps the `pads.len()+1` default when it is free and increments to the next unused positive number on collision, retaining the approved `[1,3] -> 4` correction. The generated pad ID remains stable and local to that definition. The action still emits one ordinary edit, so history treats it as one Undo unit.

The leaf is mounted inside the existing “Edit footprint” disclosure through the Parts `DefinitionNameEditor` composition. `web/src/main.rs` only registers the private feature module. The shared host/UI mount and CSS are contained in this author branch as a small integration handoff; the root owner must serially join the mount and add presentation rules for `.m1-definition-fields`, `.m1-definition-courtyard`, `.m1-definition-pad-heading`, `.m1-definition-pad`, `.m1-definition-pad-grid`, `.m1-definition-validation`, and `.m1-definition-error` to `web/assets/m1.css` before public visual qualification. Existing project Parts composition remains root-owned.

## Verification on this exact candidate

All commands used the existing target directory `/home/chris/.local/share/boardstudio/worktrees/f32c-placement-menu-followup-20261002/web/target`; no new target was created.

- `cargo fmt --manifest-path web/Cargo.toml --all -- --check` — pass.
- `git diff --check` — pass.
- `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/f32c-placement-menu-followup-20261002/web/target cargo test --manifest-path web/Cargo.toml --bin boardstudio-web --features page parts_custom_definition::tests --no-fail-fast` — 5 passed. Coverage includes the expected-red legacy candidate (`[1,3]` would attempt duplicate `3`), corrected `4`, ordinary `3` default, one replacement edit, every supported edit kind/field, blank numeric semantics, stale snapshot/selection rejection, and multi-instance pad-reference remap/removal preservation.
- `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/f32c-placement-menu-followup-20261002/web/target cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --all-targets --features page` — pass, including production UI and mounted test compilation.
- `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/f32c-placement-menu-followup-20261002/web/target wasm-pack test --headless --chrome --chromedriver /usr/bin/chromedriver web --bin boardstudio-web -- mounted_tests::mounted_pad_id_action_remaps_every_matching_instance_through_the_runtime_edit --nocapture` — 1 mounted browser test passed. The production field control submitted through Runtime into Session, renamed both matching instance references, preserved the unrelated pad reference, and Undo restored the prior pad ID.

Strict all-target WASM Clippy with `-D warnings` was not run on this author candidate. The green check above is `cargo check`, not strict Clippy; final strict verification belongs to the joined root package.

## Remaining evidence and review

The mounted regression covers the production pad-ID blur, accepted edit, multi-instance net remap, and Undo. It does not replace the required paired pinned-React/Dioxus public journey for all fields, invalid input recovery, save/reopen, archive round-trip, desktop/compact layout, or full Undo/Redo. Those remain open. The expected-red fixture currently encodes the reference duplicate candidate and proves the corrected helper chooses `4`; no separate run against a mutated/old production implementation is claimed.

No new refactoring takeaway was observed. The implementation uses the existing private feature leaf and accepted Session/history boundary (RF-002), the shared Parts composition remains a known integration hotspot (RF-001), field edits are scoped to the accepted project definition (RF-006), and source/build/browser proof remains separately accounted (RF-009). No RF register row is added or claimed resolved here.
