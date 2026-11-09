# Dev UI integration audit — 2026-11-06

## Scope and evidence boundary

Audited candidate: local `dev` at `1fda4f48a964568b98dde9c82556350c4bc50911`.
Pre-migration integrated baseline: `510861b11` (2026-10-30).
The session checkout is `ci-wall-time-plan`; it is NOT the application baseline.
The candidate source was inspected in `.worktrees/dev-live-review` and through
`git show`. This report is the only file created by this audit.

This is commit/history and targeted source inspection, not an exhaustive review
of all 322 changed files, a fresh build, or browser acceptance. No product code,
tests, refs, or worktrees were changed. No tests or mutation runs were performed.
The user's screenshots establish reported symptoms; they do not establish the
identity of the loaded WASM or the affected document's internal state.

Inspected: branch/worktree inventory; original Layout and split-board histories;
the migration merge against its integrated-dev parent; changed tests; selected
production paths in Layout, Case, Parts, PCB, Keymap, Keycaps, Library, Export,
shared docks/disclosures, and the merge-gate record. An absent original commit
is not treated as missing functionality: much of the older work was squashed.

## What happened

- October 25: `40df01155` fixed mirrored-controller heading; `9ffbe9a8d`
  implemented the approved placement breadcrumb and retired the Relations tab.
  `7c3c47a95` removed duplicate scope-popover tools; later scope work suppressed
  the redundant panel when the toolbar already contained the actions.
- October 28: `3602a9b27` integrated Core anchors and the Ergogen parity kernel;
  `51a3a180d` integrated half frames, adaptive split spacing and part anchoring;
  `38fc8e911` integrated toolbar/polish work. Magnetic snapping also reached dev.
- November 5: migration replaced controls, retired the old stylesheets, and
  rewrote DESIGN.md. Snap switched to the copied Collapsible component.
- November 6: `c79de4708` merged integrated dev into the migration line. Its
  result restored older UI contracts for Relations and scope action popovers.
  Tests were changed to expect those contracts. Follow-up fixes made the merged
  candidate build/test, then dev advanced to `1fda4f48a`.

The central failure is partial behavioral reconciliation during UI migration.
It is not wholesale loss of the layout engine or proof that all old branches
should be merged again.

## Prioritized consolidated findings

Priority describes recovery order, not proof of data loss. P1 means resolve
before accepting this candidate; P2 means UI correctness/polish recovery.

| ID | Priority | Finding and evidence status | Project impact | Next action |
| --- | --- | --- | --- | --- |
| A1 | P1 investigate | Mirrored MCU remains rotated in the user's screenshot; the original fix IS present in candidate source. Root cause open. | Potentially incorrect component orientation; downstream output impact is unknown until the stored/evaluated/rendered poses are compared. | Reproduce with the exact project and identify loaded build, definition kind, mirror linkage, stored pose, evaluated pose and SVG transform. |
| A2 | P1 fix | Relations redesign partially reverted by the merge. Confirmed source/spec mismatch. | Restores the old navigation model and hides relationship facts behind a tab instead of beside editable fields. Discards an approved interaction decision. | Restore the ticket-15 placement contract using current shared controls. |
| A3 | P1 fix | Scope popovers again duplicate inline transform actions. Confirmed source regression and user screenshot. | Competing entry points, clutter and overlapping canvas controls make matrix/row/column editing harder to understand. | Restore one action surface per selected scope, preserving no-selection guidance. |
| A4 | P1 verification | Tests were rewritten to accept A2/A3 rather than preserve their behavioral contracts. Confirmed diff evidence. | Green checks can certify the opposite of approved behavior; future merges can repeat this failure. | Restore contract assertions alongside A2/A3 and demonstrate their RED/GREEN behavior. |
| A5 | P1 acceptance | Final full default/browser gates, whole-branch review and full human workspace review remain open in ticket 026. Confirmed record, not a new product defect. | Reliability of drag → save → Undo and integration across all pages is not established by the scoped green runs. | Complete missing coverage against the final reconciled candidate, without relabeling prior partial runs as full acceptance. |
| A6 | P2 fix | Snap uses a Collapsible wrapper without expansion motion. Confirmed missing styling; user reports abrupt opening. | Loss of interaction polish; the shared wrapper also provides no expansion animation to other callers unless they supply their own. | Add and verify deliberate open/close motion and reduced-motion behavior; handle mounted/hidden content correctly. |
| A7 | P2 fix | Disclosure chevrons use the wrong state selector at 17 Layout-related call sites. Confirmed against pinned primitive source. | Expanded/collapsed arrows do not reflect the disclosure state across Layout tree, matrix and component inspectors. | Use the actual primitive state contract and check open/close at representative nested disclosures. |
| A8 | P2 records | Ticket 15 still says the Relations tab is gone while code/tests explicitly restore it; ticket 026 is scoped-resolved with acceptance boxes still open. Confirmed documentation inconsistency. | Future agents/reviewers can mistake implementation completion for product acceptance or follow contradictory design instructions. | Reconcile records with recovered behavior and keep acceptance evidence explicit. |

