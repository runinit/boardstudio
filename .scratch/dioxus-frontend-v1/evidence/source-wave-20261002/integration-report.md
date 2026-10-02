# PCB05 + F5.2b reviewed-source integration — 2026-10-02

Role: bounded merger under implement-spec. Integration worktree `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, branch `codex/rust-v1-ui-parity-20261001`.

Starting HEAD: `454c493be293fe61250e2e4abc19191b6570061f`.
Final HEAD: **`837002729024475796cdc0fb99089ac86d6b33e7`**.

## Serial source integration

| Reviewed/source commit | Integration commit |
| --- | --- |
| PCB05 source `0dd1f3ce904dc11ba62d175dba078729d5263472` | `329da6815dd910565f5bbd01f50cf0b266f9f13c` |
| PCB05 correction `9e9459a5826514d7cb060cac7a073cafcb834582` | `31258f745ab0ac72247ec77240585716eeea62cc` |
| F5 source `6fe8fb875585fca7fba4636cf1a9696e55b5406f` | `0a576177723fef9daf164e3b495a6a405ea48dd8` |
| F5 settlement/slot correction `ebe58a38b250e6ba35b683aa7f03155a3e8046a4` | `308087a8913fa8e312b44cddfab844a49f22ac30` |
| F5 suppression removal `05b4611b8a8adcdd02f89a53a0efab70bfab5394` | `837002729024475796cdc0fb99089ac86d6b33e7` |

All five cherry-picks succeeded without manual conflict resolution. Git automatically combined the independent `web/src/main.rs` module registrations. The final main module keeps physical_setup test-only and firmware_position_projection available for page tests/WASM, preserving reviewed semantics.

Before picking, inspected tracked diff and computed exact intersection of every incoming path against untracked files: **no overlapping untracked file**, and only the unrelated `.scratch/dioxus-case-workspace/README.md` was tracked dirty. That README diff remains intact. All 227 untracked files remain untracked. No stash/reset/discard/removal, history rewriting, public API changes, checker weakening, or source edits were performed. Documentation-only merge commits were not picked.

Source identity verification compared final Git blobs to approved source commits: all eight PCB files (`physical_setup.rs`, Parts module/catalogue/adapter, package test script, Cargo manifest/lock, package.json) match `9e9459a5` exactly; all five F5 files (projection, wiring component/controller, presentation root, workspace composition) match `05b4611b` exactly. The combined main module was inspected separately.

## Executed checks on integrated HEAD

- `cargo fmt --manifest-path web/Cargo.toml -- --check`: **passed**.
- `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page`: **passed**, 12 library + 36 page tests, zero failures/ignored.
- `cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown -- -D warnings`: **passed**.
- `git diff --check 454c493b..HEAD`: **passed**.
- `git diff --check`: **passed**.

The exact-source review already reran PCB05 real packaged Gateron WASM proof (1 pass) and seven native proposal tests, plus six F5 projection/admission/SessionCore tests. This integration does not replace mounted browser proof.

## Retained acceptance limits

PCB05 is a tested prerequisite only: ticket06 owns production callable activation, Editor operation lifetime and accepted selection. F5 exposes the reviewed private Element handoff with numeric version subscription; F6 owns mounting controls and delayed save/failure/edit/Undo/Redo/reopen browser acceptance. No source feature ticket or parent is closed by this merge. RF-001/006/009 handoffs and source-review limitations remain as recorded in `/tmp/frontend-wave-source-standards-review-20261002.md`.

## Verification scope correction — later all-targets repair

The original WASM Clippy pass above **did not use `--all-targets`**. It covered production page compilation and did not compile WASM test-only code. Running the broader required command at later integration `56a71514` exposed two test-only failures: `option_env_unwrap` in the packaged normalizer test and a missing `SnapshotToken` import in PCB wiring tests. Both were reproduced, minimally fixed in `a02dac961e7bd1644194a35b042dd24dc7ed326f`, and verified by the actual `--target wasm32-unknown-unknown --all-targets -- -D warnings` command. Native48 and real packaged WASM1 also pass. Exact red→green commands and limits: `/tmp/frontend-wasm-test-target-repair-20261002.md`. The original historical pass is retained with this narrower coverage explicitly stated.

## Keycaps final-source integration — 2026-10-02

Started from `a02dac961e7bd1644194a35b042dd24dc7ed326f` and serially cherry-picked only these four feature/fix commits:

| Source | Integration |
| --- | --- |
| `448c3274693b611ad747d0461de1b18d88245fb8` | `ca091ecfe085f447fb5ed207cda1a082674385b3` |
| `1741404b9a1da02de53dc5523524525324716db3` | `b4249228184ddd156271a7bdcc09cacc58334c7b` |
| `4d9196d75e272addced891b6988a9dbd330064e4` | `2cd11c9ed185ebb0f5046aef000c73b433e2ee86` |
| `13533ccf373cbdda0ca0c478744907117ea906cf` | `3061935ee6cd25ff8559023390d8c9f718e2338e` |

All picks succeeded; presentation root and Cargo manifest merged automatically. Reviewed module, workspace, lifecycle harness, Runtime and CSS blobs match approved `13533ccf` exactly. Shared composition retains firmware controls and both native Dioxus/WASM test dependencies. No incoming path overlapped dirty/untracked work, and unrelated changes were preserved.

Checks at integrated source `3061935e`:

- `cargo fmt --manifest-path web/Cargo.toml -- --check`: passed.
- `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page`: **54 passed** (12 library, 36 page, 6 mounted lifecycle/state tests).
- `cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings`: passed, including WASM test targets.
- Committed-wave and working-tree `git diff --check`: passed.

Latest independent Spec/Standards reports and the WASM test-target repair report are copied into `.scratch/dioxus-frontend-v1/evidence/source-wave-20261002/`. This integrates reviewed source only; final candidate browser parity/currentness/failure/edit/Undo/Redo/reopen gates and all parent acceptance remain open.
