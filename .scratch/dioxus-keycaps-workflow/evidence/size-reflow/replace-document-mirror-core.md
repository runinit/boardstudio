# F6C.3 accepted `ReplaceDocument` and linked-half regression

This is supporting Core-path evidence for the size/reflow slice. It does not close the paired React/Dioxus browser journey, mounted lifecycle, Inspector placement, or whole F6C.3 gates.

## Fixture provenance

- Pinned BoardStudio fixture: `/tmp/keycaps-fit-fixture.boardstudio`
- Fixture SHA-256: `f2c38c70edbb02994c98b8fbd1eadfe99143b5d637916414dbeb4546162324bb`
- Extracted project JSON: `/tmp/keycaps-fit-fixture-project.json`
- Project JSON SHA-256: `2d5cdfd8c6d31cb5150b92fe7354f8b7fc01be996d1aecfbed91287591fc59ca`
- The regression changes the Sofle fixture's second half to a same-board linked layout, then opens it through `CoreEngine`.

## Accepted path exercised

The test plans a width resize from the source matrix cell, submits the exact `CoreRequest::Edit` with `EditOperation::ReplaceDocument` and `EditPhase::Commit`, then verifies the accepted revision, source key size, Core-synchronized linked key size and reflected pose, neighboring row reflow on both linked halves, and selected-group center preservation. Core Undo and Redo are both exercised and checked against the accepted document.

This proves that the chosen document replacement operation enters Core's linked-layout synchronization and history path for this fixture. It does not prove the Dioxus browser control dispatches the same request, nor React parity for event timing or visual feedback.

## Reproduction

```sh
KEYCAPS_MIRROR_PROJECT_JSON=/tmp/keycaps-fit-fixture-project.json \
  cargo test --manifest-path web/Cargo.toml --bin boardstudio-web \
  presentation::objects::keycap_resize::tests::replace_document_reflows_source_and_core_syncs_linked_partner_through_history \
  -- --ignored --exact --nocapture
```

Result: **1 passed**.

## Supporting checks

- `cargo test --manifest-path web/Cargo.toml --bin boardstudio-web presentation::objects::keycap_resize::tests -- --nocapture`: 4 passed, 1 intentionally ignored (the fixture-gated test above).
- `cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page --bin boardstudio-web`: passed.
- `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --all-targets --features page -- -D warnings`: passed.
- `cargo fmt --manifest-path web/Cargo.toml --check`: passed.
- `git diff --check`: passed.

Warnings from unrelated existing native test-only dead code are emitted by `cargo test`; the strict wasm Clippy target passes without suppressions.