## Evidence and acceptance per finding

### A1 — MCU: fix integrated, observed symptom unresolved

`core/src/mechanical/pair.rs:67` implements `mirrored_part_heading`: controllers
use the reflected heading without the generic independent-part half turn.
`core/src/mirrored_half/derive.rs` identifies controllers by PartKind; the
reconciliation and inverse paths call the shared rule. This is the carried
form of `40df01155`, not a missing cherry-pick.

`git diff 510861b11 dev -- core application` is empty. That rules out a Core or
application source change in this migration range; it does not prove the
loaded artifact, document classification, UI inputs or rendering are correct.
Do not assume stale saved state or patch the renderer until the actual path is
identified. Recovery acceptance must compare both halves' evaluated orientation,
rendering, save/reload and the relevant export if document geometry is affected.

### A2/A4 — Relations contract and its test changed together

Original contract: `docs/plans/split-board-editing/issues/15-placement-breadcrumb-retire-relations.md`.
Recovery history: `9ffbe9a8d`; integrated reference: `510861b11`.

Candidate `web/src/presentation/layout_workspace.rs:1072-1109` retains the
breadcrumb but wraps Properties and Relations in tabs. Component Inspector
also retains tab machinery. The merge restored `LayoutInspectorTab`.
In `layout_relation_highlight_tests.rs`, the test named
`the_relations_tab_is_gone_and_facts_live_beside_the_fields` became
`the_breadcrumb_and_relations_tab_both_expose_placement_facts`; the assertion
that no Inspector tab bar exists was removed, and helpers now open Relations.

Acceptance: all facts remain reachable beside the selection; breadcrumbs and
references navigate without moving split focus; violet reference highlighting
remains distinct from amber field previews; no Relations tab is required.

### A3/A4 — duplicate actions explicitly expected by the new test

`web/crates/layout/src/objects/layout_toolbar.rs:1246` renders inline context
actions, and `:1351` renders another context-action toolbar in the scope panel.
The candidate test at `:547`,
`column_scope_trigger_opens_migration_actions_when_tools_visible`, explicitly
expects Stagger/Spread/Offset in the popup. The baseline instead asserted no
scope panel when inline tools were visible and counted each action once.

Acceptance: Matrix, Row and Column scopes each expose the intended actions
once, including after scope reselection; no-selection prompts and keyboard
access still work. Recover behavior, not old CSS or obsolete wrappers.

### A5 — what the existing green results actually cover

`docs/plans/rust-ui-build/issues/026-merge-gate.md` records a successful quick
plan, checked build and partial crate-browser run. It explicitly says these
are not the full default/browser gate. Its remaining acceptance boxes include
full checks, human workspace review and a whole-branch two-axis review.

The record reports drag-smoke blocked at the Rows slider focus wait and also
notes a dead `.m1-save-state` selector. It leaves follow-up filing open for the
drag rewrite and the 20 opt-in agent-browser scripts. These are retained
recorded results, not reruns during this audit. The fact that failures were
classified as pre-existing does not supply the missing end-to-end evidence.

Acceptance: run the agreed final-source checks; prove real gesture/save/Undo
behavior; review populated Layout, Case, Parts, PCB, Keymap, Keycaps, Library,
project menu and Export across the agreed theme/appearance/width matrix.

