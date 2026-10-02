# Parts13 implementation handoff — 2026-10-02

## Source and authority

- Authoring worktree: `/home/chris/.local/share/boardstudio/worktrees/parts13-new-component-author-20261002`
- Branch: `codex/parts-new-component13-20261002`, based on migration source `d62584724506f27dbb6392ea8b8bc9b26b3a2fc2`; the frozen source `e3566700` is an ancestor.
- Reviewed contextual spec SHA-256: `f739b30e6e4fb5aa743ccfd0be3b069a81abd701d96297857a0251bfa48f9884`.
- Reviewed ticket SHA-256: `81b7df372a7d0b0db907ece02748986604e12b0632a94f4e428d8984d5e9483b`. The current canonical draft is a status-only successor at SHA-256 `c50934a967017604ad57a3144d43f2c2cf4be3af028ebf1892fa23a4a85673c3`.
- Capability receipt SHA-256: `45801157346a296d63b60cab4a61f40b2d05ada3fbf8ffff4f1d9901fe72159b`.
- Independent planning review is clear at `/home/chris/.local/share/boardstudio/reviews/parts13-planning-review-sol-20261002.md`.

## Implementation boundary

`web/src/parts_new_component.rs` owns the private create proposal, fresh `ui-` identity allocation with collision avoidance, default custom definition and accepted-operation reconciliation. It submits one current-document `ReplaceDocument` edit through existing Runtime/Session/Core operation and history behavior. Its outcome handler only changes Parts selection, query and panels after this operation completes and only while the same accepted project scope, active Parts view, mount and local query/selection generation remain current.

The coordinator mount patch registers that private module, keeps the action mounted in the common Parts catalogue-actions area across loading, error, empty and no-match content, and advances the Parts-local generation on each actual search or catalogue-selection action. It uses the existing `on_select` panel transition and does not alter any canonical parent edge or acceptance criterion. The button remains visible during catalogue loading and error.

The Dioxus Name editor remains the existing issue12 child. No Core operation, shared wire type, file-format field, public API, Runtime visibility, CSS, or board-placement behavior was added. The required `m1-parts-actions` CSS hook is left to the coordinator-owned stylesheet patch.

## Verification

- `cargo fmt --manifest-path web/Cargo.toml -- --check` — PASS.
- `cargo test --manifest-path web/Cargo.toml --locked --features page --bin boardstudio-web parts_new_component -- --nocapture` — PASS, 4 focused tests. They cover exact default fields/document preservation, stale and duplicate-ID rejection, identity collision avoidance, accepted Session edit/Undo/Redo, and suppression for changed scope, workspace or Parts intent generation.
- `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings` — PASS.
- `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web` — PASS.
- `git diff --check` — PASS.

These source checks do not close the ticket's paired React/Dioxus browser journey, visible action styling, creation-and-rename workflow, save/reopen, delayed mounted-runtime outcomes, or F4.2/INT.2/F9 joins. No public acceptance is claimed here. Preserve the canonical 62-parent graph and all its joins.

## Refactoring handoff

No new refactoring takeaway was observed in the reviewed Parts13 private leaf and mount patch. Retain the existing RF-001 shared-composition integration hotspot and RF-009 source/evidence accounting context. The machine register and readable post-port handoff point here.
