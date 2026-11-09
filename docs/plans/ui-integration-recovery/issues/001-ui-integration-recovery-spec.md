# 001: UI integration recovery specification

Status: ready-for-agent
Type: planning
Owner: fix/ui-integration-recovery
Blocked by: none for ticket planning; implementation awaits the to-tickets step
Related: [Plan map](../map.md), [Integration audit](../../../research/2026-11-06-dev-ui-integration-audit.md), [UI migration spec](../../rust-ui-migration/spec.md)

## Problem Statement

The integrated UI candidate retains the newer shared controls but restores
obsolete Layout interactions: relationship facts require a Relations tab, and
scope popovers repeat actions already available beside the canvas. Tests were
changed to accept those regressions. Snap opens abruptly, and disclosure arrows
do not follow the actual expanded state. A user's screenshot also shows an
incorrectly oriented mirrored MCU, despite the original heading fix being
present in source; its cause and downstream geometry impact remain unknown.

The recorded quick and partial browser checks do not establish the missing real
gesture/save/Undo workflow or acceptance of populated Workspaces. Some records
describe recovered behavior that the candidate no longer provides, while a
scoped-resolved migration ticket still has open product acceptance criteria.
Users and maintainers need a coherent editing experience and evidence that
corresponds to the actual final candidate.

## Solution

Restore the approved placement and scope workflows using the current shared
controls. Keep existing engine work and deliberate migration decisions. Diagnose
the actual MCU symptom before choosing a repair. Recover shared disclosure
motion and correct chevron binding as distinct behaviors. Restore meaningful
behavioral assertions and correct documentation alongside each relevant change.

Finish with real gesture, durable save, Undo and reload evidence; full final-source
checks; independent spec and standards review; and human review of populated
Workspaces. Implementation completion and product acceptance remain distinct.

| Finding | Evidence at planning | Recovery outcome |
| --- | --- | --- |
| A1, P1 investigate | User-observed MCU rotation; heading fix present; cause open | Reproduce, diagnose and verify the actual affected orientation path |
| A2, P1 fix | Confirmed Relations workflow regression | Placement breadcrumb and Relationships beside editable fields; no Relations tab |
| A3, P1 fix | Confirmed duplicate scope actions | One contextual action surface, with correct selection and hint transitions |
| A4, P1 verification | Assertions accept regressions; split-focus fixture is insufficient | RED/GREEN behavioral proof accompanying each repair |
| A5, P1 acceptance | Full gates and integration/human review remain open | Final-source acceptance, including real drag/save/Undo/reload |
| A6, P2 fix | Shared expansion motion absent | Deliberate open/close motion with reduced-motion and content-lifetime handling |
| A7, P2 fix | Seventeen Layout-related chevrons use an incompatible state selector | Arrows reflect their own disclosure state, including nested sections |
| A8, P2 records | Behavior, heading guidance and acceptance records are inconsistent | Documentation reconciled with behavior and evidence at the relevant repair |

P1 identifies work required before acceptance, not proof of data loss. P2 work
also remains part of this recovery; its priority is not an acceptance waiver.

## User Stories

