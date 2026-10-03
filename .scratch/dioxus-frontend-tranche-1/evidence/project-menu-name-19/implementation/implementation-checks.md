# Project-menu name implementation checks

Source is frozen at `ffaaf8382f1addb0923524f9ee14ab40c308fe8a` on `codex/project-menu-name-19-20261002`, based on integration source `9c92e9fa7886a8b149b24c3f9fe44e71d6249a6a`.

The loaded Project menu now renders the accepted project's name field only when an accepted document exists. The field keeps a same-project draft across unrelated accepted revisions, rebases its synchronous submission capture onto the latest accepted snapshot, and submits a normal `ReplaceDocument` edit changing only `name`. Accepted name changes reset the draft, so the existing Undo/Redo/reopen flow remains authoritative. Enter blurs, Escape restores, and blank/unchanged blur submits nothing. Project identity includes `SessionEpoch` and document ID; submission also rechecks snapshot token and revision immediately before dispatch.

The production-mounted `Library` regression exercises the actual DOM input and `Runtime::submit` seam, then applies the captured event to a real `Session`/`CoreEngine`. It verifies trimmed blur, preservation of an unrelated accepted field, accepted-value refresh, Undo/Redo, Enter, Escape, blank/unchanged no-op behavior, replacement project identity, and reopening the same project ID under a new session epoch.

Checks run:

- Focused mounted headless-Chrome test: 1 passed, 0 failed, 114 filtered. Normalized-log SHA-256 `1c1f8c1fc5705924dab15e19b45199157ce46f2c834f5412771953f410bb5c6d`; byte-exact raw log is retained at `/home/chris/.local/share/boardstudio/retained-tmp/20261002/project-menu-name-19/mounted-chrome.raw.log`, SHA-256 `9a86addebf00aea711d7722724354d1b3c3823a26cbfa92fa3f9e12b5941b345`. Command: `CARGO_TARGET_DIR=<reused frontend-new-keyboard-20261002 target> CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=<installed wasm-bindgen-test-runner> CHROMEDRIVER=/usr/bin/chromedriver WASM_BINDGEN_TEST_ONLY_WEB=1 cargo test --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web project_menu_name_draft_survives_unrelated_accepted_revision_and_commits_latest_document -- --nocapture`.
- Strict WASM Clippy, all targets, `-D warnings`: passed. Log SHA-256 `19a8bd6af999ab5ab9ae86f180de480b9744b0433e86f600d44d3fe5fc168368`. Command: `CARGO_TARGET_DIR=<reused frontend-new-keyboard-20261002 target> cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings`.
- Native web library tests: 19 passed, 0 failed (`cargo test --manifest-path web/Cargo.toml --lib --no-default-features --features page`).
- `cargo fmt --manifest-path web/Cargo.toml -- --check` and `git diff --check`: passed before the source commit.

The source audit expected-red baseline remains the pre-port browser observation in `evidence/project-menu-name-19/source-audit/` from docs commit `82695ffa2942f11d3deaa6a8262aca38ef5d5b7c`: pinned React showed the labelled field; the served Dioxus menu had no such input. This source-only implementation check does not claim the paired root-package rename, compact/theme, persistence-failure, or saved-reopen acceptance journey; those remain integration/public qualification joins.

No new RF-006/RF-009 refactoring takeaway was found in this bounded change.
