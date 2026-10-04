# F5.6 ticket 06 implementation handoff — Editor-owned physical setup

## Exact reviewed planning inputs

- Issue 06 SHA-256: `e15e0a54ad15c36d85e7f9f75c97324db00126fdc507269a0ef8e55bd0b508cb`
- Prerequisite spec SHA-256: `546024a3ab50565e3b9afd30689ce752174c066652886c74ca879d1c37eebc30`
- Independent source Spec and Standards clearances for ticket05 are recorded in `/tmp/frontend-wave-source-spec-review-20261002.md` and `/tmp/frontend-wave-source-standards-review-20261002.md` for corrected ticket05 commit `9e9459a5826514d7cb060cac7a073cafcb834582`.

## Implemented in this branch

- Added an unconditional Editor-lifetime owner in `presentation/pcb_physical_setup/controller.rs`, beside the PCB wiring owner and before Editor's early returns. It captures the accepted snapshot identity and typed Project/Case intent, invokes the Parts-owned packaged proposal bridge, rechecks owner identity after asynchronous normalization, registers the exact operation outcome before `ReplaceDocument`, retains settlement while Case UI is hidden, and reconciles a topology primary only after the exact accepted proposal settles successfully. Project-guide admission is based on accepted `model.active_board_id`/token/revision/generation and does not require tree selection or a `Scope`; Case Inspector admission remains instance-scoped.
- Mounted the TypeScript Case-context physical-assembly information, half-connection select, and wired TRRS guidance in the Case Inspector. It submits a typed transport intent. Added matching M1 styles.
- Added a Project-stage control Element slot with typed topology/transport/reversible actions for the concurrent SetupGuide stream. The slot is intentionally not mounted yet; the Project guide is not present in this worktree. Root is coordinating its `guide_open && guide_stage == SetupGuideStage::Project` admission input. Do not relocate these controls to another workbench.
- Kept packaged module loading, normalizer calls and proposal construction private to Parts. No public Rust API or project schema changes.
- Added an acceptance guard test: selection reconciliation accepts only a completed, current operation whose durable accepted document exactly matches the immutable proposal (aside from Session-assigned revision); persistence failure, stale accepted content, and owner changes reject reconciliation.

## Checks

- `cargo fmt --manifest-path web/Cargo.toml`
- `cargo test --manifest-path web/Cargo.toml --features page --locked` — 43 tests passed (12 library, 31 binary).
- `cargo check --manifest-path web/Cargo.toml --features page --target wasm32-unknown-unknown --locked` — page compiles. The unmounted Project-stage Element slot and its three actions currently produce expected dead-code warnings; the warning-clear check depends on the SetupGuide slot being connected. No lint suppressions were added.
- `git diff --check` — passed.

## Open evidence and joins

- The Project guide Element/active-stage value is still coordinator-owned. Until it is connected, the Case transport intent is mounted and usable; Project topology/reversible actions cannot be exercised in-app. The current dead-code warnings are only for the unmounted Project controls/actions; they are expected to clear at the guide join, and must not be suppressed.
- The full F5.6a paired TypeScript/Dioxus browser journey, accepted topology selection, Undo/Redo, save/reopen, hidden-panel durability, and actual packaged-normalizer UI path remain open. The F5.6a journey is still the sole public paired acceptance suite.
- Case PCB selector and per-half flip controls remain excluded by the prerequisite spec. Do not expand this slice into those features.
- No shared run/refactor ledger or publication JSON was edited. Source findings remain within RF-001, RF-006 and RF-009; this slice adds no new refactor ID.

## Paired F5.6-C01 Project-stage qualification — 2026-10-04

- Compared the frozen Dioxus app at `http://127.0.0.1:34802/` and React app at `http://127.0.0.1:5175/` using the same imported Sofle v2 archive (`layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`) in isolated sessions.
- Both began Split/Wired/non-reversible. Selecting One keyboard accepted that topology and hid the half-connection controls. Returning to Split restored those controls and selected Wireless in both. Wired↔Wireless changes and enabling Reversible layout updated the selected control states and corresponding copy identically.
- Undo twice then Redo twice restored the same Split/Wireless/reversible state in both. Reloading each same imported session retained that accepted state. Undo after reload reported empty history in both; no post-reload history persistence was expected or claimed.
- No F5.6-C01 action/result mismatch was reproduced in this bounded paired journey. This is source/fixture-specific qualification, not closure of the full F5.6a acceptance suite or F5.8.
