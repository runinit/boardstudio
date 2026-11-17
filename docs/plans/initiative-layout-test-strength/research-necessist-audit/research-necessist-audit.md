---
type: research
title: "Necessist audit of Layout engine and PCB layout tests"
created: 2026-11-15
commit: 14c276248bb8064c8f5d27760b5c33ac7ce63b53
---

# Necessist audit of Layout engine and PCB layout tests

Observations from one Necessist 4.1.0 run (Rust framework, default settings) at the commit above, on branch `audit/layout-engine-pcb-layout`, and from an independent read-only review by GPT-6 Astra (xhigh). A removal that "passed" means the test still passes with that statement deleted. Raw results: `necessist.db` in the worktree root (untracked); logs in `.evidence/necessist-audit/`.

## Scope and result

- Audited: `core/src/matrix.rs`, `core/src/matrix/{layout,kernel,splay,deletion}.rs`, `core/src/geometry.rs`, `core/src/constraints.rs`, `core/src/constraints/{endpoints,relation,retune,framed_tests}.rs`, `core/src/outline_edit.rs`, `core/src/artifact/outline.rs`, `application/tests/drag_snap.rs`, `web/crates/runtime/src/layout_viewer_source.rs`, native tests under `web/crates/layout/src`.
- 1,045 removals: 176 passed, 239 failed, 630 did not build. All 176 passing spans match the commit (none stale). Each removal ran once.
- No production defect was established.
- Not reachable as laid out (0 candidates): every `core/tests/*.rs` module, because `core/tests/integration.rs` links them into one `integration` binary through `#[path]` modules with `autotests = false`, and every wasm-only test.

## Findings

Each is a test that cannot detect the regression its name describes. Line numbers are at the commit above.

- **F1** `core/src/matrix/layout.rs:4794` `case_clearance_holes_are_not_adopted_by_linked_layouts`. The fixture has no key parts, so `adopt_unassigned_components` returns at `layout.rs:3581` before the `case-closure/` filter at `layout.rs:3609`. Removing the call (4800) passes.
- **F2** `core/src/constraints/framed_tests.rs:687` `persisted_framed_constraints_solve_through_the_stored_document_path`. The fixture's `alpha/T` pose (line 27) equals the pose asserted at 698. Removing the constraint push (689) or `resolve` (696) passes.
- **F3** `core/src/matrix.rs:1160` `projection_rejects_invalid_draft_geometry`, second block (1183–1200). `column_layout` is left one record too long by the first block and is rejected at `core/src/matrix/kernel.rs:521`, before the finite-origin check at `kernel.rs:547`. Removing the NaN assignment (1197) passes.
- **F4** `web/crates/runtime/src/layout_viewer_source.rs:1013`. The duplicate snapshot is a new document allocation, so `part_for_current_pick` returns `None` at the owner-currency check (`matches_current`, reached from 491–493) before duplicate-reference resolution. Removing either duplicate-setup statement (1014, 1019) passes. Duplicate rejection itself has a direct test at `web/crates/runtime/src/case_preview.rs:315`.
- **F5** `core/src/constraints/framed_tests.rs:460` `a_local_mirror_is_an_ordinary_part_endpoint`. The fixture's `beta/T` pose (line 28) equals the pose asserted at 469. Removing `resolve_prepared` (468) passes; the `.unwrap()` still proves the record is accepted.
- **F6** `application/tests/drag_snap.rs:49` `gap_snap_preserves_edge_alignment_for_rotated_envelopes`. The fixture keycap is 18×18, so 0° and 90° give the same envelope. Removing the rotation assignment (59) passes.
- **F7** `web/crates/layout/src/matrix_transform_lifecycle.rs:252` `accepted_membership_reuses_preview_notifications_and_retires_on_source_change`. `matrix_fixture()` (218) has no matrix and no parts, so membership is empty throughout; removing `model.accepted = None` (325) passes, and the reopen at 330 uses a different owner. Found by the review; the fixture was re-read and agrees.

## Lead

- **L1** `web/crates/layout/src/objects/keycap_resize.rs:1408` `replace_document_reflows_source_and_core_syncs_linked_partner_through_history` is `#[ignore]` and reads `KEYCAPS_MIRROR_PROJECT_JSON`, which nothing in the repository sets. Its 11 passing removals are artifacts of a test that did not run. The ignore reason and doc comment name an extracted `project.json` from the pinned Sofle archive.

## Harness limitation

- **H1** Necessist resolves a file under `tests/` to a test target only when a `[[test]]` target's path is that file; otherwise it lists `--lib` tests and drops the module prefix. No `necessist.toml` option changes this. The 14 matrix-placement, anchor and outline files under `core/tests/` (207 tests) import only `boardstudio_core`, serde, serde_json, sha2 and fixtures under `core/tests/fixtures`, so each could compile as its own target. Untried routes: a scratch crate outside the repository with symlinked test files, or temporary `[[test]]` entries in `core/Cargo.toml`.

## Review corrections

- **C1** Necessist measures sensitivity to deleting test statements, not production mutations. A clean run does not establish that code is safe to restructure.
- **C2** The yield and cost of an integration-test audit are not calibrated. The only timing sample is 26 removals in 145 s on one library file; 630 of 1,045 removals did not build.
- **C3** A repaired test that then fails does not by itself prove a product defect.
- **C4** Results from one-target-per-file builds must be reproduced in the normal `integration` target before they are accepted; ignored tests and assertion-helper calls must be excluded, and the named test must be shown to execute, including by a deliberate failing control that the audit detects.
- **C5** Neither the audit nor the review justifies a production refactor.

## Proposal recorded, not decided

- **P1** Replace the repeated resize/reset/zip fixture block in `core/src/matrix/splay.rs`, `core/src/matrix.rs` and `web/crates/layout/src/matrix_transform_operation.rs` with one helper per test module.