1. As a keyboard designer, I want the newer shared controls to retain the approved editing workflows, so that a visual migration does not change how I place and inspect objects.
2. As a keyboard designer, I want a “How it is placed” breadcrumb above the Inspector fields, so that I can understand the selected object's placement context while editing it.
3. As a keyboard designer, I want Relationships beside those fields without a Relations tab, so that placement facts remain available with the values they explain.
4. As a keyboard designer, I want all applicable mirror-partner, follows, anchor and carried relationships preserved, so that removing the tab loses no information.
5. As a keyboard designer, I want breadcrumb and “Carries” chips to select their referenced objects, so that I can navigate the placement structure directly.
6. As a split-keyboard designer, I want breadcrumb and relationship navigation to preserve the half I am editing, so that inspection does not unexpectedly move split focus.
7. As a keyboard designer, I want reference navigation and highlighting to leave the accepted document and Undo history unchanged, so that looking around is not treated as an edit.
8. As a keyboard designer, I want hovered or keyboard-focused references to highlight their actual targets in violet, so that I can distinguish relationships from amber edit previews.
9. As a keyboard designer, I want transient highlighting to clear on leave, blur, selection change or target removal, so that old references do not remain misleadingly highlighted.
10. As a keyboard designer, I want hidden references to name their layer and remain navigable while removed references become inert, so that inspection explains the target's actual availability.
11. As a keyboard designer selecting Matrix, Row or Column scope, I want its intended contextual actions once in the toolbar area, so that a duplicate popup does not compete with the inline controls.
12. As a keyboard designer, I want changing or reselecting scope to retain the correct selected state and action availability, so that the controls follow my selection consistently.
13. As a keyboard designer with nothing selected, I want the scope control to explain what to select, so that removing duplicate actions does not remove useful guidance.
14. As a keyboard designer selecting an object after opening that guidance, I want the hint panel to close when inline actions become available, so that it does not cover the controls I now need.
15. As a keyboard designer using a keyboard, I want scope choices, guidance and actions to remain reachable with appropriate focus behavior, so that recovery preserves accessible interaction.
16. As a keyboard designer, I want approved canvas handles and Command palette actions preserved, so that removing competing scope popovers does not remove other intended interaction methods.
17. As a split-keyboard designer, I want the reported mirrored MCU orientation investigated in the actual affected project and build, so that a repair addresses what I observed.
18. As a split-keyboard designer, I want stored, evaluated and displayed controller poses to agree with the accepted mirroring contract, so that each representation describes the intended orientation.
19. As a split-keyboard designer, I want controller orientation to survive saving and reopening, so that the apparent correction is durable.
20. As a keyboard designer exporting affected geometry, I want the relevant output checked against the accepted poses, so that an orientation repair also covers downstream geometry when applicable.
21. As a keyboard designer, I want Snap and the shared disclosure behavior to open and close deliberately, so that the interface does not jump abruptly.
22. As a user requesting reduced motion, I want disclosure transitions to respect that preference while remaining functional, so that recovery does not impose unnecessary movement.
23. As a keyboard designer editing a draft inside a retained disclosure, I want closing and reopening it to preserve that draft, so that presentation changes do not discard my input.
24. As a keyboard user, I want collapsed disclosure content excluded from interaction while focus remains usable, so that visually hidden controls do not interrupt navigation.
25. As a keyboard designer, I want every affected chevron to show its own section's expanded state, so that nested sections do not display misleading arrows.
26. As a keyboard designer dragging an object or transform handle, I want the actual gesture to preview, commit and save the intended edit, so that the canvas and durable document agree.
27. As a keyboard designer undoing a drag, I want the prior document and canvas restored and the Undo result saved, so that Undo reverses the complete edit.
28. As a keyboard designer reopening a saved project, I want the saved edited or undone geometry restored correctly, so that reload verifies persistence rather than only a save indicator.
29. As a user of different Themes, Appearances and screen widths, I want populated Workspaces and their controls reviewed in those combinations, so that acceptance covers realistic use.
30. As a maintainer, I want each repair accompanied by a regression test that fails on the broken behavior, so that future reconciliation cannot silently reintroduce it.
31. As a maintainer, I want design and ticket records updated with the relevant change, so that future work follows the recovered contract and knows which acceptance gates remain open.
32. As a product owner, I want source presence, focused checks, full gates, independent review and human acceptance reported separately, so that completion claims match the evidence.
33. As a product owner, I want this recovery delivered in bounded, verifiable slices, so that known UI corrections can progress without guessing the MCU cause or re-merging entire histories.

## Implementation Decisions

- **Preserve the current architecture and controls.** Retain the shared UI layer,
  revised Header/Docks, Command palette, Toasts, Themes and Appearance handling.
  Preserve saved widths and the deliberate legacy auto-hide-to-pinned behavior.
  Recover interaction contracts without restoring retired styles or wrappers.
- **Restore placement at its presentation boundary (A2/A4/A8).** The Layout
  workspace and component Inspector expose the breadcrumb above fields and
  Relationships beside them. Retire the Relations tab without losing applicable
  facts, normal selection semantics, split focus or transient reference feedback.
  Navigation remains view state, not an accepted edit.
