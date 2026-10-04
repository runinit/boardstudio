# Consolidated review — Opus 5.5 (route claude, role sol-review), 2026-10-04

Reviewer: independent Opus 5.5 high agent, read-only; not the author. Range: `f709ab60^..HEAD` (core, web, scripts) plus the Case and key-Inspector receipts.
Checks run by the reviewer: `matrix_transform_operation` 11 passed; `cad_jobs` 6; `case_generation_admission` 1; core `--test core case` 4.

## Verdicts
- F8.2-C01 (verified): ACCEPT. `authored_case_geometry_ready` (core/src/lib.rs:76) keeps scene-revision = document-revision, board, solid outline, Case body, and no Case error other than `case:<body>:material`;
  `preparation_request` still checks session epoch/document/revision (cad_jobs.rs:768-790); mechanical admission still requires `instance_selection.is_current`; `export_step` still requires `active_board_id == scope.board_id`.
  No path admits a Case for a stale board, instance or revision.
- F7.6-C01 (implemented): ACCEPT as implemented, not verified (cancel/retry not re-exercised on 34814).
- Parent F8.2: ACCEPT (C02-C04 untouched by the range; follow-up defect 4).
- F3.5-C05 (was verified): HOLD. Mirror-target override not implemented; defects 1-3 are real.

## Confirmed defects
1. `set_cell_attached` (matrix_transform_operation.rs) always sets `assemblies_local = Some(true)`; reference only does so for a linked target (Workbench.tsx ~793-805; part_placement.rs ~1871-1881). Core `reflected()` (core/src/matrix/layout.rs ~176-177) skips propagation for local cells, so an Attached edit on the canonical half no longer reaches the mirrored half. Reproduced against Core: remove on canonical with the flag -> left=[] right=["switch"]; reference way -> both [].
2. `set_cell_assembly` never sets `assemblies_local` on a linked target; a Key Assembly choice on the right half rewrites the left half (reproduced: left=Some("alt") without the flag).
3. matrix_transform_controller.rs loads the catalogue with `reversible=false`; Parts uses `reversible_layout(document)`. In a reversible document the non-reversible build is attached under the shared id and a later Parts placement fails with `Conflicting definition` (core/src/lib.rs ~1276-1279).
4. `default_case_body_does_not_reference_an_absent_material` (case_controller.rs ~732) never runs: `mod presentation` is wasm-only and the test is a plain `#[test]` (`cargo test --bin boardstudio-web default_case_body` ran 0 tests).

## Lower-confidence
- case_generation_admission.rs:104-107 asserts `is_ready(..., configured=true)` for the `case_ready` scene regardless of that scene (vacuous).
- Material exclusion matches `finding.id.ends_with(":material")`; only validate.rs:477 produces it today.
- Attached choices skip the reference `partChoices` exclusions (nice_nano_pretty, assembly snapshots); neither path checks kind.
- Missing-cell attached edit is safe (no row renders).
- Held up: stale/double-submit guards (owner/generation, monotonic request id, pending rejection, token/revision admission, full-value baseline), Core validation of new fields, Enabled on a target propagating like the reference; the 11 operation tests assert real state but only cover `build_operation`.
