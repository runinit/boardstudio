# 113: Inverse-map outline and reviewed-geometry edits

Status: claimed
Type: build
Blocked by: [112](112-inverse-map-hosted-electrical-edits.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Prepare private translation of the outline families and settlement inputs listed in 110.
Do not route production outline operations through the translator until 111.

## Scope and evidence

At `2478fb151`, top-level `SetOutline`, `CopyOutline`, `SelectOutline`, `RenameOutline`,
`RemoveOutline` and `ApplyOutlineEdit` target board/version/feature geometry and identities.
`core/src/outline_edit.rs::OutlineChange` has 24 variants enumerated in 110. Reviewed contours
use exact equality: mapping only the target can incorrectly retire a valid reviewed edit.

- Classify each of the 24 changes. Scalar settings retain values; geometry/refs map through
  108's world/part-anchor rules; lifecycle requests retain allocation/seed/settlement meaning.
- Translate board/version/feature IDs, geometry, reviewed shapes, nested controls/bridges/gaps,
  winding/index associations and ownership together. Preserve authored identity reservations.
- Test linked versus fixed versions, additions/removals and relevant-versus-unrelated review
  retirement. Do not change existing outline lifecycle or exact review comparison semantics.
- Keep this as a private prepared adapter. If all 24 cases cannot fit a bounded session, report
  the exact remaining lifecycle family to root before requesting further IDs or coding broadly.

## File ownership

Exclusive: private mirrored-half translation module(s), established mirrored-half tests.
`core/src/outline_edit.rs` and outline/mechanical implementation files are read-only evidence;
request root ownership for any narrowly necessary helper change. No visibility widening,
operation schema change, production route or generated golden edit. Root granted a narrow
exception for `core/src/mirrored_half/reconcile/outlines.rs` and its established tests: classify
anchored geometry by actual source-board matrix membership, per the owner's attached-part rule.

## Test-first and acceptance

- [ ] Direct translation tests fail first and classify every top-level/nested outline variant.
- [ ] Axis and rotated/translated-map geometry, anchored controls and reviewed contours map
      consistently; source-equivalent reviewed edits remain admissible.
- [ ] Relevant changed review inputs retire, unrelated changes do not; identity/seed allocation
      and independent local exceptions preserve existing behavior.
- [ ] No unclassified variant hides behind wildcard pass-through; production routing remains
      unchanged, with the owner-approved outline anchor classification exception above.

Independent verifier: mutate reviewed-contour conversion, winding/anchor mapping and lifecycle
identity translation separately. A missing lifecycle family blocks resolution, not parent scope.


## Comments

- Initial adapter implemented on branch `typed-core-edits/113-inverse-map-outline` from `62e7d85d1`, in private `mirrored_half::inverse::outline` plus established mirrored-half tests. The later owner-approved anchor classification change to forward outline reconciliation is narrowly recorded above; no production route, payload or visibility changed.
- All six top-level outline operations are classified. Every one of the 24 `OutlineChange` variants has an explicit match arm: scalar settings and names/seed remain authored values; feature, connection, gap, control, part/matrix reference, world/anchored geometry, winding and reviewed contour inputs are mapped; lifecycle operations keep their existing operation and settlement semantics. Explicit IDs are reversed by stored correspondence or whole-board-token mapping. Local targets execute as written; omitted targets refuse.
- Exact review equality exposed floating point drift for a rotated map (`inverse(forward(point))` differed by a few ulps). The private adapter now substitutes the exact current source contours only when the submitted review exactly equals the current mirror projection. A stale review is still inverse-mapped and remains stale. Settlement implementation and equality semantics were not changed.
- Red/green: the initial linked `SetOutline` test failed on the ticket-113 refusal, then passed after translation. Final focused suite: `cargo test -p boardstudio_core mirrored_half::inverse::tests:: --lib` — 50 passed, 0 failed.
- Mutation evidence: `.evidence/ticket113-mutations-final/results.json` — reviewed-contour normalization, polygon winding, anchored control handedness and created-version identity mutations all killed; 0 survived, 0 compile failures, every source file restored exactly.
- Independent review follow-up: the owner selected the rule that each attached part uses its own frame. Forward outline reconciliation derives matrix-owned parts from the source board's matrix memberships, matching creation-time ownership. Inverse mapping now checks membership in a mirror-board matrix and the board's part list, not the `matrix/` ID prefix. The red regression at `.evidence/ticket113-cross-matrix-forward-red/output.txt` showed a second-matrix anchor was incorrectly treated as full-rule under the old active-matrix heuristic. Forward and inverse regressions pass for anchors in a second matrix while the projection targets the base matrix.
- Follow-up coverage also adds direct `ApplyOutlineEdit` translation for Fixed and Linked target versions with distinct review shapes; the Fixed version remains active while the non-active Linked version is edited. Each target's current review maps to its own exact source shape, while a stale review stays unequal. The shared `substitute_token` helper now handles whole-token inverse replacement; the inverse-only prefix fallback remains.
- Token helper mutation: `.evidence/ticket113-token-mutation/results-shared-inverse-token-replacement.json` — mutation skipping shared whole-token replacement was killed by `every_outline_change_arm_is_classified_and_keeps_the_targeted_scalar_or_lifecycle_value` (1 killed, 0 survived, 0 compile failures).
- Cross-matrix mutation: `.evidence/ticket113-cross-matrix-rule-mutations/results.json` — wrong forward anchored-frame selection was killed by 4 failures and wrong inverse frame selection by 1 failure (2 killed, 0 survived, 0 compile failures). Follow-up prefix-only mutation: `.evidence/ticket113-final-ownership-mutation/results-prefix-only-matrix-ownership.json` — killed by the non-member `matrix/...` regression (1 killed, 0 survived). Legacy membership mutations: `.evidence/ticket113-legacy-matrix-membership-mutations/results.json` — both prefix-only classification and requiring `Matrix.board_id` were killed (2 killed, 0 survived). Focused suites after follow-up: 14 outline reconciliation tests and 54 inverse tests passed. The canonical native test baseline was updated with `python3 scripts/check.py test --update-native-baseline`: 1706 executed tests, all steps passed.
- Independent review correction: inverse matrix-frame classification precomputes the same ownership set as forward mapping: all `matrix.part_ids` intersected with the mirror board's `part_ids`, regardless of optional `Matrix.board_id`. This preserves legacy matrix ownership metadata and avoids rescanning the document for each anchor. A `matrix/...`-shaped linked part without actual matrix membership uses the independent/full-heading rule. The second-matrix inverse fixture includes actual matrix and board membership. The pre-fix red assertion was `a_matrix_shaped_anchor_without_matrix_membership_uses_full_heading_rule`: prefix classification returned `[-12,14],[-8,10],[-4,6]`, expected `[12,-14],[8,-10],[4,-6]`.
- Settlement acceptance now calls the existing `outline_edit::evaluate` directly on the translated operation: the current review is accepted, changing the reviewed source feature retires it, and renaming the unrelated mirror board preserves it. No production route or settlement helper was changed. Inverse `contour()` duplication and the unreachable Version target arm after the early return were also removed.
- Review-retirement mutation: `.evidence/ticket113-final-review-retirement-mutation/results-skip-reviewed-shape-retirement.json` — skipping the existing review comparison was killed by the direct settlement assertion (1 killed, 0 survived); the read-only settlement implementation was restored exactly.
- Legacy membership red/green: `legacy_matrix_membership_without_board_id_uses_matrix_frame_rule` failed before the ownership-set adjustment because inverse produced `[12,-14],[8,-10],[4,-6]` instead of matrix-frame `[-12,14],[-8,10],[-4,6]`; it passes with the forward-equivalent membership intersection.
- Independent review and integration remain outstanding; acceptance boxes and status remain open/claimed for the coordinator.