- **Restore scope transitions (A3/A4/A8).** Matrix, Row and Column controls select
  scope without opening a competing action panel when inline tools are available.
  Keep no-selection guidance; close it when selection supplies the inline tools.
  Preserve selected state, keyboard reach and focus across those transitions.
  Duplication concerns the competing toolbar/popover surfaces, not approved
  canvas handles or Command palette access.
- **Diagnose before prescribing an MCU change (A1).** Use `diagnosing-bugs` to
  establish and run a reproducible, red-capable command for the user's symptom,
  then minimize it before testing causes. Capture the loaded candidate identity,
  project, part definition kind, mirror linkage, stored pose, evaluated pose and
  rendered transform. The existing controller heading exception is integrated;
  no missing cherry-pick, stale-save cause or renderer defect is assumed. If the
  actual project/build cannot be obtained or reproduced, record the blocker and
  needed input rather than invent a fix or close the symptom.
- **Respect pose ownership.** Preserve accepted half-frame storage, evaluated
  board-space reads, mirroring and explicit local exceptions. Compare poses in
  the appropriate frame. Change the diagnosed layer only; introduce no speculative
  document schema, export contract or public API redesign.
- **Recover motion at the shared disclosure boundary (A6/A4/A8).** Support actual
  opening and closing transitions, reduced motion and the content lifetime needed
  for those transitions. Retain draft-owning children where required; ensure
  closed content is not interactable. Do not presume every Workspace previously
  promised the same animation; assess callers against their accepted design.
- **Correct chevron binding independently (A7/A4/A8).** Use the pinned primitive's
  actual disclosure-state contract at the affected Layout call sites. Each arrow
  follows its own disclosure. Other primitives have different state contracts;
  this is not a global selector replacement or a dependency upgrade.
- **Repair the existing integration harness (A5).** Bring the real-page drag
  workflow into agreement with current controls and save status, diagnosing its
  setup/focus failures as necessary. Preserve trusted gestures, document and
  scene comparisons, durable persistence and provenance checks. Add explicit
  reload verification; do not replace failing assertions with weaker signals.
- **Update records with behavior (A8).** Reconcile placement documentation,
  disclosure comments, controller heading guidance and acceptance records when
  the relevant behavior is addressed. Preserve historical results with their
  source and scope. A scoped-resolved ticket does not certify final acceptance.
- **Bound the work.** A4 and A8 accompany the relevant repair rather than being
  deferred to a final cleanup. Keep MCU diagnosis, known placement/scope recovery,
  shared motion, chevron binding and final acceptance distinguishable. Actual
  implementation slices and blocking edges are decided in `to-tickets`.

## Testing Decisions

The confirmed approach prefers existing production boundaries. A good test drives
the real user action and asserts its observable effect, with a fixture capable of
exposing the regression. Use accessible roles/names for controls and appropriate
semantic scene observations for canvas behavior. A renamed test, changed class
string, non-crashing render or unrelated green subset is not recovery evidence.

- **Primary presentation seam: mounted production Layout.** Extend the existing
  relation-highlight, component-Inspector, split-focus, field-preview and Layout
  toolbar coverage. Assert the breadcrumb and applicable facts are available with
  fields without opening Relations, and that navigation changes selection without
  document or history writes. Exercise hover and keyboard reference highlighting,
  clearing transitions and distinction from amber previews and persistent anchors.
  Hidden references retain navigation and explain their layer; removed targets
  retain an explanation without a dead action.
- **Make split-focus proof meaningful.** Reuse the existing populated split-editor
  fixture patterns, establish an observable focused half, then navigate breadcrumb
  and relationship references and assert it remains unchanged. The current
  breadcrumb test's comparison of optional labels on a nonsplit fixture cannot
  establish this contract. Missing preconditions must fail the test.
- **Cover every requested scope and transition.** Use existing toolbar fixtures
  for focused action-surface assertions and production Layout integration for
  selection/focus behavior. Cover Matrix, Row and Column, scope changes and
  reselection, empty selection guidance, selection arriving while that hint is
  open, and keyboard operation. Require the competing panel to close and the
  intended actions to remain available once, rather than merely hiding one label.
