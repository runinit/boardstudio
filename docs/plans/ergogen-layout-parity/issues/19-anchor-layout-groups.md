# 19: Evaluate multi-matrix Layouts and maintain single-key anchors

Status: resolved
Type: build
Blocked by: [18](18-core-placement-edits.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns the Core edit for aligning through anchors; the Setup, Align and selection consumer wiring that calls it is left to [22](22-update-layout-consumers.md).

## Outcome

Complete anchor graph evaluation and Core-owned create/change/detach behavior. A
Layout is one board-local group, not an array-order parent tree. No aggregate arm.

## First RED / GREEN slice

Fail an actual Core edit creating a 1×3 anchored matrix in its selected key's Layout;
move that key and assert dependent member poses. Add only the graph/evaluation needed
to pass, then test reordered matrix lists through the same public route.

## Subsequent acceptance

- Evaluate dependency order, actual final key frames, Origin/Rotation then Follow;
  cover every subset, zeroing, rotated frames and constrained matrix drag directions.
- Add/change/detach are one typed Core transaction/Undo, not several UI requests.
  Default Add is Origin `(0,-pitch.y)`, all Follow components, 1×3 and same Layout.
- Refuse self/cycles, cross-Layout/board targets, missing targets and constraint-driven
  anchor keys; refuse constraining an anchor key. Solve constraints after anchors.
- Logical disabled/deleted keys and Omitted matrices/keys keep evaluating before
  visible output filtering. Do not materialize missing geometry for anchors.
- Compaction remaps physical references; actual shrink/removal detaches dependants
  and keeps frame, never targets a new key at an old coordinate. Deliver the group-level form of the
  confirmed asymmetric linked policy here (20–21 consume it later), not a silent fallback.
- Extend placement/Align through actual anchor frames and Follow representability;
  exact impossible edits refuse; constrained drags/Rotate availability remain truthful.

## Checks

Source seams: Core matrix layout/validation/deletion and constraint preparation,
public Open/ResolveEdit/scene interfaces and Session preview/history. Cover valid and
refused graphs with actual materialized poses and cancellation, not graph helper
success alone. Run focused Core/Session tests and quick checks for changed crates.

## Progress

Implementation lane on branch `docs/ergogen-layout-comparison`, base `2533a634a`.
Status remains claimed; no resolve, no push. `web/crates/layout/prototype/` untouched.
Production code: `core/src/matrix/anchors.rs` (new: typed Add/change/Detach,
graph validation, compaction remap/shift, removal detach, anchor-key constraint
and unfollowed-frame refusals), plus arms in `core/src/lib.rs`, hooks in
`core/src/matrix/deletion.rs`, mirrored-half inverse coverage in
`core/src/mirrored_half/inverse.rs` + `hosted.rs`, and three new `EditOperation`
variants in `core/src/model.rs` (ticket requires typed Add/change/Detach intents).

Commits (`git log` SHAs):

- `0467e4ba0` claim.
- `b2a784b8a` typed Add anchored matrix with dependency-order evaluation.
- `395951a46` anchor change/detach with preview, cancel and Undo.
- `5655bd868` anchor graph refusals leave state unchanged.
- `d2bd33390` Follow subsets, ordering and Follow-aware refusals.
- `fbada80cd` compaction, removal, invisible keys and constraint order.
- `d1bb1104a` existing-test contract update: shrink detaches dependants with
  frame instead of refusing (ticket 19 supersedes the old refusal; test name
  kept so the native baseline is unaffected).
- `117720c0c` repo rustfmt.

Proving tests per acceptance bullet (all in `core/tests/matrix_anchors.rs`, via
public `Open`/`Edit` routes; 19 new names for the integrator baseline):

- Add 1x3 in the anchor key's Layout + move-key follow + list-order
  independence: `add_anchored_matrix_creates_1x3_in_same_layout_and_follows_its_anchor_key`,
  `reordered_matrix_lists_evaluate_to_the_same_poses`.
- One typed transaction/Undo with defaults (1x3, Origin `(0,-pitch.y)`, all
  Follow, same Layout), change with preview/cancel/Undo, detach keeping the
  world frame: the Add test above, `change_anchor_retargets_dependants_with_preview_cancel_and_undo`,
  `detach_keeps_world_frame_and_clears_the_anchor`.
- Refusals (self, cycle, missing, out-of-range, cross-Layout, cross-board,
  constraint-driven anchor, constraining an anchor key, duplicate/bad id,
  detach-without-anchor, unknown matrix), each atomic:
  `anchor_graph_refusals_leave_state_unchanged`,
  `anchoring_to_a_constraint_driven_key_is_refused`.
  Linked-Layout Add is explicitly refused naming ticket 20 (mirrored creation
  belongs to 20): `adding_to_a_linked_layout_is_refused_until_ticket_20`.
- Dependency order, actual final anchor-key frames, Origin/Rotation-then-Follow,
  every Follow subset, zeroing, rotated frames, constrained drag directions:
  `reordered_matrix_lists_evaluate_to_the_same_poses`,
  `anchor_follow_covers_every_subset_with_zeroing`,
  `anchor_follow_applies_origin_and_rotation_before_filtering`,
  `anchor_follow_rotation_filters_the_completed_heading`,
  `anchored_matrix_translation_respects_follow_and_refuses_impossible_drags`,
  `column_align_moves_through_the_actual_anchor_frame`.
- Disabled/deleted keys keep anchoring without materializing geometry:
  `disabled_and_deleted_anchor_keys_keep_anchoring`. Omitted-matrix live
  anchors remain covered by existing
  `document_integrity::omitted_matrix_membership_keeps_a_live_anchor_without_materialized_parts`.
- Compaction remaps, shrink/removal detaches with frame, group-level linked
  detach rides the normal pair sync (sync failures name the Local exception
  instead of changing one side):
  `compaction_remaps_anchors_onto_the_surviving_key`,
  `removing_the_anchor_slot_detaches_dependants_with_their_frame`,
  `removing_the_anchor_matrix_detaches_dependants`,
  `shrinking_through_set_matrix_detaches_out_of_range_anchors`.
- Placement/Align through anchor frames, Follow representability, truthful
  Rotate availability, constraints solved after anchors against final poses:
  `column_align_moves_through_the_actual_anchor_frame`,
  `anchored_matrix_translation_respects_follow_and_refuses_impossible_drags`,
  `anchored_frame_edits_refuse_unfollowed_components`,
  `constraints_consume_final_anchored_poses`.

Verification (capped: `TMPDIR=/home/chris/.cache`, every heavy command under
`flock boardstudio-build.lock`, compile-then-cap with
`systemd-run --user --scope -p MemoryMax=6G -p MemorySwapMax=0`,
`RUST_TEST_THREADS=4` for the integration binary):

- `cargo test -p boardstudio_core --lib`: `ok. 806 passed; 0 failed`.
- `cargo test -p boardstudio_core --test integration`: `ok. 793 passed; 0 failed`
  (774 pre-existing + 19 new `matrix_anchors`).
- `python3 scripts/check.py quick --files <10 changed paths>`: `check: all steps
  passed: repo, lint, typecheck, test, layout-test-css, browser` in 306.9s
  (repo 1.3s, lint 44.0s, typecheck 20.5s, test 39.1s, layout-test-css 57.2s,
  browser 144.8s).

Integrator notes: do NOT edit `scripts/check-native-tests-baseline.json` here;
the 19 `matrix_anchors::*` names above are new and need baseline reconciliation.
`document_integrity::edits_that_orphan_an_anchor_or_empty_a_layout_are_refused_atomically`
kept its name (baseline-safe) but its shrink case now asserts ticket 19 detach
semantics. No shared goldens, fixtures or baselines changed.

Fix round (review follow-ups, still claimed, no push, `prototype/` untouched):

- Commits on top of `0cb960fa4`: `892f5e54` (Add preview/Undo proof +
  `:231` full-doc strengthening), `02d2bf39` (standards nits +
  rustfmt follow-up), `0afc61ab` (linked-group detach proof).
- Add proof mirrors the change `:231` snapshot pattern:
  `add_anchored_matrix_previews_without_mutation_and_undoes_in_one_entry`
  asserts preview evaluates thumb yet leaves the whole document identical,
  one Undo restores matrices/Layouts/parts, a second Undo reports
  `History is empty` (single history entry), and Redo restores the Add.
- Change `:231` strengthened from matrices-only to full-doc equality:
  trivial, green first run (refusal tests already assert full-doc).
- Group-level linked detach:
  `linked_detach_clears_both_anchors_and_keeps_both_poses` opens a
  two-group doc (source `layout` + `mirror-layout` linked at `axis_x = 10`
  with explicit matrix correspondences) through public `Open`, detaches
  `thumb` via public `Edit`, and proves both anchors clear and all 12
  materialized poses are unchanged, with one-transaction Undo/Redo.
  Green on unmodified production code (honest pass: the claim rides the
  normal pair sync plus the post-edit `layout::resolve`). Non-vacuity
  checked by sabotaging `reflected()` anchor propagation only: the test
  fails at the `thumb2`-anchor assertion (`left: Some(MatrixAnchor ...)`,
  `right: None`); sabotage reverted, tree otherwise untouched.
  The `anchors.rs:242-244` Local-exception naming stays as the
  sync-failure path (e.g. un-mappable counterpart); no linked-detach
  refusal case was reachable through public routes, so the success path
  above is the proving test.
- Standards nits: `validate_target` is now private to `anchors.rs`
  (sole caller), and `linked_layout` carries explicit `&&`/`||` parens.

Integrator notes (fix round): still no baseline edits. Reconcile two more
new names beyond the 19 above (now 21 `matrix_anchors` tests total):
`matrix_anchors::add_anchored_matrix_previews_without_mutation_and_undoes_in_one_entry`,
`matrix_anchors::linked_detach_clears_both_anchors_and_keeps_both_poses`.

Fix-round verification (capped, `TMPDIR=/home/chris/.cache/tmp`, heavy cmds
under `flock`, `RUST_TEST_THREADS=4`):

- `cargo test -p boardstudio_core --lib`: `ok. 806 passed; 0 failed`.
- `cargo test -p boardstudio_core --test integration`:
  `ok. 795 passed; 0 failed` (774 pre-existing + 21 `matrix_anchors`).
- `python3 scripts/check.py quick --files core/src/matrix/anchors.rs core/tests/matrix_anchors.rs`
  (22G cap): `check: all steps passed: repo, lint, typecheck, test,
  layout-test-css, browser` in 200.6s.

## Answer

Resolved on branch `docs/ergogen-layout-comparison` over base `2533a634a` (ticket 18 closure).
Implementation commits: `0467e4ba0` (claim), `b2a784b8a` (typed Add anchored matrix with
dependency-order evaluation), `395951a46` (change/detach with preview, cancel and Undo),
`5655bd868` (atomic graph refusals), `d2bd33390` (Follow subsets, ordering, Follow-aware
refusals), `fbada80cd` (compaction, removal, invisible keys, constraint order), `d1bb1104a`
(shrink detaches dependants with frame, superseding the old refusal; test name kept so the
baseline diff stays purely additive), `117720c0c` (repo rustfmt), `0cb960fa4` (implementation
evidence, kept claimed); fix round `892f5e54` (Add preview/Undo proof, `:231` full-doc
strengthening), `02d2bf39` (Standards nits: private `validate_target`, explicit
linked-`Layout` parens), `0afc61ab` (linked-group detach proof), `13a1fcc62` (fix-round proofs,
kept claimed). Supported native-baseline update `07ac92c9`: `integration` grew 774 to 795,
exactly the 21 `matrix_anchors::*` names inventoried in Progress, zero removals or renames.
Receipt: `.evidence/ergogen-19-integration/native-baseline.json`.

- What delivered: one board-local Layout owns its matrix group; anchors are single-key
  references evaluated in dependency order over all logical members (including
  disabled/deleted/Omitted) before display filtering, using actual final anchor-key frames
  (Origin/Rotation then Follow). Typed Add/change/Detach are one Core transaction each with
  preview, cancel and single-entry Undo/Redo; default Add is 1x3, Origin `(0,-pitch.y)`, all
  Follow, same Layout. Self/cycle/missing/out-of-range/cross-Layout/cross-board,
  constraint-driven-anchor and constrain-an-anchor-key attempts refuse atomically with the
document unchanged. Compaction remaps physical references; shrink/removal detaches
  dependants keeping the world frame. Placement/Align runs through anchor frames with Follow
  representability; impossible edits refuse and Rotate availability stays truthful.
  Constraints solve after anchors against final poses. Linked-Layout Add is refused naming
  ticket 20; group-level linked detach rides the normal pair sync with Local-exception naming
  on sync failure (proving test `linked_detach_clears_both_anchors_and_keeps_both_poses`).
- Reviews: the first Spec review of `2533a634a..0cb960fa4` returned FAIL with two should-fix
  items (unproven group-level linked detach; Add without preview/cancel/Undo proof) plus two
  notes. The fix round closed both with proving tests and the `:231` full-doc strengthening.
  The independent Spec re-review of `0cb960fa4..13a1fcc62` passed read-only with no builds.
  Standards was clean on the implementation range (two nits, both fixed) and the Standards
  re-review of the fix delta was clean with no findings.
- Tests: `cargo test -p boardstudio_core --lib`: `ok. 806 passed; 0 failed`.
  `cargo test -p boardstudio_core --test integration`: `ok. 795 passed; 0 failed`
  (774 pre-existing + 21 `matrix_anchors`). Capped fix-round
  `python3 scripts/check.py quick --files core/src/matrix/anchors.rs
  core/tests/matrix_anchors.rs`: all steps passed (repo, lint, typecheck, test,
  layout-test-css, browser) in 200.6s. Fresh integration gates ran at `07ac92c9` with
  these docs applied: quick 48/48 commands, scoped native 2037 passed with the retention
  list clean; full 55/55 commands, native 2193 passed with 2193 retained names matched
  and zero missing/new, page suite 77/77, drag/provider/CAD smokes passed. Receipts:
  `.evidence/ergogen-19-integration/quick.json` and `full.json` (plus `.md` summaries).

Known limit: mirrored creation and linked-policy evolution belong to ticket 20; this ticket
proves only the group-level detach ride-through plus refusal of linked-Layout Add. No release
approval is implied.
