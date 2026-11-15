---
type: epic
title: "Complete Layout workbench"
parent: initiative-toolbar-led-workbench
sources:
  - docs/plans/initiative-toolbar-led-workbench/initiative-toolbar-led-workbench.md
companions:
  - docs/plans/initiative-toolbar-led-workbench/architecture-toolbar-led-workbench/architecture-toolbar-led-workbench.md
  - docs/plans/toolbar-led-workbench/map.md
  - docs/plans/toolbar-led-workbench/delivery.md
  - docs/plans/toolbar-led-workbench/inventory.md
  - docs/plans/toolbar-led-workbench/verification.md
covers: ["R1", "R2", "R3", "R4", "R6", "R7", "R8", "R9", "R11"]
after: []
status: in-progress
risk: high
---

# Complete Layout workbench

## Workflow

Installed BMad skills govern orchestration, ticketing, review and verification. Legacy sources and historical notes retain behavioral requirements and evidence, not their proof-map, mutation, fixed-review or ordered-closure protocols. Functional acceptance and explicit human/phase approvals remain unchanged.

## Description

Deliver the complete M1 Layout redesign in the actual app by integrating the shared host, the Layout tools, Objects and Inspect, the retained Add/setup/placement/legacy workflows and the global header. This epic preserves domain ownership and ends at the user's hands-on review; it does not authorize M2/M3 implementation, production rollout or a merge to main.

## Outcome

BoardStudio users can complete the Layout workbench's real tasks with the new toolbar-led shell and retained workflows, with M1 evidence ready for hands-on review.

## Requirements

R1 (parent R1): Deliver the shared host and its Layout integration plus the global topbar; demonstrate retained commands and target-aware navigation through mixed old/new shells. The five remaining workspace integrations belong to M2; final six-workspace acceptance belongs to M3.

R2 (parent R2): M1 delivers a complete, usable Layout redesign in the actual app, including the single 42px accordion with Select, Transform, Align, Snap and View, Objects and unpinned Inspect utilities, and retained Add/setup/placement/legacy workflows. Demonstrate the production consumer and its real styling, not an empty host, mockup or mocked controls.

R3 (parent R3): For the shared host, Layout editors/tasks and M1 mixed-shell routes, panel and task visibility follow one current workspace authority; selection and camera behavior remain domain-owned; hiding preserves the existing draft, error, disclosure and live update-safety registration lifetimes without synthetic commits, stale-owner replay or silent loss. Demonstrate captured-owner outcomes on the specified close, switch, workspace-return and session-replacement paths.

R4 (parent R4): The Layout migration retains every listed tool, navigation and workflow capability, including active-outline selection, half focus, Inspect lifecycle/findings, Add/setup/placement and legacy geometry-script access under its existing safety. Demonstrate each inventory row at its owning ticket boundary before old dock behavior is retired.

R6 (parent R6): Preserve M1's identities and prerequisites in order 001, 009, 002, 003, 004, 006, 005, including 006 before 005. Deliver the M1 candidate and stop for user hands-on review; M2 and M3 phase execution belongs to the later epics.

R7 (parent R7): Record concise evidence for accepted behavior in the BMad ticket plan, distinguishing executed, reused and unrun checks; no separate proof-map protocol is required.

R8 (parent R8): Use installed BMad skills for implementation, review and verification. Explicit human and phase approvals remain required; report unmet criteria and follow-ups without waiving approval.

R9 (parent R9): M1 acceptance includes the named Layout and global-header 007 rows, the mixed-shell integration routes, a stable integrated candidate gate and hands-on keyboard/visual review, with evidence and limitations recorded. Stop for the user's hands-on review before M2; do not claim M1 proves future adapter acceptance.

R11 (parent R11): Demonstrate the architecture's responsive, keyboard, touch/trackpad, contrast and reduced-motion literals for Layout and the global header using the named M1 acceptance rows. Record real-app measurements and limitations; M2 owns adapter-specific behavior and M3 owns final six-workspace acceptance.

