# F5.2d current-plan Apply source receipt

Source commit: `e68702543397e7cf5301fed7e9b93286a6b2bf44` on `codex/pcb-wiring-mode-apply-20261002`.

This bounded private Editor change adds **Apply wiring** to the board-level PCB Wiring inspector. It revalidates the saved accepted board/UI scope and exact current `ElectricalPlan`, calls Core `electrical::materialize` on an accepted-document clone, and submits one existing `ReplaceDocument` edit with the registered `OperationId` outcome. The control is disabled unless the current board-level plan and Session conditions are actionable. It does not add a public API, schema, Core implementation, or a second history owner.

The production owner is mounted in `presentation.rs`; the inspector prop is routed through the existing private workspace composition. Mounted owner coverage checks that a current plan submits one board-targeted edit containing the exact generated electrical materialization, that the Session's advanced saved proposal settles to Saved feedback, that a retained callback is rejected after selection changes, and that a Session rejection leaves the accepted revision unchanged and shows failure feedback.

Validation on the frozen source:

- `cargo test --manifest-path web/Cargo.toml --features page presentation::pcb_wiring::mode_owner_tests -- --nocapture`: 8 passed, 0 failed. Log SHA-256: `af1935160bac79abb8e1e88d6e4fea5f9d9366e6551897694ac90c637f98d934`.
- `cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page`: passed after the workspace-composition relay was added.
- `cargo fmt --manifest-path web/Cargo.toml --check` and `git diff --check`: passed.

Direct Core request coverage is separately frozen in `09b9c014c8b87213bf1e5e130b59a6be4be42e92` and independently reviewed CLEAR by Sol Parts at `/home/chris/.local/share/boardstudio/reviews/core09-persisted-lock-coverage-review-09b9c014-sol-20261002.md` (SHA `289ed2dd81c0bb05926cf2a5f4d7b95911c85e0902f0822bef09ed6b105c3179`). That packet verifies existing saved-board lock behavior; it is not a Core defect or an Apply dependency.

Source review, root serial join, integrated strict checks, packaged same-fixture Apply, accepted history, Undo/Redo, and save/reopen remain separate gates. This receipt makes no public or parent-completion claim.
