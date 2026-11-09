# 27: Verify the complete cutover and release the integration gate

Status: resolved
Type: build
Blocked by: [26](26-rebuild-examples-and-fixtures.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns only the end-to-end acceptance checklist and release gate; fixture regeneration and cutover gates are left to [17e](17e-fixtures-and-final-gate.md) and the example rebuilds to [26](26-rebuild-examples-and-fixtures.md).

## Outcome

Prove the whole accepted scope through production paths and permit promotion of the
version-7 implementation only after the coordinated lane is complete. Planning map
closure is not product completion, and aggregate anchors remain explicitly deferred.

## First RED / GREEN slice

Start with an end-to-end traced case from public edit/preview through materialization,
scene, saved archive and reopen, including an anchored mirrored group. If it exposes
a gap, add the smallest behavioral regression at the relevant agreed seam and fix
that path before taking another scenario. Do not replace real routes with test doubles.

## Release acceptance

- Account for every decision/spec scenario and every preserved dependent contract;
  check primitive golden parity and real prepared edit/lifecycle coverage separately.
- Verify strict v7 gates at all entries, no old-data writes/migrations/wipes, stale-tab
  and storage-failure safeguards, raw-data truthfulness and history/draft lifecycle.
- Verify curved/rotated/anchored/mirrored placement, origin compensation, Local/Omitted
  graph/resize/removal/confirmation, constraints/companions, column Align and key sizes.
- Verify source/receiver reference highlighting versus amber actual preview, hidden
  contributors, keyboard routes, real trusted drags, cancellation and exact Undo.
- Verify the three editable examples, member/scan/keymap identity, scene/materialized
  consistency and affected outline/PCB/CAD/export consumers. No fabrication claim.
- Remove staging leftovers, old vector/formula readers and legacy oracle activation;
  retain deliberate negative fixtures and unrelated behavior. Do not merge prototype
  code, flags, duplicate geometry or bulk-regenerated unrelated snapshots.
- Run one combined spec/standards closure review of the actual cross-crate change.
  Route concrete findings to their owning build tickets, not silent spec relaxation.

## Checks and evidence

Run `python3 scripts/check.py` for the full pre-PR scope, including its checked build,
native/contract/provider and mounted browser coverage. Use README prerequisites and
the scheduler; native-only or `browser --only` evidence is partial. Collect the
supported check receipt under `.evidence/`, observed test counts/names, trusted-input
engine and any failures/unrun environments. Never claim planned checks passed. Hardware
verification remains downstream; unresolved required acceptance blocks this ticket.

## Evidence

Per-bullet proving-test mapping for the Release acceptance above. Every name
below was verified to exist as a `#[test]` / `#[wasm_bindgen_test]` function
in the cited file on this branch. Correction to the worker report: the names
`durable_session` and `demo_projects` exist neither as test files nor as test
functions (grep finds only the `pub(crate) mod demo_projects;` module
declaration in `web/crates/library/src/lib.rs`). The real suites are cited
instead: durable behavior under its actual test names in
`core/src/mirrored_half/lifecycle/tests.rs`,
`core/src/mirrored_half/reconcile/tests.rs` and `core/tests/core.rs`; demos
via `demo_rebuild_tests` in `web/crates/library/src/demo_projects.rs`.

- Decisions/spec scenarios, dependent contracts, golden parity, prepared
  edit/lifecycle coverage: `core/tests/boundary_parity.rs`
  (`fixture_hashes_match_their_bytes`,
  `native_transport_reproduces_the_golden_transcript`),
  `core/tests/contracts.rs`
  (`project_document_serialization_keeps_format_and_numeric_revision`),
  `core/tests/generator_export.rs`
  (`every_bundled_generator_exports_a_board_kicad_parses`),
  `core/tests/matrix_placement.rs`
  (`placement_survives_save_and_reopen_unnormalized`).
- Strict v7 gates, no old-data writes/migrations/wipes, stale-tab and
  storage-failure safeguards, raw-data truthfulness, history/draft
  lifecycle: `core/tests/current_schema.rs`
  (`open_refuses_documents_that_are_not_at_the_current_version`),
  `core/tests/legacy_archive_migration.rs`
  (`an_unmigrated_document_is_refused_by_open`,
  `an_unversioned_archive_is_refused_without_releasing_assets`),
  `core/tests/document_integrity.rs`
  (`refused_open_preserves_nonempty_history`,
  `malformed_missing_and_foreign_format_inputs_refuse_before_any_state_change`,
  `max_revision_undo_and_redo_are_refused_with_their_history_intact`),
  `core/tests/core.rs` (`preview_does_not_commit_and_stale_edit_fails`,
  `undo_redo_increments_revision`),
  `web/crates/host/src/host/storage_lifetime_tests.rs`
  (`stale_tab_save_preserves_the_other_tabs_document`,
  `#[wasm_bindgen_test]`). Durable-session behavior:
  `core/src/mirrored_half/lifecycle/tests.rs`
  (`valid_durable_omission_accepts_an_absent_mirror_part`),
  `core/src/mirrored_half/reconcile/tests.rs`
  (`durable_local_exception_is_atomic_and_repeats_through_parent_kernel`,
  `durable_omitted_independent_part_stays_absent_through_parent_kernel`).
  Entry-path gate detail is owned by [17d](17d-entry-path-gates.md)
  (resolved) and format recovery by [25](25-project-format-recovery.md)
  (resolved); this ticket cites their evidence instead of duplicating it.
- Curved/rotated/anchored/mirrored placement, origin compensation,
  Local/Omitted graph/resize/removal/confirmation, constraints/companions,
  column Align and key sizes: `core/tests/matrix_anchors.rs`
  (`anchor_follow_covers_every_subset_with_zeroing`,
  `anchor_follow_applies_origin_and_rotation_before_filtering`,
  `anchored_matrix_translation_respects_follow_and_refuses_impossible_drags`,
  `change_anchor_retargets_dependants_with_preview_cancel_and_undo`),
  `core/tests/mirrored_half.rs`
  (`a_rotated_pair_map_reflects_keys_companions_and_other_parts_by_the_pair_rule`,
  `one_undo_step_removes_the_half_and_redo_restores_it`),
  `core/tests/key_move_assemblies.rs`
  (`matrix_key_move_respects_rotated_splayed_and_mirrored_cell_coordinates`,
  `moving_matrix_key_moves_its_diode_and_led`),
  `core/tests/matrix_placement.rs`
  (`curved_key_world_move_changes_only_its_independent_offset`,
  `companions_follow_placement_changes`),
  `core/tests/document_integrity.rs`
  (`omitted_matrix_membership_keeps_a_live_anchor_without_materialized_parts`).
  Anchor-following semantics are owned by [19](19-anchor-layout-groups.md)
  (resolved); this ticket verifies them only through the in-ticket traced
  case below.
- Source/receiver highlighting versus amber actual preview, hidden
  contributors, keyboard routes, real trusted drags, cancellation and exact
  Undo: owned by [24](24-relations-and-highlights.md) (resolved at
  `1ccce1ed2`, Spec and Standards reviews PASSED, native baseline
  `eaa1b392f` +9 names) and [23](23-inspector-toolbar-and-handles.md)
  (resolved at `34b8045ec`, re-reviews PASSED, native baseline `47820fc85`
  +13 names); this ticket cites their evidence instead of duplicating it.
  Concrete pins: `web/crates/layout/src/objects/layout_transform_toolbar.rs`
  (`mounted_pointer_dispatch_is_synthetic_never_trusted`,
  `#[wasm_bindgen_test]`, landed in `23d22080c`;
  `direct_selection_choices_open_scope_panels_and_support_keyboard`,
  `#[wasm_bindgen_test]`),
  `core/tests/matrix_placement.rs` (`preview_and_undo`), and the
  preview-cancel demo test cited in the next bullet.
- Three editable examples, member/scan/keymap identity,
  scene/materialized consistency, outline/PCB/CAD/export consumers: owned
  by [26](26-rebuild-examples-and-fixtures.md) (resolved; native baseline
  `818adfd32` +11 names, 0 removals); this ticket cites its evidence.
  The real suite is `demo_rebuild_tests` in
  `web/crates/library/src/demo_projects.rs`:
  `studio60_is_a_core_generated_v7_matrix_document`,
  `studio60_copies_do_not_share_mutable_state`,
  `macro12_uses_the_single_matrix_recipe_at_3x4`,
  `split42_has_two_linked_layouts_with_four_matrices`,
  `split42_keeps_approved_poses_with_matrix_identities`,
  `split42_anchor_edit_moves_thumb_and_counterpart_with_one_undo`,
  `split42_preview_cancel_leaves_the_document_unchanged`,
  `rebuilt_examples_share_pitch_gap_caps_and_board_intent`.
  Consumer pins: `core/tests/outlines.rs`
  (`automatic_part_envelopes_exclude_nonphysical_utilities`),
  `core/tests/core.rs`
  (`pcb_readiness_checks_pads_outlines_and_thickness`),
  `core/tests/generator_export.rs`
  (`every_bundled_generator_exports_a_board_kicad_parses`). No fabrication
  claim: all three examples are Core-generated through production routes.
- Staging leftovers, old readers, oracle activation, negative fixtures,
  no prototype merge: deliberate refusal fixtures retained and green —
  `core/tests/legacy_archive_migration.rs`
  (`an_unversioned_archive_is_refused_without_releasing_assets`,
  `an_unmigrated_document_is_refused_by_open`),
  `core/tests/current_schema.rs`
  (`rejects_obsolete_single_model_field`). `git status` on this branch
  shows no staged leftovers; `web/crates/layout/prototype/` remains
  untouched and untracked. This ticket's code change is exactly one
  production-path fix (`core/src/lib.rs`, preview_edit through
  layout resolve) plus its in-ticket test — no flags, duplicate geometry,
  or bulk-regenerated snapshots.
- Combined closure review and finding routing: the combined spec/standards
  closure review PASSED the code across the four in-ticket commits
  (`437f31478` claim, `0390a0f82` RED, `c3fa78236` fix, `052c8fc96`
  style). Finding disposition is recorded in the next section; every
  routed claim cites its owning ticket.

In-ticket traced case (First RED / GREEN slice): `core/tests/end_to_end_acceptance.rs`
(`preview_commit_scene_archive_reopen_and_undo_carry_the_anchored_mirrored_group`,
added in `0390a0f82`) traces public edit/preview through materialization,
scene, saved archive and reopen, including an anchored mirrored group, with
Undo carrying the group. The stale-Thumb gap it exposed was fixed in-ticket
by `c3fa78236` (preview_edit routed through layout resolve) with no test
doubles.

## Finding disposition

No open findings remain. The combined closure review surfaced one genuine
gap — preview showed the anchor key moved while its anchored Thumb stayed
stale (commit moved both via `layout::resolve`) — and it was fixed in-ticket:
RED test `0390a0f82`, production-path fix `c3fa78236`, style `052c8fc96`,
with the traced test above green. All other review observations route to
their owning tickets, not to silent relaxation here: anchor-following
semantics are owned by [19](19-anchor-layout-groups.md) (Status: resolved)
and preview/handles/inspector behavior by
[23](23-inspector-toolbar-and-handles.md) (Status: resolved). No routing
claim in this ticket is left uncited.

## Receipt limitations

From `.evidence/ergogen-27-acceptance.json.md` (55/55 commands, exit 0;
native 2249 passed / 0 failed; page suite 88/88):

- The page suite covers only `web/src`; it is not the whole browser gate.
  This does not block this ticket's acceptance: the receipt's Rust-side
  evidence is complete for the scope it covers (native suite plus the
  listed wasm-crate launches, including layout mounted/browser groups),
  and the `web/src` page suite ran green (88 passed, 0 failed) for the
  scope it covers. Browser coverage beyond `web/src` remains downstream
  product verification, which this ticket explicitly leaves downstream.
- No base-revision run evidence was supplied (Before/after: unknown). This
  does not block this ticket's acceptance: no before/after regression
  claim is made. Acceptance rests on absolute gates at the claimed
  revision plus fail-first evidence in-ticket — the RED test `0390a0f82`
  demonstrated the gap before the fix `c3fa78236` turned it green —
  rather than on a comparison against a base revision run.

## Answer

Resolved on branch `docs/ergogen-layout-comparison` at `8c742aa75`. The
traced case `end_to_end_acceptance::
preview_commit_scene_archive_reopen_and_undo_carry_the_anchored_mirrored_group`
(RED `0390a0f82`) walks public edit/preview through materialization, scene,
saved archive and reopen, including an anchored mirrored group, with Undo
carrying the group. Its stale-Thumb gap was fixed by routing preview edits
through layout resolve (`c3fa78236`, one production-path line in
`core/src/lib.rs`; `052c8fc96` style). The combined spec/standards closure
review PASSED the code; the gap re-review PASSED the doc fix `8c742aa75`
(evidence mapping, finding disposition, receipt limits). No open findings:
anchor-following semantics stay owned by [19](19-anchor-layout-groups.md)
and preview/handles/inspector behavior by
[23](23-inspector-toolbar-and-handles.md), both resolved. Native baseline
`4c205050` (+1 name, 0 removals). Fresh gates green — quick: `check: all
steps passed: repo, tooling, lint, typecheck, test, browser` (227.9s);
full: `check: all steps passed: repo, tooling, lint, build, test, browser`
(302.3s).

No release approval is implied.