- **Shared disclosure seam plus production callers.** Extend mounted shared-control
  coverage and representative Layout callers, including Snap and nested
  disclosures. Check rendered expansion/collapse and computed arrow orientation,
  not selector text alone. Exercise normal and reduced motion, closing content
  lifetime, retained drafts, keyboard focus and closed-content exclusion. Motion
  and arrow-state assertions are separate so one passing does not mask the other.
- **Real native workflows where relevant.** Follow ADR-0021: drive Runtime through
  the real Session and Core with controlled in-memory persistence and existing
  delay/failure adapters. These seams cover settlement, persistence, Undo and any
  diagnosed domain behavior; they are not restricted to Core-only defects. Do not
  invent a second application implementation or duplicate every mounted test.
  Native results do not prove browser input, rendering, worker or storage behavior.
- **MCU regression at the diagnosed seam.** Capture the actual symptom with a
  red-capable reproduction before choosing a fix. Preserve its minimized fixture
  as a regression where the real path can be exercised, then rerun the original
  scenario. Compare both halves' evaluated orientations with their rendering,
  save/reload and relevant exported geometry if affected. Existing unit coverage
  of the controller heading helper alone does not close A1.
- **Packaged-app gesture and persistence proof.** Reuse the existing Rust browser
  drag harness and supported runner. Prove actual drag preview/commit, durable
  storage, exact Undo restoration of document and scene, and saving the Undo as
  the appropriate new revision. Explicitly reopen/reload to verify saved geometry
  after an edit and after Undo, using separate scenarios if necessary so this does
  not invent an Undo-history-across-reload requirement. A save badge alone and the
  existing unrelated Case fixture reload are insufficient. Bind evidence to the
  tested source, packaged artifact and browser; diagnose obsolete selectors and
  setup failures without declaring an unobserved product cause.
- **RED/GREEN with the repair.** Restored or strengthened assertions must fail for
  the broken behavior and pass after its correction. Record the command, failure
  and passing result against the applicable source. Keep existing coverage,
  meaningful document/scene comparisons and test-name accounting intact. Use the
  repository mutation runner for mutation checks where the ticket requires them.
- **Final gates and review.** Run the full default checks, including the full
  browser gate, on the final reconciled source. Include the existing accessibility
  floor and strict old-CSS check; report their actual scope. Quick runs, selected
  crate launches and retained earlier logs do not replace this evidence. Obtain
  independent spec+standards review covering the reconciled migration candidate,
  including inherited integration behavior, not only the final repair diff.
  Resolve findings or record an explicit disposition without relabeling an open
  gate as passed.
- **Populated-workspace human acceptance.** Review Layout, Case, Parts, PCB,
  Keymap, Keycaps, Library, the project menu and Export in Original/shadcn Themes,
  Light/Dark Appearances and desktop/mid/narrow widths. Use the existing review
  sizes 1440×900, 1024×900 and 390×844. Include real Case geometry and a populated
  3D canvas, split selection/reference states, spacing ghosts, anchor badges,
  editable fields, menus, dialogs and long Inspector content. Inspect contrast,
  geometry visibility, overflow, scrolling, focus, selected/disabled states and
  touch targets. Record the project, combination, screenshot and result. Automated
  captures are supporting evidence; the user must approve the actual review.

Final acceptance remains open until the specified behaviors, A1 diagnosis and
verified outcome, full-source checks, independent review and populated human
review have evidence. A blocked reproduction or gate remains explicitly open;
this planning ticket provides no waiver.

## Out of Scope

- Product repairs or implementation-ticket creation during this `to-spec` step.
- Wholesale merging or replaying original branches; original histories contain
  squashed equivalents, obsolete harnesses and retired styles.
- Reverting the shared control migration, restoring auto-hide, redesigning the
  shell, or adding a new test framework or speculative public interfaces.
- Reimplementing source-present engine work, expanding the accepted Ergogen
  compatibility subset, or claiming new Detach, export or Workspace defects
  without evidence.
- Broad retirement of all opt-in browser scripts. Preserve their recorded
  follow-up debt; recover the missing unique acceptance coverage needed here.
