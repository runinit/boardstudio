---
type: epic
title: "Layout canvas controls"
parent: initiative-layout-controls-polish
sources:
  - docs/plans/initiative-layout-controls-polish/initiative-layout-controls-polish.md
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
covers: ["R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10"]
after: []
refined: true
risk: high
---

# Layout canvas controls

## Description

Preserve and complete the approved variant D Layout controls outcome through exactly the eight existing tickets. The frozen source companions retain detailed acceptance, decisions and progress.

## Outcome

Layout canvas controls are compact, screen-sized and consistent with documented color roles, and users receive usable part manipulation, moving re-anchor and canvas pick feedback.

## Requirements

R1 (parent R1): The approved variant D defines the compact rail, anchor badge, fixed-size labels, feedback and pick visuals. Demonstrate the approved reference and preserve later decisions overriding its pose-preserving re-anchor behavior.

R2 (parent R2): Passive canvas labels and anchor chrome stay screen-sized at different zooms; the readout and accent, violet and amber roles match the design. Demonstrate zoom, selection/readout and theme behavior without changing document state.

R3 (parent R3): Existing tools use a screen-sized token rail with the specified geometry and hover/active feedback. Demonstrate retained drag, quick Offset, keyboard nudge and hidden-handles behavior.

R4 (parent R4): Picking a new part anchor keeps carried/local values and moves it from the new anchor in one Undo step, including mirror behavior and existing refusals. Demonstrate matrix re-anchor movement and unchanged pose-preserving Follow and Detach.

R5 (parent R5): Row and key Shift/Bend use existing placement edits and the rail, preserving row Offset, one Undo per commit, mover preview, hidden handles and Core mirror rules. Demonstrate tool lists and drags.

R6 (parent R6): A view-only value popup and accessible commit toast show current value and delta, follow interaction and expire without extra document revisions. Demonstrate dismissal, zero delta and timeout.

R7 (parent R7): Independent parts offer their applicable Move, Rotate, Spread and Stagger tools, feedback and one-Undo edits; detached and member behavior follows the source. Demonstrate mirrored movement, half focus, existing part drag and the unchanged upper strip. Retain the source's existing progress and outstanding gates.

R8 (parent R8): Anchored matrix and part rails offer canvas Anchor pick, candidate and loop handling, amber move preview using Core evaluation, one-Undo commit, toast and cancellation. Demonstrate both halves, including the dimmed half.

R9 (parent R9): Preserve the eight original titles and actual Blocked by edges. 01, 02, 03, 05 and 06 are resolved; 04, 07 and 08 are ready. Demonstrate status translation without treating historical partial evidence as a newly passed gate.

R10 (parent R10): Each ticket retains its own RED/GREEN, applicable native, mounted and live evidence, source-specific reviews, full check and ordered project closure obligations. Demonstrate the named behavior and record actual evidence and unmet gates.

## Constraints

- Source companions are the detailed behavioral and testing contract; exact original Blocked by headers govern dependencies. Shared files require coordination without invented dependencies.
- Variant D is superseded where the map explicitly requires re-anchor to move; Follow-subset changes, Detach and drop auto-anchor retain pose. Only 04 changes public Core anchor semantics; its source explicitly requires combined Spec + Standards review. Current closure for every implementation ticket follows the review gate below.
- Compact hit areas remain undecided; retain row Offset by default and leave the upper strip unchanged pending its live confirmation. No implementation is authorized by this migration.

## Done when

