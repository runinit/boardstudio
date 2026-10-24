# 21: Preserve Local/Omitted state through resize, detach, Rejoin and removal

Status: ready-for-agent
Type: build
Blocked by: [20](20-reflect-linked-layouts.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns the exception lifecycle (Local/Omitted resize, Rejoin, confirmed detach, source removal) and the Core behaviour of confirmed exception changes; linked group reflection is left to [20](20-reflect-linked-layouts.md) and the UI confirmation to [23](23-inspector-toolbar-and-handles.md).

## Outcome

Close the recorded lifecycle gaps through the actual prepared route, not merely the
direct helper tests. Preserve explicit ownership, geometry and source identity in one
atomic transaction; confirmation is not permission for silent extra exceptions.

## First RED / GREEN slice

Fail a public source resize with a Local partner matrix: source grows, Local dimensions
and poses stay independent, with valid correspondences. Fix that route before adding
the Omitted-partner case; do not bypass production preparation in the test.

## Subsequent acceptance

- Local keys retain world poses through shared changes using only independent offset/
  turn compensation; their Shift/Bend remain shared. Explicit pose edits remain local.
- Make Local on an anchored matrix detaches/keeps pose; its future layout stays own.
  Omitted items retain live logical poses; Local→Omitted uses current shared layout
  rather than freezing former Local placement. Preserve explicit child exceptions.
- Rejoin adopts current source dimensions/layout/anchor; missing anchors or invalid
  graph refuse atomically without changing a Local parent or unrelated state.
- Shared/as-removal Detach preserves both poses. If another Local exception is needed,
  expose the proposed changes for explicit confirmation before commit; cancel changes
  nothing. Revalidate the confirmed intent against current accepted state.
- Source removal retains a complete Local matrix/group, not loose parts; retire only
  dead correspondence, preserve surviving ownership/anchors and unrelated links. No
  source replacement is silently joined. Omitted records retire without resurrection.
- Cover same-board and native systems, source/follower edits, frame/Follow restrictions,
  child exceptions, identity remaps, preview/cancel, one Undo and exact Redo.

## Checks

Primary source: `core/src/mirrored_half/lifecycle.rs`, reconciliation membership kernel
and same-board layout lifecycle. Regressions must enter Core/Session production
preparation; cite the helper/production discrepancy from [decision 11](11-mirrored-halves-rules.md#answer) but do not treat
those helper tests as proof. Run focused lifecycle/mirror tests, then quick/native
checks with name retention. No private adapter may become a second authority.

## Progress

Implementation lane on branch `docs/ergogen-layout-comparison`, base `eb4e0ac63`.
Status remains claimed; no resolve, no push. `web/crates/layout/prototype/`
untouched.

Implementation lane on branch `docs/ergogen-layout-comparison`, base `eb4e0ac63`.
Status remains claimed; no resolve, no push. `web/crates/layout/prototype/`
untouched.

Commits (`git log` SHAs):

- `6be9bf2` test-only (RED): public source resize with a Local partner.
- `dcbd45c` Omitted partners live/absent through source resize.
- `d1a0f4b` Make-Local detach, Local→Omitted reset, Rejoin materialize.
- `245aa5f` Rejoin adoption, grouped-edit exclusion, Omitted guards.
- `14a5e06` confirmed detach for asymmetric linked groups.
- `4358e55` linked source-removal lifecycle (+ superseded-test update in
  `core.rs` keeping the name for the baseline).

Proving tests (all via public `Open`/`Edit` routes with preview/cancel/Undo
where noted; 14 new `matrix_anchors::*` names for the integrator baseline,
1 updated `core.rs` name retained):

- Resize: `linked_resize_keeps_local_partner_dimensions_and_poses`,
  `linked_resize_keeps_omitted_partner_live_and_absent`.
- Make Local / Omit / Local→Omitted:
  `linked_make_local_on_anchored_matrix_detaches_and_keeps_pose`,
  `linked_local_to_omitted_key_returns_to_shared_pose`
  (Omit sheds parts but keeps explicit child exceptions; tested),
  `linked_compaction_excludes_local_partner`,
  `omitted_storage_edits_are_refused_with_rejoin_guidance`.
- Rejoin: `linked_rejoin_adopts_current_source_and_keeps_child_exceptions`,
  `linked_rejoin_refuses_missing_anchor_atomically` (names `matrix/main2/r0c2`,
  Local parent and unrelated state byte-identical).
- Detach: `linked_detach_needing_local_exception_refuses_with_proposal`
  (RED verified without the guard), `linked_detach_confirmed_applies_..._in_one_entry`
  (new `DetachMatrixAnchorConfirmed` operation, stale-confirmation refusal).
- Removal: `linked_source_removal_retains_local_matrix_and_retires_only_defunct_links`,
  `linked_source_removal_deletes_linked_partner_and_keeps_unrelated_pair`,
  `linked_source_removal_retires_omitted_records_without_resurrection`.
- Updated (supersedes pre-decision-11 survival, name kept):
  `core::mirrored_pair_deletion_keeps_surviving_half_and_cleans_membership`
  now proves Local survival; linked follow-deletion is proven in
  `matrix_anchors`.

Production code: `core/src/matrix/layout.rs` (`sync` Local skip / Omitted
live-update with remap-or-preserve and validity guard, `edits_alone`,
`refuse_omitted_storage_edit`, `apply_exception_transitions`,
`add_local_matrix_exception`, `plan_linked_matrix_removal`,
`synthesize_rejoin_record`), `core/src/matrix/anchors.rs`
(`linked_detach_needs_local` probe-on-clone, `detach_confirmed`, confirmed
refusal in `detach`), `core/src/matrix/deletion.rs` (grouped-edit
exclusion, Omitted refusals, per-group canonicalization),
`core/src/lib.rs` (SetLayout lifecycle hook, RemoveMatrix removal plan,
Omitted refusals, confirmed arm), `core/src/model.rs` + native
`inverse.rs`/`hosted.rs` (confirmed variant mapping).

Residual (native, for a follow-up — same-board contract fully proven):
native matrix-level Local/Omitted + membership-change transitions
(expansion, removal) are refused by the reconciliation kernel at
`validate_matrix_association` (`reconcile.rs:5168`, "The transition matrix
association disagrees with its actual pair correspondence": the matrix
association carries `Local explicit=true` where the gate demands
`Linked`). Native part-level lifecycle is already proven by parent19;
native origin-only edits with a Local mirror succeed. Repro: matrix-level
`set-half-exception state=local` on the mirror matrix, then source
`set-matrix` expansion or `remove-matrix`. Fixing needs kernel-flow work
(validation + mirror-projection skip + member accounting), deliberately
not attempted here to avoid risking the mature kernel.

Integrator notes: do NOT edit `scripts/check-native-tests-baseline.json`
here. New names needing reconciliation: 14 `matrix_anchors::*` listed
above; `core::mirrored_pair_deletion_...` is name-retained with changed
body. No shared goldens, fixtures or baselines changed.
