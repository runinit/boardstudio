# WASM test-target compilation repair — 2026-10-02

Integration worktree `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, branch `codex/rust-v1-ui-parity-20261001`.
Base: `56a715149d0dde2963f0985be217269bdc97a179`.
Repair commit: **`a02dac961e7bd1644194a35b042dd24dc7ed326f`** (`Fix WASM test-target compilation checks`).

Changed exactly two test-only sites:

- `web/src/presentation/parts/catalogue.rs`: explicit match on `option_env!` retains the existing actionable panic when the package-test runner did not supply its generated module URL. `Some` still imports and executes the actual packaged module. No ignore, return-success fallback, compile-time environment requirement, or lint suppression was introduced.
- `web/src/presentation/pcb_wiring.rs`: imports `SnapshotToken` inside its existing tests module, fixing the missing symbol after the production import was narrowed.

The two files had no overlapping dirty edits. Preserved unrelated dirty Case README and all untracked work. No config, visibility, schema, production behavior or acceptance-gate change.

## Reproduction and verification

Red before editing: `cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings` exited **101**, showing both expected failures:

- `catalogue.rs:362`: denied `clippy::option_env_unwrap` for the test module URL.
- `pcb_wiring.rs:595`: `E0425`, `SnapshotToken` not in scope.

After repair:

- Same exact WASM **all-targets** strict Clippy command: **passed**.
- `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page`: **48 passed** (12 library + 36 page).
- `cargo fmt --manifest-path web/Cargo.toml -- --check`: **passed**.
- `git diff --check` and staged diff check: **passed**.
- `pnpm run test:web:physical-setup`: **1 real packaged Gateron WASM test passed**; runner rebuilt module assets, supplied the compile-time URL and exercised immutable proposal/normalized output assertions.

The original merge report's strict WASM command lacked `--all-targets`. That earlier pass covered the production page, not its WASM test-only modules. This record establishes the broader test-target compile gate; it does not claim that all browser/WASM tests were executed. Runtime browser and feature acceptance remain separate.
