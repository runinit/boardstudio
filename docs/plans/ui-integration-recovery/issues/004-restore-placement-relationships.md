# 004: Restore placement and Relationships

Status: resolved
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Placement contract](../../split-board-editing/issues/15-placement-breadcrumb-retire-relations.md), [Audit A2/A4](../../../research/2026-11-06-dev-ui-integration-audit.md#a2a4--relations-contract-and-its-test-changed-together)

## What to build

Restore the approved placement workflow throughout the Layout Inspector using
the current shared controls: “How it is placed” above the fields, Relationships
beside them, and no Relations tab. Matrix and component inspection form one
coherent navigation contract and are recovered together.

## Acceptance criteria

- [x] Breadcrumbs, carried relationships and every applicable mirror-partner,
  follows and anchor fact are available alongside editing without opening a
  Relations tab. Both matrix-context and component inspection obey the contract.
- [x] Breadcrumb and reference navigation use normal selection rules without
  accepted-document or Undo-history writes. Hidden references explain their
  layer and remain navigable; removed targets have no dead navigation action.
- [x] A populated split fixture establishes an observable focused half before
  breadcrumb and reference navigation. The same half remains focused afterward;
  missing labels or an absent split must fail the test, not compare equal.
- [x] Hover and keyboard focus mark the intended reference targets in violet,
  distinct from amber edit previews and persistent anchor feedback. Transient
  feedback clears on leave, blur, selection change and target removal.
- [x] Restore the approved behavioral assertions and show RED before the repair
  and GREEN afterward (A4). Use the real mounted Layout/Inspector and existing
  split-focus, relation-highlight and field-preview fixture patterns.
- [x] Retain shared controls, field editing and draft ownership while retiring
  obsolete tab machinery made unused by this repair. Do not restore old styles
  or refactor unrelated Inspector behavior.
- [x] Update the placement/design and historical ticket records with the recovered
  behavior and its actual verification scope (A8). Preserve earlier evidence as
  history; do not mark the full acceptance gate complete.

## Verification and completion

Run focused mounted production Layout relation, Inspector, split-focus and
field-preview coverage. Record exact RED/GREEN invocations and source identity,
and run the applicable compile/style checks for the touched modules. No MCU
diagnosis, disclosure-motion or scope-action change is a prerequisite. Keep the
parent specification and original audit unchanged.

## Proof map

User decisions (2026-11-10): standalone component/part inspection shows
Relationships beside the fields with no invented matrix breadcrumb (earlier
decision, unchanged); finding return focuses the first editable field. User
explicitly confirmed per `.evidence/004-recovery/acceptance-handoff.log`:
(1) returning from a finding prefers the first native text/number field even
when a dropdown/checkbox button appears earlier; (2) unsaved choices persist
across view changes until committed or the selected owner changes. These are
behavior decisions, not visual acceptance, waiver or promotion approval.
Exact historic oracle recovered from
`510861b11f3abc105b0d7ec948da9e4826307e34` via Gortex commit view (no
exact-source gap): `the_relations_tab_is_gone_and_facts_live_beside_the_fields`.

- R1 matrix no-tab — Column 3 of 5 selected in the mounted Editor → observe the
  Inspector → breadcrumb nav, `#m1-relations-heading` “Relationships”,
  “Inherited by” and a transform input co-present, no `.layout-inspector-tabs`
  · mounted · GREEN (`the_relations_tab_is_gone_and_facts_live_beside_the_fields`,
  `.evidence/004-recovery/green-inspector.log`; RED in `red-relation-component.log`)
- R2 facts retained — bent key / anchored thumb / aggregate anchor / spanning
  selection → Cumulative shift, Follows/Followed by, Anchor part, cross-matrix
  note render and navigate · mounted · GREEN (`bent_key_explains_its_cumulative_shift_and_opens_its_row`,
  `anchored_follows_navigates_to_its_key_without_writing`,
  `hovering_an_aggregate_part_highlights_only_its_key`,
  `selecting_parts_in_two_matrices_explains_the_single_context_chain`; RED pattern in
  `red-one-hover-inherited.log`)
- R3 component no-tab — standalone part mounted → no `[role='tab']`, enabled X
  field, `#component-relationships-heading` and the projected summary co-present,
  no invented breadcrumb · mounted · GREEN
  (`mounted_component_facts_live_beside_the_fields_without_tabs`;
  RED in `red-one-mounted_component_facts_live_beside_the_fields_without_tabs.log`)
- R4 no-write navigation — chip, reference and constraint navigation → selection
  changes, accepted revision unchanged · mounted · reused GREEN
  (`breadcrumb_carries_chip_selects_its_column_without_writing`,
  `anchored_follows_navigates_to_its_key_without_writing`,
  `matrix_relations_panel::a_constraint_reference_selects_its_object_and_escape_returns`)
- R5 hidden/inert — hidden parts → “(hidden layer)” note and still navigates;
  removed anchor → click inert, no selection/revision/error · mounted · reused GREEN
  (`hidden_reference_names_its_layer_and_still_navigates`,
  `a_removed_anchor_key_leaves_its_follows_reference_inert`)
- R6 split focus — populated split (mirrored 1×3 halves), Column 1 selected →
  focus label positively present naming the dimmed mirror; breadcrumb then
  reference navigation → same label present, revision unchanged · mounted · GREEN
  (`split_navigation_keeps_a_present_focus_label_on_the_same_half`;
  RED in `red-one-split-focus.log`; the vacuous `None == None` focus assert left
  `breadcrumb_carries_chip_selects_its_column_without_writing` for this row)
- R7 highlight lifecycle — hover/focus mark violet keys; leave/blur/selection
  change/removal clear; revision unchanged; violet distinct from amber preview
  and anchor badge · mounted · reused GREEN (`hovering_an_inherited_column_highlights_only_its_keys`,
  `keyboard_focus_highlights_and_blur_clears`, `changing_the_selection_clears_the_highlight`,
  `a_key_removed_while_highlighted_leaves_nothing_highlighted`; field-preview and
  anchor-badge suites GREEN unchanged)
- R8 navigation cleanup — hover a reference, navigate via its click →
  selection-change cleanup clears the highlight, selection moves, no write · mounted · GREEN
  (`navigating_a_reference_releases_its_highlight` replaces
  `unmounting_the_actual_relations_reference_releases_its_highlight`;
  RED in `red-one-navigate-clears.log`)
- R9 draft ownership — dirty X draft and disclosures survive an unrelated
  revision, blur lands against the latest owner, Relationships section and
  summary stay rendered across a further revision · mounted · GREEN
  (`mounted_component_position_draft_survives_unrelated_acceptance_and_blur_uses_latest_owner`;
  RED in its `red-one-*.log`); the tab-selection tail it replaces is retired.
  Proof gap, GREEN: unsaved axes choice survives an unrelated acceptance
  (`mounted_offset_form_follows_the_selection_a_conversion_its_undo_and_keeps_unsaved_choices`,
  renamed from `…_and_the_reset`; passes pre/post, needs M7 — see M7 disposition
  below: unreachable mutation, draft preservation proven without it).
- R10 finding focus — mounted component in the panel, focus starts on X →
  finding-return focus lands on Anchor row, the first visible native field, with
  no tab bar · mounted · GREEN
  (`mounted_finding_return_focus_reaches_the_first_editable_field` replaces
  `…_reaches_the_selected_copied_inspector_tab`; RED as the component module
  first failure in `red-relation-component.log`); Board/Outline cases reused GREEN
  (`layout_findings_state::…mounted_back_enter_focuses_board_and_outline_inspectors_and_rejects_stale_scope`,
  `.evidence/004-recovery/green-layout-findings.log`)
- R11 records — this proof map, the boundary below and the completion notes;
  ticket 15 history appended; no full acceptance claimed.

## Slice boundary

Checks (Builder, GREEN): `python3 scripts/run-wasm-tests.py --files` over
`layout_relation_highlight_tests.rs`, `layout_component_inspector_tests.rs`,
`layout_draft_safety_tests.rs`, `layout_field_preview_tests.rs`,
`layout_anchor_badge_tests.rs`, `layout_split_focus_tests.rs`, `layout_workspace.rs`
(112 executed, 0 failed); `wasm-pack test --headless --chrome web/crates/layout
--locked --lib -- layout_findings_state` (2 passed); `cargo fmt --check`;
`cargo clippy --workspace --all-targets`; `cargo test -p boardstudio-web --no-run`.
Each cargo/wasm invocation ran under the FD9 build lock; logs in
`.evidence/004-recovery/`.

Mutations (Checker owns running via `scripts/mutation-check.py`, no outer lock,
FD9 inside the nested test command; a kill anywhere but the named assertion is
a survivor):

- M1 `004-M1-matrix-tab-chrome-restored` (`mutations-relation.json`): re-add a
  `.layout-inspector-tabs` marker in the matrix Inspector →
  `the_relations_tab_is_gone_and_facts_live_beside_the_fields` must fail at “no
  Inspector tab bar remains” · mounted · `run-wasm-tests.py --files
  web/src/presentation/layout_relation_highlight_tests.rs`.
- M2 `004-M2-relations-panel-suppressed`: drop `MatrixRelationsPanel` from the
  matrix Inspector → the same test must fail waiting for “the Relationships
  section renders beside the fields” · mounted · same command.
- M3 `004-M3-split-focus-flipped`: move `#[default]` from `Left` to `Right` in
  `SplitHalf` → `split_navigation_keeps_a_present_focus_label_on_the_same_half`
  must fail naming “Right half” · mounted · same command.
- M4 `004-M4-focus-label-removed`: rename the canvas `data-focus-label` marker →
  the same test must fail in `split_focus_label` (“a populated split renders its
  focus label”) · mounted · same command.
- M5 `004-M5-highlight-release-suppressed`: drop the unmount `highlight.release`
  in `RelationReference` → `changing_the_selection_clears_the_highlight` must
  fail at “a selection change clears the Relations highlight” · mounted · same
  command.
- M6 `004-M6-component-relationships-suppressed` (`mutations-component.json`):
  blank the component Relationships heading →
  `mounted_component_facts_live_beside_the_fields_without_tabs` must fail at “the
  Relationships section renders beside the fields” · mounted · `run-wasm-tests.py
  --files web/src/presentation/layout_component_inspector_tests.rs`.
- M7 `004-M7-constraint-resync-always-resets` (proof-gap row): force the
  accepted-constraint resync true →
  `mounted_offset_form_follows_the_selection_a_conversion_its_undo_and_keeps_unsaved_choices`
  must fail at “the unsaved choice survives an unrelated acceptance” · mounted ·
  same component command.

Checker follow-up disposition (Builder triage, 2026-11-10; detail in
`.evidence/004-recovery/builder-followup-handoff.log`; no production/test
change, 65/65 focused GREEN in `green-followup-65.log`):

- M5 SURVIVED as a nonrepresentative mutation, not an unmet check.
  Selection-change cleanup is the Editor-level `highlight_context` clear
  (`web/src/presentation.rs`, beside the `relation_highlight` provider), not
  `RelationReference`'s `use_drop` (`web/src/presentation/
  layout_matrix_relations.rs`, untouched by 004). Both R7 paths and R8's
  navigation path clear through selection change; removal clears structurally
  (no keys render). The named assertion's mechanism is proven; the drop is
  redundant backup. No new test work.
- M6 KILLED (meaningful, earlier same-literal assertion). The blanked
  `#component-relationships-heading` failed first in
  `mounted_component_position_draft_…` (“stays rendered across an unrelated
  accepted revision”); wasm aborts the module on first panic, poisoning the 24
  later tests including the named
  `mounted_component_facts_live_beside_the_fields_without_tabs`. Ordering
  artifact, not a survivor. Required: Checker reruns M6 alone with a
  single-test filter on the named test via `mutation-check.py` (exact command
  in the follow-up handoff).
- M7 SURVIVED as an unreachable mutation, not an unmet check. The resync
  effect reacts only to `(identity, initial_constraint, nonce)`, all unchanged
  by the fixture's unrelated revision, so the forced branch never runs; the
  branch guard is 011's, untouched by 004 (004 only removed the tab reset).
  Draft preservation has valid proof (R9 RED→GREEN plus the offset tail GREEN).
  No new test work.
- M3 KILLED at the setup assertion (“focus starts on the source half …”),
  proving fixture setup rather than navigation-preserves-focus; all test
  navigation stays within the left half, so even focus-follows-selection would
  pass. R6 keeps its RED + M3/M4 kills as specified. A navigation-focused
  proof (cross-half reference-navigation step plus M3b deriving the canvas
  focused half from the selected context) needs a boundary change — user
  decision, specified in the follow-up handoff, not implemented.

Order: RED → fix → affected checks (done by Builder) → boundary mutations → one
fresh Spec+Standards review → integrated gate (Checker/Reviewer/coordinator).

## Builder completion (004 slice, 2026-11-10)

- HEAD at handoff: `db99b9326` plus the uncommitted 004 worktree changes below
  (no commit, no stash, no push per brief).
- Changed: `web/src/presentation/layout_workspace.rs` (tabs retired; breadcrumb
  + fields + Relationships render together; `LayoutInspectorTab`, tab state,
  reset hook and tab sets removed), `web/src/presentation/inspector/
  layout_component_inspector.rs` (tabs retired; fields + Relationships section
  together; tab prop, tab-only draft reset and “Go to Properties” removed),
  `web/src/presentation.rs` (one line: retired `InspectorInput.inspector_tab`
  construction — boundary-adjacent, flagged for coordinator),
  `web/crates/layout/src/layout_findings_state.rs` (finding return focuses the
  first visible editable field, then a non-return action, then the panel
  fallback), `web/crates/layout/src/objects/matrix_departure_actions.rs`
  (stale tab comment only), plus the relation/component/draft-safety test files.
  `layout_matrix_relations.rs` needed no change; 006, 001, the original audit
  and all foreign dirty docs untouched; no public API widened.
- Retired/renamed proof mapping: `the_breadcrumb_and_relations_tab_both_…` →
  `the_relations_tab_is_gone_and_facts_live_beside_the_fields`;
  `unmounting_the_actual_relations_reference_…` → `navigating_a_reference_…`;
  `mounted_finding_return_focus_…_inspector_tab` → `…_first_editable_field`;
  `mounted_offset_form_…_and_the_reset` → `…_and_keeps_unsaved_choices`;
  Relations-tab-survives tail → Relationships-rendered tail; vacuous split-focus
  assert → `split_navigation_keeps_a_present_focus_label_on_the_same_half`;
  `open_any_relations`/`open_relations_tab` → `settle_relations` (no tab clicks).
- Unmet gates: boundary mutations M1–M7 (Checker), one combined independent
  Spec+Standards review (required: cross-crate focus change), the ticket's
  integrated gate. Gortex `change tests` shows no static reach for the three
  production symbols (wasm-mounted reach is not graph-modelled); behaviour is
  proven by the GREEN oracles above, not the graph.
- Known limitations/risks: the first-editable-field target is fixture-pinned to
  Anchor row (first visible native field; headless selects/checkboxes are
  buttons); hidden/disabled fields are skipped by construction. Both Relations
  sections set the shared transform `inspector_mounted` flag while co-mounted;
  mount/unmount together under the same selection lifetime, teardown-only
  overlap, no behaviour change observed.
- Follow-ups (not blocking): none required by the boundary; any review finding
  that does not show a boundary row unmet stays a follow-up per the stop rule.
- Next action: Checker reruns M6 alone with a single-test filter on the named
  test; then the independent Spec+Standards review; coordinator dispositions
  findings and any proposed boundary amendments (M3b/M5b/M7b in the follow-up
  handoff).

## Acceptance record (004 human acceptance, 2026-11-10)

- Human approval: user stated 'Human acceptance approved.. next ticket please'
  after code commit `4f42cbae03f23ad9123b40770e10a5affde06c5f` and live 004
  preview. No per-test or screenshot observations were stated; none are
  recorded here. Earlier explicit behavior decisions unchanged: standalone
  Relationships beside fields with no invented breadcrumb; finding return
  prefers the first native text/number field even before a dropdown/checkbox
  button; unsaved choices persist across view changes until committed or the
  selected owner changes (`.evidence/004-recovery/acceptance-handoff.log`).
  The 004 preview was served at :4173; the earlier 011 build remains on disk.
- HEAD: accepted code `4f42cbae03f23ad9123b40770e10a5affde06c5f`; this record is
  ticket-only, no production/test change. No commit/push by this agent;
  coordinator handles the authorized 004 acceptance record commit.
- Checks newly executed: none (no test/source changes, no additional gates,
  no screenshots, no preview/server changes per brief). Reused GREEN: Builder
  focused mounted 112 executed / 0 failed + `layout_findings_state` 2 passed
  + `cargo fmt --check` + `cargo clippy --workspace --all-targets` +
  `cargo test -p boardstudio-web --no-run` (logs in `.evidence/004-recovery/`);
  follow-up focused 65/65 GREEN (`green-followup-65.log`); review-fix focused
  65/65 GREEN (`review-fix-focused.log`); full default gate 78/78 PASS in
  732.8s (`.evidence/004-recovery/integrated-gate.log`, receipt
  `/home/chris/.cache/tmp/004-integrated-gate-receipt.json`) before later
  ticket-only record edits; source correlation and build/served binding
  unknown.
- Review provenance/limitations: independent combined Spec+Standards review
  initially partial (base diff view unavailable); blocking unused-`mut` wasm
  lint finding RED (`review-lint-red.log`) then full lint GREEN after removal
  (`review-lint-green.log`). Follow-up scoped review of previously unreviewed
  removed hunks + lint fix returned Spec PASS / Standards PASS, comparing
  exact Gortex base and worktree reads; for five large files it used `difflib`
  on saved Gortex tool outputs because `review.diff_context` returned no diff.
  Other findings classified as follow-ups. Focus helper as reviewed: first
  enabled visible native input, select or textarea; button-based controls only
  as fallback when no eligible native field exists.
- Mutations preserved (Checker via `scripts/mutation-check.py`; a kill anywhere
  but the named assertion is a survivor): M1, M2, M4 killed as specified; M6
  killed at its named assertion in the focused single-test rerun with exact
  restoration (`checker/component-followup/`); M5 SURVIVED as
  nonrepresentative (Editor-level `highlight_context` clear, not
  `RelationReference` `use_drop`); M7 SURVIVED as unreachable (resync branch
  never runs on the fixture's unrelated revision; 011 guard, untouched by
  004); M3 KILLED at the setup assertion only, proving fixture setup rather
  than navigation-preserves-focus. Reviewer treated M3b/M5b/M7b as optional
  follow-ups; they are not relabelled as kills and no optional work was
  demanded. See Slice boundary and
  `.evidence/004-recovery/builder-followup-handoff.log`.
- Criterion disposition (each checked only as supported by existing proof +
  independent review + human approval):
  - C1 placement + Relationships beside fields, no Relations tab (matrix +
    component): SUPPORTED — R1/R2/R3 GREEN with RED
    (`red-relation-component.log`, `red-one-hover-inherited.log` pattern,
    `red-one-mounted_component_….log`); review Spec/Standards PASS; human
    approved after live 004 preview.
  - C2 no-write navigation, hidden-layer note + navigable, removed inert:
    SUPPORTED — R4/R5 reused GREEN (chip/reference/constraint, hidden/inert);
    review PASS; human approved.
  - C3 populated split focus established then preserved, vacuous compare
    rejected: SUPPORTED — R6 GREEN with RED (`red-one-split-focus.log`),
    M3/M4 kills as specified with M3 setup-only limitation preserved and M3b
    optional per reviewer; review PASS; human approved.
  - C4 violet hover/focus lifecycle, distinct from amber/anchor, clears on
    leave/blur/selection-change/removal: SUPPORTED — R7 reused GREEN + R8
    GREEN with RED (`red-one-navigate-clears.log`); M5 nonrepresentative
    survival preserved with mechanism proven and M5b optional per reviewer;
    review PASS; human approved.
  - C5 approved assertions with RED before and GREEN after on the real mounted
    Layout/Inspector (A4): SUPPORTED — RED to GREEN per R1/R2-pattern/R3/R6/
    R8/R9/R10 with mounted logs in `.evidence/004-recovery/`; R4/R5/R7 reused
    GREEN; R9 proof-gap GREEN; R10 Board/Outline reused GREEN; review PASS;
    human approved.
  - C6 shared controls/field editing/draft ownership retained, tab machinery
    retired, no unrelated refactor: SUPPORTED — R9/R10 GREEN with RED, M6
    named-assertion kill, M7 unreachable with draft preservation proven (R9
    RED to GREEN + offset tail GREEN); changed/untouched scope per Builder
    completion (`layout_matrix_relations.rs` unchanged; 006, 001, audit
    unchanged; no public API widened); review PASS (blocking lint fixed);
    human approved including draft retention.
  - C7 records updated with actual scope, history preserved, full gate not
    claimed (A8): SUPPORTED by this record — proof map/boundary/completion
    herein, ticket 15 history appended 2026-11-10 (record otherwise
    unchanged), earlier 004 evidence preserved as history; parent/full
    acceptance gate remains unclaimed. All 7 boxes checked; Status `resolved`.
- Unmet gates / retained cleanup: none required by the slice boundary. M3b/M5b/M7b
  remain optional follow-ups, not prerequisites. No production/test cleanup
  retained.
- Follow-ups (optional, not blocking): M3b cross-half navigation proof +
  canvas-focused-half derivation; M5b/M7b as specified in
  `.evidence/004-recovery/builder-followup-handoff.log`; review non-blocking
  findings as classified by the independent reviewer. Open `needs-triage`
  tickets only if a real defect is confirmed.
- Pointers: this ticket; `.evidence/004-recovery/` (RED/GREEN, mutations,
  `builder-followup-handoff.log`, `review-lint-red.log`,
  `review-lint-green.log`, `review-fix-focused.log`, `integrated-gate.log`,
  `acceptance-handoff.log`); receipt
  `/home/chris/.cache/tmp/004-integrated-gate-receipt.json`;
  [015](../../split-board-editing/issues/15-placement-breadcrumb-retire-relations.md).
- Next action: coordinator commits this authorized 004 acceptance record;
  user said 'next ticket please'.

## Comments

- 2026-11-06: Cancelled by the user. This ticket will not be actioned; its
  acceptance criteria are retained as historical context, not completed evidence.
- 2026-11-06: Published as one behavior slice because the Inspector views share
  tab state and navigation semantics; tests and documentation travel with it.
- 2026-11-08: REOPENED by user during 011 human acceptance review (see 011
  placement/Relations findings). Original cancellation preserved above as history.
  Scoping to follow after 011 defect slices; do not treat the 2026-11-06
  cancellation rationale as current.
- 2026-11-10: Builder slice (Spark Contributor): tabs retired in matrix and
  component Inspectors with facts beside fields, finding return focuses the
  first editable field. Proof map, boundary (M1–M7 for Checker) and completion
  notes recorded above; RED and GREEN logs in `.evidence/004-recovery/`.
  Not resolved: mutations, independent Spec+Standards review and integrated
  gate remain.
- 2026-11-10: Builder follow-up triage (Spark Contributor): M5/M7 survived as
  nonrepresentative/unreachable mutations, M6 killed at a same-literal earlier
  assertion (wasm abort-ordering artifact), M3 kills at setup. No production
  or test change; 65/65 focused GREEN. Dispositions in the boundary above and
  `.evidence/004-recovery/builder-followup-handoff.log`. Not resolved: M6
  focused rerun, independent Spec+Standards review and integrated gate remain.
- 2026-11-10: Review and verification update: the independent combined review
  was partial because the base diff view was unavailable. It found an unused
  `mut` that failed wasm lint; `.evidence/004-recovery/review-lint-red.log`
  records the failure, and `review-lint-green.log` records the full lint step
  passing after its removal. Focused relation/component tests passed 65/65 in
  `review-fix-focused.log`. M6 was killed at its named assertion in the focused
  Checker rerun, with exact restoration (`checker/component-followup/`).
  The full default gate subsequently passed all 78 commands in 732.8s
  (`.evidence/004-recovery/integrated-gate.log`; receipt
  `/home/chris/.cache/tmp/004-integrated-gate-receipt.json`); its source
  correlation and build/served binding are unknown. Full-diff review and human
  acceptance remain open. The focus helper targets the first enabled visible
  native input, select or textarea; button-based controls are used only as a
  fallback when no eligible native field exists.
- 2026-11-10: The independent follow-up review of previously unreviewed
  removed hunks and the lint fix returned Spec PASS / Standards PASS. It
  compared exact Gortex base and worktree reads; for five large files it used
  `difflib` on saved Gortex tool outputs because `review.diff_context` returned
  no diff. The earlier review's blocking wasm lint finding is fixed; other
  findings were classified as follow-ups. Human acceptance remains open,
  including confirmation of native-field-first focus and draft retention.
- 2026-11-10: Acceptance handoff (`.evidence/004-recovery/acceptance-handoff.log`):
  user explicitly confirmed (1) finding return prefers the first native
  text/number field even before a dropdown/checkbox button, (2) unsaved
  choices persist across view changes until committed or the selected owner
  changes; earlier standalone Relationships-beside-fields with no invented
  breadcrumb unchanged. These are behavior decisions only, not visual
  acceptance, waiver or promotion/push approval. Review/gate provenance:
  independent combined review initially partial (base diff unavailable);
  follow-up scoped review of removed hunks + lint fix returned Spec PASS /
  Standards PASS with five large per-file diffs via `difflib` on saved Gortex
  outputs (`review.diff_context` unavailable); full gate 78/78 PASS in
  `.evidence/004-recovery/integrated-gate.log` (receipt
  `/home/chris/.cache/tmp/004-integrated-gate-receipt.json`, source
  correlation and build/served binding unknown) before later ticket-only
  record edits; M5/M7 survived as nonrepresentative/unreachable, M3 kills at
  setup only, M6 killed at its named assertion (see boundary). Existing 011
  preview at :4173 when alive is not 004. No promotion, commit or push. Status
  set to `ready-for-human`: outstanding human work is acceptance of 004 per
  its unchecked criteria (placement + Relationships, finding-return focus,
  draft retention); acceptance boxes remain unchecked. M3b/M5b/M7b are optional
  follow-ups, not prerequisites for acceptance under the slice boundary.
- 2026-11-10: Human acceptance approved. User stated 'Human acceptance approved..
  next ticket please' after commit `4f42cbae03f23ad9123b40770e10a5affde06c5f`
  and live 004 preview. No per-test or screenshot observations were stated.
  Acceptance record above dispositions all 7 criteria as supported; Status set
  to `resolved`. Parent/full acceptance gate remains unclaimed.
