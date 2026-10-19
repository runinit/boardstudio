# 07: Fix WASM Clippy warnings across workspace crates and widen the lint gate

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)

**What to build:** All workspace crates and test-support modules compile cleanly under Clippy with `-D warnings` on both native and `wasm32-unknown-unknown` targets, and `scripts/check.py lint` enforces WASM Clippy across the web workspace crates so WASM-only modules stay covered by the standard lint gate.

## Acceptance criteria

- [ ] The WASM-gated and test-support Clippy warnings (`duplicate_mod`, `redundant_closure`, `redundant_locals`, `clone_on_copy`, `collapsible_if`, `type_complexity`, `dead_code`) are resolved cleanly without blanket `#[allow]` suppressions.
- [ ] `python3 scripts/check.py lint` lints the web workspace crates under `--target wasm32-unknown-unknown --all-targets -- -D warnings` and passes with zero warnings or errors.
- [ ] Existing tooling tests for `check.py` stay green with the updated `lint` step.
