# T1-01 INT.1 implementation handoff

- Worker worktree: `/home/chris/.local/share/boardstudio/worktrees/frontend-int1-20261002`
- Branch: `codex/frontend-int1-20261002`
- Base: `f3bb02a3d0eb15cf01822ed29d9b1632048df9c2`
- Commit: `ad483859` (`refactor(web): split private workspace presentation modules`)
- Integration target: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, branch `codex/rust-v1-ui-parity-20261001`
- Reviewed contract: `/tmp/frontend-run/int1-contract.md`
- Astra preimplementation review: `/tmp/frontend-run/int1-contract-review.md` (approved direct private-child modules)

## Change

Moved the existing `Library`, `Objects`, and `Inspector` Dioxus component implementations to `web/src/presentation/{library,objects,inspector}.rs`, registered as direct private children of `presentation.rs`. The Inspector numeric edit state and helpers moved with it. Existing root Runtime/version contexts, hook order, DOM, labels, IDs, panel slots, and action paths are preserved. `web/src/runtime.rs`, `web/src/main.rs`, and manifests are unchanged. No public API visibility changed.

## Checks

- `cargo fmt --manifest-path web/Cargo.toml --all` — passed.
- `cargo fmt --manifest-path web/Cargo.toml --all -- --check` — passed.
- `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target cargo clippy --manifest-path web/Cargo.toml --locked --all-targets -- -D warnings` — passed.
- `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target cargo test --manifest-path web/Cargo.toml --locked --lib` — passed, 12 tests.
- `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --features page --bin boardstudio-web` — passed.
- `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --features page --bin boardstudio-web -- -D warnings` — passed.
- `git diff --check` — passed before commit.

The integration `web/target` was used only while this worker was the sole compiler, as authorized by the coordinator. No `dx` build or browser run was performed by this worker. Root reports existing public F3a evidence at `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-tranche-1/evidence/int1-before/`, browser session `frontend-int1-before`, server PID 1190604/session 89812. Astra's review calls out numeric inspector public traces (preview→Escape, preview→Enter/Apply→Undo, selected-target change and Layout teardown); coordinator-owned integrated acceptance must confirm applicable existing evidence or run those traces.

## Refactoring register

RF-001/RF-002 remain the applicable existing findings. **No new refactoring takeaway observed.** This packet did not change the refactoring register; add this explicit no-new-takeaway result to the workflow register during coordinator integration/handoff.

## Ownership and next action

This commit is on the assigned worker branch and is ready for root/merger integration. Review the integrated source against the approved contract and coordinator's public evidence before marking INT.1 accepted. No source changes remain unstaged.
