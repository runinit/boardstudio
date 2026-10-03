# Parts04 custom definition source handoff

This is an implementation candidate for `.scratch/dioxus-parts-catalogue/issues/04-custom-footprint-authoring.md`, based on planning packet commit `e056a8f8f68d9542a6b94b4534a90a74f7d7bfab` (Sol planning report SHA-256 `68cd4cc48d0b1270277221ca873e5c67b967ddc1aca5c3f3baebf7b2c5e3739d`). It does not close F4.1/F4.2, INT.2, or the public acceptance joins.

## Source boundary

The new private leaf `web/src/parts_custom_definition.rs` prepares one scoped accepted `ReplaceDocument` edit per Definition kind, courtyard, or pad action. Admission checks the accepted snapshot token, session epoch, document identity/revision, Parts scope, and selected project-definition ID. Text and numeric drafts stay local until blur; Enter blurs and Escape restores the accepted value. Invalid dimension/identity edits report recoverable errors and retain the accepted document. Blank X/Y becomes zero, blank drill removes the optional drill, and blank positive dimensions fail validation. Courtyard resizing preserves the prior bounds center and marks the envelope authored. KiCad-source pads remain read-only.

Pad ID edits remap matching pad references for every placed instance using that definition; removal drops only references to the removed pad on those instances. Add pad keeps the `pads.len()+1` default when it is free and increments to the next unused positive number on collision, retaining the approved `[1,3] -> 4` correction. The generated pad ID remains stable and local to that definition. The action still emits one ordinary edit, so history treats it as one Undo unit.

The leaf is mounted inside the existing “Edit footprint” disclosure through the Parts `DefinitionNameEditor` composition. `web/src/main.rs` only registers the private feature module. The shared host/UI mount and CSS are contained in this author branch as a small integration handoff; the root owner must serially join the mount and add presentation rules for `.m1-definition-fields`, `.m1-definition-courtyard`, `.m1-definition-pad-heading`, `.m1-definition-pad`, `.m1-definition-pad-grid`, `.m1-definition-validation`, and `.m1-definition-error` to `web/assets/m1.css` before public visual qualification. Existing project Parts composition remains root-owned.

## Verification recorded on the frozen 629 baseline

All commands used the existing target directory `/home/chris/.local/share/boardstudio/worktrees/f32c-placement-menu-followup-20261002/web/target`; no new target was created.

- `cargo fmt --manifest-path web/Cargo.toml --all -- --check` — pass.
- `git diff --check` — pass.
- `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/f32c-placement-menu-followup-20261002/web/target cargo test --manifest-path web/Cargo.toml --bin boardstudio-web --features page parts_custom_definition::tests --no-fail-fast` — 5 passed. Coverage includes the expected-red legacy candidate (`[1,3]` would attempt duplicate `3`), corrected `4`, ordinary `3` default, one replacement edit, every supported edit kind/field, blank numeric semantics, stale snapshot/selection rejection, and multi-instance pad-reference remap/removal preservation.
- `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/f32c-placement-menu-followup-20261002/web/target cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --all-targets --features page` — pass, including production UI and mounted test compilation.
- `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/f32c-placement-menu-followup-20261002/web/target wasm-pack test --headless --chrome --chromedriver /usr/bin/chromedriver web --bin boardstudio-web -- mounted_tests::mounted_pad_id_action_remaps_every_matching_instance_through_the_runtime_edit --nocapture` — 1 mounted browser test passed. The production field control submitted through the Runtime test seam; the emitted event was then applied by the actual Session/Core harness, remapping both matching instance references, preserving the unrelated pad reference, and restoring the old ID with one Undo.

Strict all-target WASM Clippy with `-D warnings` was not run on this author candidate. The green check above is `cargo check`, not strict Clippy; final strict verification belongs to the joined root package.

## Remaining evidence and review

The mounted regression covers the production pad-ID blur, accepted edit, multi-instance net remap, and Undo. It does not replace the required paired pinned-React/Dioxus public journey for all fields, invalid input recovery, save/reopen, archive round-trip, desktop/compact layout, or full Undo/Redo. Those remain open.

## Follow-up candidate after exact-629 review

Sol's source review of `629e4cdfbaae0032efb59e022db0cfe1f69d89cd` found owner-transition draft leakage, coupled sibling-field reset, and an unchanged courtyard-blur edit for empty/nonrectangular geometry. The current follow-up source keys pad drafts by scope/definition/pad identity, resets courtyard drafts and errors when the selected owner changes, synchronizes each accepted field independently, and submits courtyard dimensions only when that scalar draft differs from its accepted value. Mounted browser regressions cover an equal-value A→B owner switch with dirty courtyard and pad drafts/error state, and focus/blur without editing an empty courtyard. These fixes are a separate source commit; only its exact SHA and green results establish their disposition.

The follow-up used the same existing Cargo target. Focused native `parts_custom_definition::tests` passed 5/5; WASM `cargo check --all-targets --features page` passed; the mounted owner/error/draft synchronization test passed 1/1; the mounted unchanged-blur/empty-courtyard test passed 1/1. The two mounted tests explicitly opened the disclosure before sending browser focus/input/blur events.

The approved Add pad correction also has an actual expected-red run against a disposable mutation of frozen 629. Worktree `/home/chris/.local/share/boardstudio/retained-tmp/20261002/parts04-add-pad-legacy-red` starts at exact `629e4cdfbaae0032efb59e022db0cfe1f69d89cd`; only `next_pad_number` was restored to legacy `pads.len()+1`. The existing test `parts_custom_definition::tests::add_pad_keeps_free_default_and_repairs_collision_as_one_edit` failed as expected for `[1,3]`: actual `3`, expected corrected `4`, exit 101. Retained log: `.scratch/dioxus-parts-catalogue/evidence/parts04-add-pad-legacy-expected-red-20261003.log`, SHA-256 `030bbd2b48de858d6ca4cab81e34da4277b9919b325b2a7160aa8c31a54bf042`. The mutation worktree is separate; frozen 629 and the corrected source are unchanged by that run.

Strict all-target WASM Clippy with `-D warnings` was not run on the follow-up candidate. The WASM result above is `cargo check`, not strict Clippy; final strict verification belongs to the joined root package.

No new refactoring takeaway was observed. The implementation uses the existing private feature leaf and accepted Session/history boundary (RF-002), the shared Parts composition remains a known integration hotspot (RF-001), field edits are scoped to the accepted project definition (RF-006), and source/build/browser proof remains separately accounted (RF-009). No RF register row is added or claimed resolved here.
