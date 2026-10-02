# Final source, regression-test, and formatting review (`b50ddbdd`)

Integration base: `b9748745ec444416136a97d4befd075670ef49b7`  
Final candidate source: `b50ddbdd6b74a2489e80078e4a0d2923aeca6edb`  
Merged provider worktree head: `e1e8606ec04ba49ff8846b88b4d2c7aab9dbe003`  
Production paths: `web/src/presentation.rs`, `web/src/runtime.rs`, `application/tests/durable_session.rs`
Exact binary diff SHA-256 for those paths relative to the integration base: `7895289585011358589b7a2efdf40a68948a9bbb1fc3651ddc58bf111d81fdb7`

## Review result

No blocking Spec or regression-test findings. The reviewed startup restoration calls the existing scoped `active_project_id("")` store query and delegates to `open_saved`, which uses the established open-sequence identity so an explicit later open supersedes delayed restoration. Missing or unreadable saved state leaves a public recovery route. Physical-instance choices are restricted to instances belonging to the active board, include a canonical-board option, and dispatch the existing `Navigate` event. Session validation checks board and instance membership before changing scope; accepted navigation invalidates gestures and cancels generation/export work. No persisted field or public API was added.

The session regression uses public events and completions plus `CoreEngine::handle`. It covers canonical-to-instance-to-canonical scope changes, generation and export cancellation, and rejection of unknown board/instance IDs without corrupting scope. The pre-format Standards review of the test is in `standards-instance-navigation-test-a350f467.md`; it found no findings.

The final `b50ddbdd` commit contains only rustfmt changes to `application/tests/durable_session.rs` and `web/src/runtime.rs` relative to the reviewed pre-format source/test candidate (formatting commit diff SHA-256: `34dba988dc875849fb0f62c4cb8f8fdb9195ad2f5bcde3856ad14ac876ca8e3a`). No behavior change was introduced by formatting. Exact final source review and test behavior therefore match the reviewed diffs plus formatting.

## Reported validation

Provider reports the following successful checks on final candidate source `b50ddbdd`:

- `cargo fmt --manifest-path application/Cargo.toml -- --check`
- `cargo fmt --manifest-path web/Cargo.toml -- --check`
- `cargo test --manifest-path application/Cargo.toml --locked` (14 passed)
- `cargo test --manifest-path web/Cargo.toml --locked --no-default-features --features page,cad-worker` (12 passed)
- `cargo clippy --manifest-path application/Cargo.toml --locked --all-targets -- -D warnings`
- `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --all-targets --no-default-features --features page -- -D warnings`
- `dx build --web --release --base-path / --no-default-features --features page --cargo-args=--locked` (focused component stage, exit 0)
- `git diff --check`

The exact focused build asset record is `.scratch/m1-production/evidence/integration/startup-restore-instance-focused-b50ddbdd.json`. Public browser green evidence and the final maintained release build are pending; this report is not an M1 acceptance claim.