- 01's approved variant D, later map decisions and precise sizing and color roles remain traceable. 02 demonstrates screen-size labels, 7 px ring and selection readout at two zooms, unchanged mounted suites, live light/dark three-zoom comparison and DESIGN.md update; 03 demonstrates rail geometry, existing handles, hover/active/leave without revision, live drags including Ctrl Offset and nudge and DESIGN.md; 05 demonstrates Row/Key tool lists, drag mapping, amber preview, one Undo, hidden handles and mirror rules; 06 demonstrates reducer choose/drag/commit/zero/Esc/expiry, mounted docking/follow/toast with no extra revision and polite focus-preserving announcement. Each source requires full `python3 scripts/check.py`; retain recorded partial or waived historical results honestly.
- 04 follows RED/GREEN at the public Core interface: unchanged carried/local values and moved pose, one Undo and restore; mirrored pair and matrix movement with stored frame; unchanged Follow and Detach tests; locked, driven, member, cross-layout, missing key/mirror and loop refusals; mutation restoring `solve_for_pose` killed; boundary transcript re-blessed only if downstream Pick hashes change and diff explained; mounted Inspector movement with values retained; ADR-0040 and applicable glossary/help corrected; one fresh combined Spec + Standards review adjudicated in the Answer, followed by full `python3 scripts/check.py` as the integrated gate.
- 07 demonstrates anchored, detached, member and locked native tool lists; mounted Move, Rotate and Stagger, one Undo each, mirror and dimmed-half focus, popup/toast and selected/unselected outline roles; confirms the unchanged upper strip in the live pass; existing part-drag browser smoke and full `python3 scripts/check.py` pass. Existing 07 Answer records substantial implementation and scoped tests but its header remains ready and its smoke/full gate unchecked.
- 08 demonstrates native matrix, anchored/detached part candidates and loop exclusion; mounted enter, hover preview matching committed pose, click with one revision/movement/toast, Undo and Esc without revision; live split-board pass on both halves including dimmed; DESIGN.md update and full `python3 scripts/check.py`. Use Core evaluation for preview and retain loop refusal.
- For each implementation closure, map requirements to proof and distinguish reused from newly run evidence; establish RED or an identified proof gap, fix, run affected checks (including applicable lint), run named boundary mutations for proof-gap rows through `scripts/mutation-check.py --locked`, run one fresh reviewer covering Spec and Standards, adjudicate findings, then run the integrated ticket gate. Source disclosures of RED/GREEN and diff-only review remain historical evidence, not a replacement for this current closure order. Record HEAD, commands/results, unmet criteria, cleanup and follow-ups. Use native-baseline updates only for deliberate test renames. Historical resolved status alone does not prove all original gates passed.

## Boundaries

- 01 establishes the reference; 02/03 render chrome and existing rail; 04 owns Core re-anchor semantics; 05/06 add Shift/Bend and feedback; 07 adds part rail; 08 adds canvas Anchor pick. Actual prerequisites are entry `after` edges.
- Out of scope: compact touch redesign without decision, Follow/Detach movement, extra refactor stories and copying or changing original progress.

## References

- [Design](../../../../../DESIGN.md): adopted canvas tokens, color roles and control guidance.

- [Parent requirements](../../initiative-layout-controls-polish.md#requirements) and [frozen map](../../../layout-controls-polish/map.md).
- [01](../../../layout-controls-polish/issues/01-layout-controls-iconography-mockup.md), [02](../../../layout-controls-polish/issues/02-screen-space-canvas-chrome.md), [03](../../../layout-controls-polish/issues/03-handle-rail.md), [04](../../../layout-controls-polish/issues/04-re-anchor-moves.md), [05](../../../layout-controls-polish/issues/05-row-and-key-shift-bend-handles.md), [06](../../../layout-controls-polish/issues/06-value-popup-and-commit-toast.md), [07](../../../layout-controls-polish/issues/07-part-rail.md), [08](../../../layout-controls-polish/issues/08-canvas-anchor-pick.md): source requirements, acceptance and original evidence.
- [Issue tracker](../../../../agents/issue-tracker.md#building-and-verifying-a-ticket), proof map, slice boundary, review reports and completion record.

## Notes

- Decision: 2026-11-15 — User approved this one epic and exact original identities, statuses and dependency headers. 02 and 03 cite the 01 decision but are blocked by none. No added stories or platform setup.
- Decision: 2026-11-15 — Preserve 01 as the completed visual reference, not an invented end-to-end implementation tracer. The original controls path progresses through 02/03, 05/06 and 07 to 08, with 04 providing moving re-anchor semantics; only the source dependency headers define ordering. No new tracer or human setup story is added.
- Decision: 2026-11-15 — The default refactor-sweep story is declined under the user's approved exact-source migration scope; the eight original tickets are the complete breakdown.
- Review disposition: 2026-11-15 — Corrected closure order, current fresh-review obligation, design companions, slicing decisions and existing human-pass descriptions. The initiative follows the template by omitting epic-only `refined` metadata. Risk labels are provisional migration assessments, not new user-approved priority decisions; agreement and business-impact rationale remain a refinement follow-up before implementation, without changing source readiness.
- Resolved 02, 03, 05 and 06 retain their original Answer disclosures of partial, pre-existing failure or waived verification; do not silently promote them to full green. 07 remains ready despite substantial source-recorded progress.