## Constraints

- Preserve existing workspace/domain ownership, geometry, selection, edit and export behavior; shared chrome does not create a universal object model.
- Keep M1 as a real-app candidate and stop for hands-on review; this epic does not authorize production rollout, main merge, M2 or M3.
- Create only the seven approved legacy identities assigned to this epic; do not add refactor-sweep tickets or duplicate legacy receipts.

## Done when

- The resolved 001 transfer record and all six remaining M1 entries have their source-linked plans and approved status transitions.
- The production Layout consumer, all retained capabilities, owner/draft safeguards and named M1 acceptance rows meet their accepted behavior with results and limitations recorded.
- Follow installed BMad skills for implementation, review and verification; explicit user and phase approvals remain required.
- A stable integrated M1 candidate is ready for hands-on keyboard/visual review, then implementation stops before M2 until the user approves.

## Boundaries

- Touch point: Shared panel/focus/lifetime host and global reveal commands — 009 and 006; domain owners remain authoritative.
- Touch point: Layout toolbar/actions, Objects, Inspect, Add/setup/placement and legacy task access — 002–005.
- Touch point: Planning/transfer and M1 acceptance evidence — 001 and the Layout/global-header portion of 007 as scoped by the delivery contract.
- Must not change: PCB/Keymap/Keycaps/Case/Parts implementations, old dock retirement, geometry, saved formats, manufacturing/export policy, or unrelated shared menus.

Trace: 001 transfer record → 009 shared host → 002 Layout toolbar → 003/004 Layout panels and Objects → 006 global header → 005 retained workflows → stable integrated M1 candidate → hands-on stop.

## References

- parent — [initiative Requirements](../../initiative-toolbar-led-workbench.md#requirements)
- architecture — [workbench architecture companion](../../architecture-toolbar-led-workbench/architecture-toolbar-led-workbench.md)
- map — `docs/plans/toolbar-led-workbench/map.md`, Tickets and dependency order; Risks and deliberate limits
- delivery — `docs/plans/toolbar-led-workbench/delivery.md`, Milestones and authority; M1 acceptance and proof ownership; Coordination, verification and stop
- inventory — `docs/plans/toolbar-led-workbench/inventory.md`, Shared shell; Layout tools; Layout Objects and Inspector; Overflow routing
- verification — `docs/plans/toolbar-led-workbench/verification.md`, M1 proof ownership; Closure and limits
- legacy tickets — [001](../../../toolbar-led-workbench/issues/001-reconcile-design-adr-scope.md), [009](../../../toolbar-led-workbench/issues/009-shared-toolbar-panel-host.md), [002](../../../toolbar-led-workbench/issues/002-toolbar-accordion-label-fitting.md), [003](../../../toolbar-led-workbench/issues/003-inspect-popout.md), [004](../../../toolbar-led-workbench/issues/004-objects-popover.md), [006](../../../toolbar-led-workbench/issues/006-topbar-simplification.md), [005](../../../toolbar-led-workbench/issues/005-retain-objects-capabilities.md)

## Notes

- Decision: 2026-11-15 — Preserve resolved 001 as the opening transfer record; the working tracer is 009's host through 002's real styled Layout consumer. Keep 003/004 after 002, 006 after 009 and before 005, and 005 after both panels and header routing. These are build order and actual prerequisites, not additional serial dependencies.
- Decision: 2026-11-15 — The approved migration retains exactly 15 existing tickets. Do not add a refactor sweep; retain cleanup and acceptance obligations within the existing ticket boundaries.

- Decision: 2026-11-15 — User approved this epic grouping. Original ticket identities and source receipts remain in the frozen plan; child entries link to them without copying their progress logs.
- Note: 002's width-band replacement and 280ms entry/immediate close are the approved architecture target, with its remaining proof, review and acceptance gates still open.