### A6/A7 — shared disclosure integration

Snap migration: `99b316c08`, `ee2017790`.
`web/crates/layout/src/objects/layout_toolbar.rs:1402` uses Collapsible.
`web/crates/ui-shared/src/ui/collapsible.rs:49` gives its content only
`overflow-hidden`; its header comment nevertheless claims a grid transition.
The baseline BoardAccordionSection had opening/closing motion rules.

Seventeen `group-data-[state=open]:rotate-180` occurrences appear in Layout
objects, matrix inspectors, layout workspace, component Inspector and companion
placement. Examples: `objects/matrix_transform_inspector.rs:416`,
`objects/matrix_inspector.rs:574`, `objects.rs:824`.
The pinned dependency at revision `9a75825`,
`primitives/src/collapsible.rs:181`, emits `data-open`, not `data-state`.
The copied wrapper does not translate it. Other primitives do use data-state;
do not make a global string replacement across unrelated components.
Context7 was consulted for component documentation; the exact pinned source
is the evidence for this attribute mismatch.

## Recovered work that is already integrated

These are source-presence checks, not fresh behavior acceptance:

- Core anchors/Ergogen parity kernel, aggregate-anchor fixture work, split-half
  frames and adaptive spacing exist in candidate source. The parity claim is
  the accepted subset, not complete Ergogen YAML compatibility.
- Magnetic snap and the gap-radius correction are in dev under rewritten
  commits, including `12b59784c` and `17b891597`.
- Inspector moves using evaluated board coordinates are carried as
  `ec47d0ef7`; `layout_component_edits.rs` is unchanged in this migration range.
- The Parts assembly-coordinate overlap fix `c576c8614` survives as
  `assembly_editor.rs:1186`'s responsive auto-fit grid. Do not restore the old
  stylesheet to recover this already-preserved fix.
- Field-preview handlers, split focus, spacing ghosts, canvas anchor badge and
  reference highlight logic remain present. Their existence does not certify
  final theme rendering, and they belong in the recovery browser matrix.
- The historical Detach pose-test adjustment is represented by evaluated-pose
  comparisons in the current test; original SHA reachability alone would have
  incorrectly marked it missing. No fresh Detach defect is asserted here.

## Deliberate differences, not restoration defects

- Auto-hide removal and migration of saved auto-hide to pinned are explicit in
  rust-ui-build ticket 011 and DESIGN.md. Preserve widths; do not restore hover
  timers merely because the earlier implementation had them.
- Custom controls, the palette/Toasts and the revised dock structure are
  declared migration changes. Assess their behavior against the accepted
  migration spec rather than treating all visual differences as regressions.
- KeycapsKeyList's WASM-only presentation boundary is documented as deliberate
  in ticket 022; it is not evidence of a missing UI page.

## Workspace coverage and remaining uncertainty

Layout has direct source/spec regressions above. Case, Parts, PCB, Keymap,
Keycaps, Library and Export received targeted diff inspection of their migrated
editors, disclosures or workspace hosts. No additional high-confidence
workspace-specific data-editing regression was established in those samples.
This is not a clean bill of health: no populated-page browser review, animation
measurement, focus/scroll test or export round trip was performed here.

The shared motion gap reaches beyond Snap as a component limitation, but not
every caller previously promised animated disclosure. Classify per-page motion
restoration only after comparison with its accepted design.

## Recovery order

1. Establish exact MCU reproduction and candidate identity (A1); in parallel
   planning terms, restore the known Relations and scope contracts with their
   original behavioral assertions (A2–A4).
2. Repair shared disclosure motion and Layout chevron state binding (A6/A7).
3. Reconcile docs and acceptance state (A8); verify the final merged candidate
   through the missing integration and populated-workspace review (A5).

Use `510861b11` as the integrated behavioral reference, the approved prototype
`prototype/split-board-editing` at `ef1c8a008` for interaction intent, and the
original feature histories for provenance. Do not wholesale merge those older
branches: they contain squashed equivalents, old harnesses and obsolete styles.