- Changing unrelated files, the original `ci-wall-time-plan` checkout, or the
  detached `dev-live-review` checkout; creating another worktree or switching this
  branch; pushing, merging to `dev`, or replacing the user's live preview.
- Committing local agent skills, lock metadata or T3 setup configuration. Their
  local availability was repaired separately and is not product acceptance.

## Further Notes

### Provenance and current evidence

- Source candidate: `1fda4f48a964568b98dde9c82556350c4bc50911`. Integrated behavior:
  `510861b11`. Problematic reconciliation: `c79de4708`. Approved prototype:
  `prototype/split-board-editing` at `ef1c8a008`, variant B.
- The [original audit](../../../research/2026-11-06-dev-ui-integration-audit.md)
  is retained unchanged. It is targeted history/source inspection, not a fresh
  build, browser run or exhaustive review of the migration's changed files.
  Screenshots establish reported symptoms without proving loaded artifact or
  document identity. This specification adds no new execution evidence.
- The source recheck confirmed the changed Relations and scope assertions, the
  nonsplit breadcrumb focus fixture, the obsolete drag save-status selector,
  the absence of drag/Undo reload proof, and the pinned disclosure attribute
  mismatch. These refinements are included above without rewriting the audit.
- Core/application have no source diff from the integrated reference to the
  candidate. Anchors and the accepted Ergogen parity kernel, half frames,
  adaptive spacing, magnetic snap, evaluated-coordinate Inspector moves and the
  Parts assembly layout correction are source-present. Field previews, split
  focus, spacing ghosts and anchor/reference feedback are also present. These
  are integrated work, not newly browser-verified results.
- No additional high-confidence data-editing regression was established by the
  audit's targeted sampling of other Workspaces. Their populated acceptance
  remains open; sampling is not a clean bill of health.

### Approved contracts and record reconciliation

- [Placement ticket 15](../../split-board-editing/issues/15-placement-breadcrumb-retire-relations.md)
  specifies the breadcrumb, beside-field Relationships, selection/focus behavior
  and violet highlighting. [The split-board plan](../../split-board-editing/map.md)
  retains the approved prototype and related interaction decisions.
- [The UI migration spec](../../rust-ui-migration/spec.md),
  [ADR-0034](../../../adr/0034-adopt-rust-ui-components-with-two-themes.md),
  [ADR-0035](../../../adr/0035-rebuild-the-workbench-shell-on-rust-ui-patterns.md),
  [DESIGN](../../../../DESIGN.md) and the [glossary](../../../../GLOSSARY.md)
  govern the retained shared controls, shell and terminology.
- [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)
  still describes the generic independent-part heading rule broadly. The later
  [controller/half-frame ticket 12](../../split-board-editing/issues/12-half-frame-parts-and-mirrored-controller.md)
  and integrated controller exception differ from that blanket wording.
  Reconcile this documentation under A8 with the diagnosed A1 path; do not
  silently apply the generic rule to controllers or revert the integrated fix.
  Preserve [ADR-0039](../../../adr/0039-half-frame-parts-mirrored-controller.md)
  and [ADR-0040](../../../adr/0040-part-anchoring-column-vocabulary.md)'s pose and
  anchor ownership.
- [ADR-0021](../../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md)
  governs the native/mounted test boundary. [Migration gate 026](../../rust-ui-build/issues/026-merge-gate.md)
  retains historical focused results and the still-open full/human/review gates.
  This specification neither rewrites its historical results nor grants merge
  or push authorization.

### Planning handoff

The worktree is already bound to `fix/ui-integration-recovery`; its `Owner:`
above records the requested ownership. The user's 2026-11-06 reinvocation of
`to-spec` against the reviewed summary carries the corrected test seams into
this specification. `ready-for-agent` means ready for the next planning step,
not ready to bypass it and implement the whole recovery.

Next: run `to-tickets`, present the bounded behavior slices and actual blocking
edges for confirmation, then publish those implementation tickets. Attach A4
and A8 to the relevant slices, retain A1's diagnostic uncertainty, keep A6/A7
separate, and reserve a final acceptance step covering the reconciled candidate.
