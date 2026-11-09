# 179: Replace remaining fixed timer sleeps and trim duplicate browser tests in PCB, Parts, Case, Runtime, and Host

Status: resolved
Type: build
Blocked by: None (can start immediately)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

Browser and native tests in `pcb`, `parts`, `case`, `runtime`, `host`, and `catalogue` wait on bounded state predicates instead of fixed `TimeoutFuture::new(80)` loops, spawn only the Web Workers they use, and avoid running duplicate `RendererHost` or test-scaffolding suites.

## Acceptance criteria

- [x] Replace the fixed `TimeoutFuture::new(80)` loops in `web/crates/pcb/src/pcb_module_inspector.rs` (`mounted_save_replacement_tests`) with bounded `test_wait` predicates (`wait_until` / `wait_until_step`), shorten the 58 `TimeoutFuture::new(80)` calls in `web/crates/parts/src/parts_definition_name.rs` to 5 ms yields and the 3 `settle_fields` (`15 x 40ms`) loops in `web/crates/case/src/mechanical_settings.rs` to 2-iteration drains, and centralize the `queued_edit_tests.rs` `document()` / `test_part()` / `test_definition()` fixture in `pcb_wiring.rs::test_fixtures` (the native `mode_owner_tests.rs` copy compiles under `cfg(not(wasm32))`; see Answer).
- [x] Merge or remove the duplicate tests identified in `parts`, `case`, and `catalogue`: `mounted_definition_owner_switch_retires_dirty_courtyard_pad_and_error_state` and the duplicate baseline-return tests in `web/crates/parts/src/parts_definition_name.rs`, `uploaded_model_projects_accepted_filename_without_erasing_dirty_width` in `web/crates/parts/src/parts/generator_settings.rs`, `add_menu_search_excludes_retired_and_assembly_snapshot_definitions` in `web/crates/parts/src/parts.rs`, `assigned_profile_exposes_case_local_remove_control`, `process_thickness_draft_escape_enter_and_blur_follow_numeric_field_semantics`, and `unchanged_failed_mechanical_fields_restore_accepted_text_and_report_inline` / `older_mechanical_failures_report_beside_newer_drafts_and_original_accepted_text` in `web/crates/case/src/mechanical_settings.rs`, `a_legacy_halves_shared_field_edit_reads_back` in `web/crates/case/src/mechanical_session_tests.rs`, and `same_flip_reused_pcb_halves_have_no_representable_pair_map`, `reused_pcb_opposite_flip_halves_propose_the_physical_reflection_and_shared_correspondences`, and `ineligible_targets_propose_no_mechanical_pair` / `an_empty_pair_id_is_a_malformed_link_not_an_ineligible_target` in `web/crates/catalogue/src/physical_setup.rs`.
- [x] In `web/crates/runtime/src/runtime_collision_browser_tests.rs`, split document/scene setup from worker creation so `vendor_refinement_fixture` and `complete_check_fixture` do not spawn and immediately close a throwaway Web Worker, and remove the two subsumed tests (`mechanical_collision_flow_analyzes_physical_pcb_and_reports_case_pair_provenance` and `tight_vendor_refinement_shares_one_preparation_without_merging_placements`). The two remaining mock workers keep separate protocol scripts because their message shapes differ.
- [x] Remove the 4 `gate_next_core_reply_then_fail` test-scaffold meta-tests in `web/crates/runtime/src/runtime.rs` (`in_process_adapter_tests`). `renderer_host_page.rs` / `renderer_host_page_base.rs` are intentionally retained unchanged because `renderer_host_source_sync` asserts byte-for-byte normalized equality against `host/src/renderer_host.rs`.
- [x] Affected native and browser crate tests (`pcb` 15.49 s → 7.44 s, `parts` 12.11 s → 3.31 s, `case` 10.95 s → 9.07 s, `runtime` 4.70 s, `host` 5.62 s, `catalogue` 0.25 s) pass in the full `check.py` run.

## Answer — 2026-10-19

Implemented on `dev` without touching `host`, `renderer_host_page.rs` or `renderer_host_page_base.rs`.

