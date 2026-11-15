---
type: initiative
title: "Layout controls polish"
parent: none
sources:
  - docs/plans/layout-controls-polish/map.md
  - docs/plans/layout-controls-polish/issues/01-layout-controls-iconography-mockup.md
  - docs/plans/layout-controls-polish/issues/02-screen-space-canvas-chrome.md
  - docs/plans/layout-controls-polish/issues/03-handle-rail.md
  - docs/plans/layout-controls-polish/issues/04-re-anchor-moves.md
  - docs/plans/layout-controls-polish/issues/05-row-and-key-shift-bend-handles.md
  - docs/plans/layout-controls-polish/issues/06-value-popup-and-commit-toast.md
  - docs/plans/layout-controls-polish/issues/07-part-rail.md
  - docs/plans/layout-controls-polish/issues/08-canvas-anchor-pick.md
companions:
  - DESIGN.md
  - docs/plans/layout-controls-polish/issues/01-layout-controls-iconography-mockup.md
  - docs/plans/layout-controls-polish/issues/02-screen-space-canvas-chrome.md
  - docs/plans/layout-controls-polish/issues/03-handle-rail.md
  - docs/plans/layout-controls-polish/issues/04-re-anchor-moves.md
  - docs/plans/layout-controls-polish/issues/05-row-and-key-shift-bend-handles.md
  - docs/plans/layout-controls-polish/issues/06-value-popup-and-commit-toast.md
  - docs/plans/layout-controls-polish/issues/07-part-rail.md
  - docs/plans/layout-controls-polish/issues/08-canvas-anchor-pick.md
after: []
risk: high
---

# Layout controls polish

## Description

Migrate the existing Layout controls polish plan into one epic, preserving its eight ticket identities, statuses, actual dependency headers, source decisions and evidence in the frozen originals.

## Outcome

The Layout canvas has smaller, refined controls that fit the house toolbar, with fixed screen-size labels and accent, violet and amber used for their documented roles. Users can manipulate and re-anchor eligible items with clear feedback.

## Requirements

R1 (01 §Answer; map §Decisions so far): The approved variant D defines the compact rail, anchor badge, fixed-size labels, feedback and pick visuals. Demonstrate the approved reference and preserve later decisions overriding its pose-preserving re-anchor behavior.

R2 (02 §Acceptance): Passive canvas labels and anchor chrome stay screen-sized at different zooms; the readout and accent, violet and amber roles match the design. Demonstrate zoom, selection/readout and theme behavior without changing document state.

R3 (03 §Acceptance): Existing tools use a screen-sized token rail with the specified geometry and hover/active feedback. Demonstrate retained drag, quick Offset, keyboard nudge and hidden-handles behavior.

R4 (04 §Acceptance; map §Decisions so far): Picking a new part anchor keeps carried/local values and moves it from the new anchor in one Undo step, including mirror behavior and existing refusals. Demonstrate matrix re-anchor movement and unchanged pose-preserving Follow and Detach.

R5 (05 §Acceptance): Row and key Shift/Bend use existing placement edits and the rail, preserving row Offset, one Undo per commit, mover preview, hidden handles and Core mirror rules. Demonstrate tool lists and drags.

R6 (06 §Acceptance): A view-only value popup and accessible commit toast show current value and delta, follow interaction and expire without extra document revisions. Demonstrate dismissal, zero delta and timeout.

R7 (07 §Acceptance; §Answer): Independent parts offer their applicable Move, Rotate, Spread and Stagger tools, feedback and one-Undo edits; detached and member behavior follows the source. Demonstrate mirrored movement, half focus, existing part drag and the unchanged upper strip. Retain the source's existing progress and outstanding gates.

R8 (08 §Acceptance): Anchored matrix and part rails offer canvas Anchor pick, candidate and loop handling, amber move preview using Core evaluation, one-Undo commit, toast and cancellation. Demonstrate both halves, including the dimmed half.

R9 (map §Tickets; issue headers): Preserve the eight original titles and actual Blocked by edges. 01, 02, 03, 05 and 06 are resolved; 04, 07 and 08 are ready. Demonstrate status translation without treating historical partial evidence as a newly passed gate.

R10 (02–08 §Acceptance and review headers): Each ticket retains its own RED/GREEN, applicable native, mounted and live evidence, source-specific reviews, full check and ordered project closure obligations. Demonstrate the named behavior and record actual evidence and unmet gates.

## Constraints

- Preserve DESIGN.md tokens and the approved variant D reference `mockup/layout-controls-iconography` at `e91865f43`, `?variant=D`; later map decisions supersede conflicting mockup behavior. (01; map §Decisions so far)
- Only 04 changes public Core anchor semantics; its source explicitly requires combined Spec + Standards review. Current implementation closure for every ticket also follows the project review-before-integrated-gate protocol. Follow-subset changes, Detach and drop auto-anchor preserve their existing pose behavior. (04)
- 03's compact hit areas remain undecided; 05 retains Offset by default; 07 leaves the upper strip unchanged by default and confirms it in the live pass. (03, 05, 07 §Carried point)
- This approved migration changes documentation only; original plan files and progress stay in place, and no new feature or refactor ticket is added. No source architecture exists to copy.

## Done when

- The approved reference and all eight source-specific behavior and verification obligations are preserved under the epic and entries, with completed evidence accurately distinguished from pending checks.
- Re-anchor, part rail and canvas pick meet their source acceptance and closure gates, including 04's combined review, 07's part-drag smoke and 08's live split-board pass.
- Original dependencies and statuses are retained, and each implementation completion records proof, commands/results, unmet criteria and follow-ups under the project closure protocol.

## Boundaries

- The single epic owns the Layout canvas chrome, controls, feedback, part rail, re-anchor semantics and canvas pick covered by 01–08.
- Out of scope: feature implementation in this migration, compact touch redesign without a decision, altering Follow/Detach behavior, additional refactor stories and rewriting original plan evidence.

## References

- [Design](../../../DESIGN.md): adopted canvas tokens, color roles and control guidance.

- [Frozen map](../layout-controls-polish/map.md), destination, ticket graph and decisions.
- [01 mockup](../layout-controls-polish/issues/01-layout-controls-iconography-mockup.md); [02 chrome](../layout-controls-polish/issues/02-screen-space-canvas-chrome.md); [03 rail](../layout-controls-polish/issues/03-handle-rail.md); [04 re-anchor](../layout-controls-polish/issues/04-re-anchor-moves.md).
- [05 Shift/Bend](../layout-controls-polish/issues/05-row-and-key-shift-bend-handles.md); [06 feedback](../layout-controls-polish/issues/06-value-popup-and-commit-toast.md); [07 part rail](../layout-controls-polish/issues/07-part-rail.md); [08 pick](../layout-controls-polish/issues/08-canvas-anchor-pick.md). These adopted companions retain detailed requirements, acceptance and original progress.
- [Issue tracker](../../agents/issue-tracker.md#building-and-verifying-a-ticket) and [AI documentation](../../agents/ai-documentation.md).

## Notes

- Decision: 2026-11-15 — User approved one epic with exactly 01–08 and the original status and dependency headers. The 01 decision referenced by 02 and 03 is not a Blocked by edge.
- Decision: 2026-11-15 — Resolved becomes done; ready becomes ready-for-dev. 07 has documented implementation progress despite its ready header; do not represent it as untouched or accepted.
- Source evidence for resolved 02, 03, 05 and 06 includes partial, pre-existing failure or waived gate disclosures; done preserves the source disposition and does not claim a clean full gate.
