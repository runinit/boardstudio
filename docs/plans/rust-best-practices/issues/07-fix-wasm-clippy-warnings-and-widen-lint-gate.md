# 07: Fix WASM Clippy warnings across workspace crates and widen the lint gate

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** All workspace crates and test-support modules compile cleanly under Clippy with `-D warnings` on both native and `wasm32-unknown-unknown` targets, and `scripts/check.py lint` enforces WASM Clippy across the web workspace crates so WASM-only modules stay covered by the standard lint gate.

## Acceptance criteria

- [x] The WASM-gated and test-support Clippy warnings (`duplicate_mod`, `redundant_closure`, `redundant_locals`, `clone_on_copy`, `collapsible_if`, `type_complexity`, `dead_code`) are resolved cleanly without blanket `#[allow]` suppressions.
- [x] `python3 scripts/check.py lint` lints the web workspace crates under `--target wasm32-unknown-unknown --all-targets -- -D warnings` and passes with zero warnings or errors.
- [x] Existing tooling tests for `check.py` stay green with the updated `lint` step.

## Answer

Resolved across the workspace and `scripts/check.py`:
- Shared `core/tests/support/mod.rs` once in `web/crates/host/src/lib.rs` so `boundary.rs` uses `crate::support::{replacement_electrical, synthetic_half}` on both native and WASM targets without triggering `clippy::duplicate_mod`.
- Fixed WASM-gated and test-support Clippy lints across `ui-shared`, `library`, `keycaps`, `keymap`, `parts`, `pcb`, and `layout`.
- Widened `scripts/check.py` `LINT` (`WEB_WASM_PACKAGES`) to run `cargo clippy --target wasm32-unknown-unknown --all-targets --locked --no-deps -- -D warnings` over `boardstudio-web` and all 12 `boardstudio-web-*` workspace crates, and updated `scripts/test-check.py`.
- Verified with `python3 -B scripts/test-check.py` (31 passed) and `python3 scripts/check.py repo tooling lint typecheck` (all passed).
