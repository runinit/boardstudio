# PCB07 async owner repair

This bounded follow-up addresses the two holds in `/tmp/pcb07-source-correction-independent-review-20261002.md`.

The production admission callback now owns a hook-lifetime flag with `use_drop`. After packaged generator-source classification resolves, it checks that flag before reading the live workspace or scope-generation signals and before calling the selection predicate. It retains the workspace signal rather than snapshotting its current string before `spawn_local`.

The focused mounted Dioxus tests exercise the exact production lifetime/context guard used by the callback. The owner is unmounted while classification is pending, then the future resolves; the dropped signals are not read. A second test switches the live workspace from PCB to Layout while the future is pending; admission is rejected. For red/green, temporarily moving the workspace read before the alive test and omitting the live-workspace comparison made both tests fail: the unmounted case raised `ValueDroppedError`, and the changed-workspace case incorrectly admitted. The production guard was restored and both tests pass.

Checks:

- `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page --test pcb_part_net_admission` — 2 passed.
- `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --all-targets -- -D warnings` — passed.
- `cargo fmt --manifest-path web/Cargo.toml --check` — passed.
- `git diff --check` — passed.

This is a focused owner-guard regression, not full PCB07 acceptance. The real packaged generator classifier, proposal-admission tests, mounted Inspector, paired browser journey, history/undo/save-reopen, and full parent gates remain separate requirements.
