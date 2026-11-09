# 178: Trim redundant CreateMirroredHalf, ReplaceDocument, and pin-capacity loops in Core integration tests

Status: resolved
Type: build
Blocked by: None (can start immediately)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

Core's `integration` test binary verifies mirrored-half creation, matrix previews, companion fallbacks, electrical pin capacity, and Undo history limits without rebuilding unchanged pair fixtures dozens of times inside loops or duplicating `matrix_acceptance` and `parent_acceptance` cases.

## Acceptance criteria

- [x] Removed the duplicated bottom-of-file matrix preview tests in `core/tests/mirrored_half.rs` (the `production_*` preview block, which in this revision is `production_paired_previews_project_all_matrix_edit_families_without_publishing` plus the four `production_authored_companion_*` preview tests and their `assert_authored_companion_preview` helper) that `core/tests/mirrored_half/matrix_acceptance.rs` covers. Note: the named `production_authored_companion_families_follow_*` / `..._rejoin_restores_*` tests and the `paired_previews_project_all_six_...` / `two_matrix_pair_engine` case do not exist in this revision (already gone).
- [x] In `core/tests/mirrored_half.rs`, all three stabilisers (`stab-0`, `stab-90`, `stab-180`) are placed on one board in `stabilisers_keep_their_source_heading_and_mirrored_position_in_every_mode` (6 `CreateMirroredHalf` cycles instead of 18), the missing diode and LED companions are placed in one pass in `missing_source_companions_materialise_with_a_literal_fallback_in_modes_two_and_three` (4 cycles instead of 8), `unibody_label_seed()` is hoisted outside the refusal loops in `invalid_creation_labels_are_located_and_refused_without_mutation`, and `build()` is called once in `a_mirrored_circuit_reports_the_findings_of_its_source_and_no_reference_repeats`.
- [x] Removed the duplicate `parent19_independent_screen_and_encoder_exceptions_survive_edit_reopen_and_rejoin` test (and its now-unused `accepted_pair_with_independent_controls` helper) in `core/tests/mirrored_half/parent_acceptance.rs`, and removed the redundant `assert_duplicated_source_half_is_coherent` / `assert_unique_ids` test-helper check in `core/tests/mirrored_half/open_acceptance.rs`.
- [x] `resolver_capacity_refuses_replacement_placement_and_rejoin_without_publication` (`core/tests/mirrored_half/replacement_pins/capacity_rejoin.rs`) now exhausts the controller's spare pins once and runs both the placement and Rejoin refusal assertions against that saturated state (one 16-iteration loop instead of two).
- [x] In `core/tests/core.rs`, `public_native_pair()` and the seed `AddMirrorOnlyPart` commit are hoisted outside the 5-collision refusal loop in `mirror_only_part_refuses_ids_owned_by_other_document_entities_atomically` (one seeded engine; a single Undo after the loop proves the entry survived), and `undo_retains_only_the_latest_hundred_commits` commits a lightweight `EditOperation::MoveParts` on a one-part minimal document instead of 101 `ReplaceDocument` clones.
- [x] `cargo test -p boardstudio_core --test integration` passes; timings recorded below.

## Results

- `cargo test -p boardstudio_core --test integration mirrored_half` → **112 passed; 0 failed** (test time 6.01s).
- `cargo test -p boardstudio_core --test integration core_engine` → **67 passed; 0 failed** (test time 0.08s).
- Full `cargo test -p boardstudio_core --test integration` (with AppImage `APPDIR` cleared so `kicad-cli` loads `/usr/bin/_pcbnew.kiface`) → **682 passed; 0 failed** in **14.22s** (down from 15.33s).
- Six tests were removed (5 from `mirrored_half.rs`, 1 from `parent_acceptance.rs`); the `CreateMirroredHalf` cycles in the trimmed loops dropped from 18→6 and 8→4, and the two 16-pin loops merged to one.