- `pcb_module_inspector.rs`: every fixed `12 x 80 ms` / `8 x 80 ms` drain in `mounted_save_replacement_tests` now polls `support::run_pending` + its real predicate with `wait_until_step` (5 ms poll). The 100 x 10 ms IndexedDB predicate is unchanged.
- `parts_definition_name.rs`: `settle()` is a 5 ms yield instead of 80 ms; the duplicate `mounted_definition_owner_switch_retires_dirty_courtyard_pad_and_error_state` is deleted, and the pad-number and name baseline-return scenarios are folded into `mounted_landed_courtyard_keeps_a_newer_draft_then_clean_undo_redo_syncs`.
- `generator_settings.rs`, `parts.rs`: the two redundant prefix/duplicate tests are deleted.
- `mechanical_settings.rs`: the assigned-profile page/test, the `ProcessThicknessTestPage` and its numeric-field semantics test, and the two failure-restore tests are deleted; `settle_fields` is a 2-iteration drain instead of `15 x 40 ms`.
- `mechanical_session_tests.rs`: the standalone legacy shared-field test is deleted and its read-back assertion (`visible("third").unwrap().wall_thickness == 4.0`) is added to `converting_a_legacy_halves_construction_materializes_it_without_moving_anything`.
- `physical_setup.rs`: `_stable_signature` is deleted; the proposal-only assertions (flips, construction source, mapping round-trip, correspondences, input immutability) are folded into `an_eligible_setup_proposal_establishes_its_pair_on_the_proposed_document`; the same-flip, ineligible-target and empty-pair-id duplicates are deleted.
- `runtime_collision_browser_tests.rs`: `fixture_document_scene` owns document/scene setup; `vendor_refinement_fixture` and `complete_check_fixture` call it directly instead of spawning and closing a throwaway worker. The two subsumed tests are deleted.
- `runtime.rs`: the 4 `gate_next_core_reply_then_fail` meta-tests are deleted; the helper remains because `keycaps` uses it.

### Fixture-sharing constraint

`mode_owner_tests.rs` is compiled only natively (`lib.rs` inline `pcb_wiring` module, `cfg(not(wasm32))`) and `queued_edit_tests.rs` only for wasm (`pcb_wiring.rs`). A single Rust item cannot be shared between them without a new shared module outside this ticket's file scope or a `lib.rs` edit, so the wasm copy moved to `pcb_wiring.rs::test_fixtures` (consumed by `queued_edit_tests.rs`) and the native copy stays in `mode_owner_tests.rs`. Both contain the same base document; the wasm document adds the encoder/connector/net tail.

### Verification

- `cargo test -p boardstudio-web-pcb -p boardstudio-web-parts -p boardstudio-web-case -p boardstudio-web-catalogue -p boardstudio-web-runtime --lib`: exit 0; 34 + 20 + 42 + 23 + 125 tests passed, 0 failed
  (`.evidence/ticket-179-native.log`).
- Canonical repo lint (native `cargo clippy --workspace --locked --all-targets --no-deps -- -D warnings`, plus the runtime-only wasm clippy): both exit 0 (`.evidence/ticket-179-clippy-native.log`, `.evidence/ticket-179-clippy-runtime-wasm.log`).
- Wasm compile check for the browser test targets: `cargo check -p boardstudio-web-pcb -p boardstudio-web-parts -p boardstudio-web-case -p boardstudio-web-catalogue --target wasm32-unknown-unknown --all-targets`: exit 0 (`.evidence/ticket-179-wasm-check.log`).
- The requested single command `cargo clippy -p ... --target wasm32-unknown-unknown --all-targets --no-deps -- -D warnings` still exits 101 (`.evidence/ticket-179-clippy-requested.log`): 71 lib + 75 lib-test diagnostics, all pre-existing wasm-target lints in untouched `parts` files (`assembly_editor.rs`, `component_model_editor.rs`, `module_profile_editor.rs`, `preview.rs`, `generator_settings.rs:759/1566`, `parts.rs:251`, `parts_definition_name.rs:43`, ...). No diagnostic points at an edited region; the same wasm config is not part of the repo's canonical lint set (which lints only `runtime` for wasm).
- Browser suites were not executed here; their targets were compile-checked on wasm and the native suites above pass. `renderer_host_source_sync::private_page_host_matches_normalized_library_host` passes.
