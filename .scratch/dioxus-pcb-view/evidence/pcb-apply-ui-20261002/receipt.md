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

## Public Apply history qualification (2026-10-03)

Candidate: `frontend-module-ownership-20261003`, source `7ddd10e3`, `http://127.0.0.1:34780/`, named browser session `bs-mig-13b38a458f03`. Reference: pinned React at `http://127.0.0.1:5175/`, session `pcb-apply-ts-final-7dd31abc1fc4`. Both imported the exact populated-controller fixture `../../../dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`; Left PCB had 70 parts and controller `left-U1` (`mcu nice nano`).

In both apps I changed only the Left PCB `Row 1` assignment from P5 to the free P101 pin, then applied wiring once. The candidate displayed `Wiring plan applied and saved.` and settled at project revision 11. One Undo settled at revision 12; one Redo settled at revision 13. The public portable exports prove accepted state independently of the plan selectors and transient feedback. In `project.json`, `generated/electrical/left/row/0` binds the controller to pad-14 with assignment P5 before Apply and after Undo, and pad-24 with assignment P101 after Apply and Redo. The pin-lock proposal remains P101 in both restored states; the accepted assignment and generated net are the history target.

Normalized board-state comparison ignored revision/history metadata and included Left PCB part IDs, net IDs, all `generated/electrical/left/*` nets, and Left hardware mode, controller, assignments, and locks. The normalized hashes are identical before Apply and after Undo (`a715808ded049e29fa31c831f8dc8b5b55897fbfe323bdeb584d77c014461d6e`) and identical after Apply and after Redo (`6e78dc7ca8c70aada8474871c81228820ad9f2d8e0e911927cdbfccdd4f82152`). The React reference produced the same four normalized states. Candidate Row 1’s transient planner selector display differed while the plan was refreshing; the portable accepted-document comparisons confirm the intended board binding history and no defect is inferred from the planner projection.

| Export | SHA-256 | Revision | Row 1 accepted assignment | Controller pad on `row/0` |
| --- | --- | ---: | --- | --- |
| `candidate-before-apply.boardstudio` | `83b0039c735cf954167ea88a2431fd0aa1bcdc28995146ba69de982910aae3c6` | 10 | P5 | pad-14 |
| `candidate-after-apply.boardstudio` | `57b5599bb17a13e34afaddaf84011fbbc0c5b2e36b8cca8551c4f885e10a3b1e` | 11 | P101 | pad-24 |
| `candidate-after-undo.boardstudio` | `ed13d0485e46c2b0501e30da56bb2b6d9e92a2465d4dbee8ae51e929f58683b7` | 12 | P5 | pad-14 |
| `candidate-after-redo.boardstudio` | `201faa2c24dbcc0815c87445929b36e3e38e1b1ed35226d9b5155c52609a4126` | 13 | P101 | pad-24 |
| `react-before-apply.boardstudio` | `db5a28c906342a0508401a0145a5cc57f7696e6aec90333f8945c7449f0d307e` | 10 | P5 | pad-14 |
| `react-after-apply.boardstudio` | `2ae8d05242e6eb8fd7cc5fdf5d760ee7739aae2ceb4088645d53a92e134d0357` | 11 | P101 | pad-24 |
| `react-after-undo.boardstudio` | `9c95cb214e9a4ead45086873ba2fa3ddfb6f3b18035599cdc2f12033f42a7896` | 12 | P5 | pad-14 |
| `react-after-redo.boardstudio` | `874a2ceb946bf959249b4f3f97597cc9637ed2974fd640e5e099ac0fd9d630b7` | 13 | P101 | pad-24 |

This qualifies one changed Apply history step on the populated fixture and its paired public reference. Pin-conflict behavior, reload/reopen history, and other F5.2/F5.3 branches were not revisited.
