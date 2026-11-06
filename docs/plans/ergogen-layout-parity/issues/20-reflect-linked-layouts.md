# 20: Reflect complete layout groups and inverse-edit their actual frames

Status: resolved
Type: build
Blocked by: [19](19-anchor-layout-groups.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns linked group reflection (actual/transported frames, both linking systems, inverse edits); the Local/Omitted resize, Rejoin, confirmed detach and source-removal lifecycle is left to [21](21-exception-lifecycle.md).

## Outcome

Support the new layout data and anchors through both same-board Layout links and
native mirrored halves, retaining their distinct ownership systems but one set of
geometry/exception meanings. No second mirror math in a web factory.

## First RED / GREEN slice

Fail a prepared mirror-originating Shift/Bend/Key rotation edit that reflects all
local heading exactly once under a translated/slanted map. Pass it using the shared
kernel and exact inverse; then add an anchored group counterpart case.

## Subsequent acceptance

- Copy new local records and cell turns unchanged. Toggle None/X; root Origin is a
  mapped point and Rotation `φ−r`; anchored Origin X/Rotation negate relatively.
  Do not use forward mapping as the point inverse or negate cell rotation twice.
- Map all Layout members/anchor targets by identity and physical key, independent of
  list order. Preserve numbering, companions, hardware overrides and unrelated links.
- Apply Follow after the candidate and transport its source-world basis. Test the
  non-origin-axis example, slanted maps and unfollowed rotation.
- A dependent follows its actual Local anchor while its own parameters stay linked;
  inverse edits use that actual frame. Reflect compensation once, including column 0.
- Complete same-board per-matrix/key Local/Omitted/Rejoin support consistently with
  native records; do not add per-column exceptions or allow double ownership.
  Ownership note: the Local/Omitted/Rejoin lifecycle is owned by [21](21-exception-lifecycle.md).
- Preserve native companion-mode/side/heading distinctions and existing linked Y,
  ambiguous-alias, simultaneous-target and constraint refusals.

## Checks

Current seams: `core/src/matrix/layout.rs`, `core/src/mirrored_half/{inverse,reconcile}.rs`
and prepared transaction flow. Use real Core/Session edits, preview/commit and Undo
from both halves in both systems. Run focused mirror/matrix/companion tests and quick
checks; the upstream harness alone is not acceptance for these extended rules.

## Progress

Implementation lane on branch `docs/ergogen-layout-comparison`, base `b75b0c805`.
Status remains claimed; no resolve, no push. `web/crates/layout/prototype/` untouched.
Production code: same-board linked Add with mirrored counterpart in `core/src/matrix/anchors.rs`
(group discovery, identity-mapped anchors, `-mirror` identities, matrix correspondences),
anchor-target remap by correspondence in `core/src/matrix/layout.rs::sync` (replacing the stale
target-anchor reuse), relative negate kept in `reflected`, Local independent-pose preservation
in `sync` (either side Local keeps its offset/rotation; carried Shift/Bend still sync),
native mirror-side Add/SetAnchor id mapping in `core/src/mirrored_half/inverse.rs`
(row/column stay the physical key, Follow subset unchanged with kernel transport),
slanted Follow transport proofs in `core/src/matrix/kernel.rs`.

Commits (`git log` SHAs):

- `e7a31c3` claim.
- `9f2a520` linked-group Add, Follow transport, anchor-by-identity, mirror-originating placement.
- `6aff97c` actual Local anchors, column-0 hold-still, Omitted live pose, slanted Follow.
- `7240761` rustfmt + collapsible-if (quick-green).

Proving tests (all via public `Open`/`Edit` routes with preview/commit/Undo unless noted;
21 pre-existing `matrix_anchors` names retained, 6 new integration names + 2 new lib names):

- `matrix_anchors::adding_to_a_linked_layout_is_refused_until_ticket_20` (name retained for
  baseline; body now proves ticket 20 success): Add `claw` anchored to `left-matrix` creates
  `claw` + `claw-mirror` anchored to `right-matrix`, relative Origin `(0,-19)` on both,
  mirror poses `(20,-19)/(1,-19)/(-18,-19)` as the x=10 reflection, matrix correspondences +
  part pairs, one-Undo removes both, Redo restores both.
- `matrix_anchors::linked_follow_transports_source_basis_with_non_origin_axis`: Follow X off
  gives source X=0 and partner X=20 (not independent zero), with preview non-mutation.
- `matrix_anchors::linked_anchor_retarget_maps_by_identity_not_list_position`: reordered
  matrices + membership lists still map `thumb->main(0,0)` to `thumb2->main2(0,0)` with correct poses.
- `matrix_anchors::linked_mirror_originating_shift_bend_reflects_heading_once`: mirror-side
  RowShift/KeyBend copy unchanged, world poses reflect about x=10 (sum 20), headings opposite-signed once.
- `matrix_anchors::linked_dependent_follows_actual_local_anchor`: mirror anchor key made Local
  via `SetLayout`, moved independently to `(-10,5)`; source stays `(38,0)`, `thumb2` follows actual
  `(-10,-14)` while its row/column layouts stay linked.
- `matrix_anchors::linked_column_zero_hold_still_reflects_compensation_once`: column-0
  `OriginRelative (2,1)` from the mirror half copies splay origins unchanged and holds every
  pre-existing pose still (tolerance 1e-9).
- `matrix_anchors::linked_omitted_anchor_keeps_live_logical_pose`: Open with `main2` Omitted
  (no materialized `main2` parts) still evaluates `thumb (38,-19)` and `thumb2 (-18,-19)`
  through the live logical anchor. Move/removal lifecycle stays with ticket 21.
- `matrix::kernel::tests::transported_follow_uses_source_basis_for_slanted_maps`: slanted
  `T(p)=R(30deg)*(-p.x,p.y)+(100,20)` with Follow X off transports (partner X != 0) and matches
  manual `T(F(T^-1(candidate)))`.
- `matrix::kernel::tests::unfollowed_rotation_transports_through_slanted_maps`: unfollowed
  rotation gives world 0 but transported 30 deg, not independent zero.

Preserved refusals (existing tests still green, no model.rs type changes): linked Y mirror
refusal in `reflected`, ambiguous-alias/simultaneous-target in `inverse::matrix_frame`/`move_parts`,
constraint-driven anchor and constrain-an-anchor-key in `anchors::validate_target`/`constraining_refusal`.
Characterization proof (GREEN on current code, guard at `layout.rs:381-382`):
`matrix_anchors::linked_y_mirror_change_is_refused` proves linked Y-mirror refusal leaves state unchanged.
Same-board Local/Omitted geometry proven above; resize/Rejoin/confirmed-detach/source-removal
lifecycle remains ticket 21.

Verification (capped: `TMPDIR=/home/chris/.cache/tmp`, every heavy cmd under
`flock boardstudio-build.lock`, `RUST_TEST_THREADS=4` for integration):

- `cargo test -p boardstudio_core --lib`: `ok. 808 passed; 0 failed` (806 pre-existing + 2 new kernel).
- `cargo test -p boardstudio_core --test integration`: `ok. 801 passed; 0 failed`
  (795 pre-existing + 6 new `matrix_anchors`).
- `python3 scripts/check.py quick --files core/src/matrix/anchors.rs core/src/matrix/kernel.rs
  core/src/matrix/layout.rs core/src/mirrored_half/inverse.rs core/tests/matrix_anchors.rs`
  (22G cap): `check: all steps passed: repo, lint, typecheck, test, layout-test-css, browser`
  in 263.5s.

Integrator notes: do NOT edit `scripts/check-native-tests-baseline.json` here. New names needing
reconciliation: 6 `matrix_anchors::linked_*` + `adding_to_a_linked_layout_is_refused_until_ticket_20`
(body changed, name retained) + 2 `matrix::kernel::tests::*`. Zero removals or renames in existing
`matrix_anchors` (21 retained). No shared goldens, fixtures or baselines changed.

## Answer

Resolved on branch `docs/ergogen-layout-comparison` over base `b75b0c805` (ticket 19 closure).
Implementation commits: `e7a31c3` (claim), `9f2a520` (linked-group Add, Follow transport,
anchor-by-identity, mirror-originating placement), `6aff97c` (actual Local anchors, column-0
hold-still, Omitted live pose, slanted Follow), `7240761` (rustfmt + collapsible-if,
quick-green), `fce9af7` (implementation evidence, kept claimed), `d7ecaa8` (linked Y-mirror
refusal proof: test-only, no production change). Supported native-baseline update `9b4dce8`:
`integration` grew 795 to 802 (+7: 6 `linked_*` + Y-mirror), `boardstudio_core` grew 806 to 808
(+2 kernel), total 2193 to 2202. Baseline diff is 9 insertions, 0 deletions:
`matrix_anchors::linked_follow_transports_source_basis_with_non_origin_axis`,
`matrix_anchors::linked_anchor_retarget_maps_by_identity_not_list_position`,
`matrix_anchors::linked_mirror_originating_shift_bend_reflects_heading_once`,
`matrix_anchors::linked_dependent_follows_actual_local_anchor`,
`matrix_anchors::linked_column_zero_hold_still_reflects_compensation_once`,
`matrix_anchors::linked_omitted_anchor_keeps_live_logical_pose`,
`matrix_anchors::linked_y_mirror_change_is_refused`,
`matrix::kernel::tests::transported_follow_uses_source_world_basis_for_slanted_maps`,
`matrix::kernel::tests::unfollowed_rotation_transports_through_slanted_maps`; name-retained
repurposed `matrix_anchors::adding_to_a_linked_layout_is_refused_until_ticket_20` (body now
proves success, not counted as new). Zero removals or renames. Receipt:
`.evidence/ergogen-20-integration/native-baseline.json`.

- What delivered: same-board Layout links and native mirrored halves share one reflection
  rule set with distinct ownership. Linked Add creates the mirrored counterpart with
  identity-mapped anchors (`-mirror` identities, matrix correspondences, part pairs) and
  single-entry Undo/Redo. Anchor targets remap by correspondence in `layout.rs::sync`
  (replacing stale target-anchor reuse); relative negate stays in `reflected`. Either-side
  Local keeps its offset/rotation while carried Shift/Bend still sync; column-0 compensation
  reflects once with pre-existing poses held still. Follow applies after the candidate with
  source-world basis transport (non-origin axis, slanted maps, unfollowed rotation proven in
  kernel). Mirror-originating Shift/Bend/Key edits copy unchanged with world poses reflected
  once and headings opposite-signed once. Omitted anchors evaluate through the live logical
  pose. Native mirror-side Add/SetAnchor keep row/column as the physical key. Preserved:
  linked-Y refusal (`layout.rs:381-382` guard, proving test leaves state unchanged and is
  Y-specific via exact message + succeeding X-toggle), ambiguous-alias/simultaneous-target,
  constraint-driven refusals. Same-board Local/Omitted geometry proven here; resize/Rejoin/
  confirmed-detach/source-removal lifecycle stays with ticket 21.
- Reviews: Standards read-only review of `b75b0c8..fce9af7` was Clean with no blockers (one
  documented non-blocking stale-name exception for
  `adding_to_a_linked_layout_is_refused_until_ticket_20`, deliberate baseline-stability
  tradeoff; rename recommended at reconciliation). First Spec review of the same range
  returned FAIL with one should-fix (unproven linked Y-mirror refusal) plus four notes.
  The fix `fce9af7..d7ecaa8` (test-only, guard intact) passed independent Spec re-review
  read-only with no builds, closing the FAIL item. No production change in the fix delta.
- Tests: `cargo test -p boardstudio_core --lib`: `ok. 808 passed; 0 failed` (806 + 2 kernel).
  `cargo test -p boardstudio_core --test integration`: `ok. 802 passed; 0 failed`
  (795 + 7 `matrix_anchors`: 6 `linked_*` + Y-mirror). Capped pre-integration
  `python3 scripts/check.py quick --files core/src/matrix/anchors.rs
  core/src/matrix/kernel.rs core/src/matrix/layout.rs core/src/mirrored_half/inverse.rs
  core/tests/matrix_anchors.rs` (22G cap): all steps passed (repo, lint, typecheck, test,
  layout-test-css, browser) in 263.5s. Fresh integration gates ran at `9b4dce8` with these
  docs applied: quick 26/26 commands, scoped native 2046 passed with the retention list
  clean; full 55/55 commands, native 2202 passed with 2202 retained names matched and zero
  missing/new, page suite 77/77, drag/provider/CAD smokes passed. Receipts:
  `.evidence/ergogen-20-integration/quick.json` and `full.json` (plus `.md` summaries).

Known limit: Local/Omitted resize, Rejoin, confirmed detach and source removal belong to
ticket 21; this ticket proves only the live-pose geometry plus refusal preservation. No release
approval is implied.
