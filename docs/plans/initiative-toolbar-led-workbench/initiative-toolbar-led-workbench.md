---
type: initiative
title: "Toolbar-led workbench"
parent: none
status: in-progress
sources:
  - docs/plans/toolbar-led-workbench/map.md
  - docs/plans/toolbar-led-workbench/architecture.md
  - docs/plans/toolbar-led-workbench/inventory.md
  - docs/plans/toolbar-led-workbench/delivery.md
  - docs/plans/toolbar-led-workbench/verification.md
companions:
  - docs/plans/toolbar-led-workbench/map.md
  - docs/plans/toolbar-led-workbench/architecture.md
  - docs/plans/toolbar-led-workbench/inventory.md
  - docs/plans/toolbar-led-workbench/delivery.md
  - docs/plans/toolbar-led-workbench/verification.md
  - docs/plans/initiative-toolbar-led-workbench/architecture-toolbar-led-workbench/architecture-toolbar-led-workbench.md
after: []
risk: high
---

# Toolbar-led workbench

## Workflow

Installed BMad skills govern orchestration, ticketing, review and verification. Legacy sources and historical notes retain behavioral requirements and evidence, not their proof-map, mutation, fixed-review or ordered-closure protocols. Functional acceptance and explicit human/phase approvals remain unchanged.

## Description

Give BoardStudio users a quieter, consistent workbench across Layout, PCB, Keymap, Keycaps, Case and Parts while preserving each workspace's domain-owned tools, selection, editing, setup, preview and export behavior. The work is staged: first deliver and review the complete Layout redesign in the real app, then adapt the other workspaces after explicit approval, and finally prove six-workspace parity before retiring the old dock paths. The frozen design direction and the current delivery limits are in the linked sources and architecture companion.

## Outcome

BoardStudio users can work across all six workspaces through shared, task-focused chrome while retaining the capabilities and domain behavior they rely on.

## Requirements

R1 (map.md §Destination and authority; inventory.md §Shared shell and global surfaces): Shared workbench chrome serves Layout, PCB, Keymap, Keycaps, Case and Parts, and the quieter topbar preserves the existing global actions; each workspace keeps its own tools, selection model, editors and edit admission. Show this in the implemented workspace compositions and preserved command/navigation behavior.

R2 (delivery.md §Milestones and authority; architecture.md §Shared module and adapters): M1 delivers a complete, usable Layout redesign in the actual app, including the single 42px accordion with Select, Transform, Align, Snap and View, Objects and unpinned Inspect utilities, and retained Add/setup/placement/legacy workflows. Demonstrate the production consumer and its real styling, not an empty host, mockup or mocked controls.

R3 (architecture.md §Panel state and transitions; architecture.md §Draft owners, not merely hidden DOM; delivery.md §M1 implementation defaults and deferred decisions): Panel and task visibility follow one current workspace authority; selection and camera behavior remain domain-owned; hiding preserves the existing draft, error, disclosure and live update-safety registration lifetimes without synthetic commits, stale-owner replay or silent loss. Demonstrate captured-owner outcomes on the specified close, switch, workspace-return and session-replacement paths.

R4 (inventory.md §Layout tools and canvas; inventory.md §Layout Objects and Inspector; inventory.md §Overflow routing proposal): The Layout migration retains every listed tool, navigation and workflow capability, including active-outline selection, half focus, Inspect lifecycle/findings, Add/setup/placement and legacy geometry-script access under its existing safety. Demonstrate each inventory row at its owning ticket boundary before old dock behavior is retired.

R5 (inventory.md §PCB adapter; inventory.md §Keymap adapter; inventory.md §Keycaps adapter; inventory.md §Case adapter; inventory.md §Parts adapter; architecture.md §Shared module and adapters): The later adapters retain their named workspace capabilities and owners without introducing a universal object model or fabricated Layout actions. Demonstrate each adapter's inventory dispositions and stale-owner/admission behavior after its phase is approved.

R6 (map.md §Tickets and dependency order; delivery.md §Milestones and authority): Delivery keeps the documented dependency order and phase boundaries: M1 includes 001 as the resolved planning/transfer record and 009, 002, 003, 004, 006 and 005, with 006 before 005; M2 contains 010–015; M3 contains 007/008. Use only the approved three-epic split and exact 15 legacy identities, preserving dependencies, and preserve the hands-on stop after M1 before M2 begins.

R7 (verification.md §Reuse first and keep stories distinct; verification.md §Mutation boundary): Record concise evidence for accepted behavior in the BMad ticket plan, distinguishing executed, reused and unrun checks; no separate proof-map protocol is required.

R8 (verification.md §Closure and limits): Use installed BMad skills for implementation, review and verification. Explicit human and phase approvals remain required; report unmet criteria and follow-ups without waiving approval.

R9 (delivery.md §M1 acceptance and proof ownership; delivery.md §Coordination, verification and stop; verification.md §Closure and limits): M1 acceptance includes the named Layout and global-header 007 rows, the mixed-shell integration routes, a stable integrated candidate gate and hands-on keyboard/visual review, with evidence and limitations recorded. Stop for the user's hands-on review before M2; do not claim M1 proves future adapter acceptance.

R10 (delivery.md §Milestones and authority; map.md §Tickets and dependency order): Old dock paths and related state are retired only after all six adapters have parity and M3 acceptance is complete; preserve unrelated preferences, project/Export navigation, save honesty, legacy geometry access and existing data/geometry/output semantics. Demonstrate parity and retained behavior before retirement.

R11 (architecture.md §Responsive and keyboard contract (proposed); architecture.md §Motion and layering; delivery.md §M1 acceptance and proof ownership; map.md §Decision round 1): Responsive, keyboard, touch/trackpad, contrast and reduced-motion behavior is demonstrated against the architecture's exact acceptance literals in the real app, including the named M1 Layout/global-header rows and the later all-workspace acceptance. Record actual measurements and limitations; frozen mockups do not establish production accessibility.

## Constraints

- M1 is authorized for coordination and implementation, then stops for hands-on review. M2, M3, production rollout and merge to main require later approval. (map.md §Destination and authority; delivery.md §Milestones and authority)
- The user approved the three-epic grouping on 2026-11-15. This migration creates only its exact 15 legacy ticket identities; M2 and M3 implementation remain gated by later phase approvals. Do not treat a ready label as permission to cross a phase gate. (map.md §Tickets and dependency order; delivery.md §Milestones and authority)
- Share chrome, not domain document models or edit authority; do not introduce a universal object/command registry or widen cross-crate interfaces without approval. (architecture.md §Shared module and adapters)
- Do not change geometry algorithms, saved formats, arithmetic, manufacturing readiness, link rules, CAD/export/fabrication claims, or unrelated scripting/menu behavior under this initiative. (map.md §Risks and deliberate limits; inventory.md §Overflow routing proposal)
- Accessibility, keyboard, touch, contrast, motion and responsive requirements remain acceptance criteria; the approved 42px toolbar does not waive other target-size or accessibility requirements. (architecture.md §Responsive and keyboard contract (proposed); architecture.md §Motion and layering)
- Preserve all inventory capabilities and owners. A missing live workflow blocks retirement; absence of a draft registration is not proof that a form is explicit-submit or safe to lose. (inventory.md §Completeness boundary; architecture.md §Draft owners, not merely hidden DOM)

## Done when

- The real M1 Layout candidate meets its owning ticket proofs, including built-style 42px toolbar behavior, retained workflows, draft/update safety, named Layout/global-header acceptance rows and stable integrated gate; its evidence and limitations are recorded for hands-on review.
- The user-approved split is reflected in the authorized epic/story planning documents; M2 implementation begins only after hands-on M1 review and the user's explicit approval.
- The approved M2 adapters preserve their full inventoried capabilities, domain ownership, stale-owner protections and named proof receipts.
- M3 demonstrates responsive, keyboard, contrast, motion, packaged and adapter integration acceptance across all six workspaces before the old dock paths and state influence are retired.

## Boundaries

Approved three-epic split (2026-11-15): Epic 1 — complete Layout (001, 009, 002, 003, 004, 006, 005); Epic 2 — remaining adapters (010, 011, 012, 013, 015, 014); Epic 3 — acceptance and retirement (007, 008). The split authorizes planning the exact 15 entries, not M2/M3 implementation or production rollout.

- Touch point: Shared shell, panel/focus/lifetime host and command routing — preserve typed workspace ownership and one current panel authority; owner: Epic 1, with adapter routing completed by Epic 2.
- Touch point: Layout tools, Objects, Inspect, setup/Add, placement and legacy entries — preserve all inventoried capabilities and existing owners; owner: Epic 1.
- Touch point: PCB, Keymap, Keycaps, Case and Parts adapters — preserve each domain's navigation, editors, previews, admissions and action semantics; owner: Epic 2.
- Touch point: Responsive, accessibility, keyboard, motion, packaged and adapter acceptance plus dock retirement — gated on completed adapters; owner: Epic 3.
- Must not change: Geometry, saved formats, edit/selection ownership, export/fabrication claims, unrelated preferences, or legacy scripting safety/capability.

Proposed tracer path, subject to approval: resolved 001 transfer record → shared host and complete Layout delivery → hands-on review stop → approved M2 adapters → M3 six-workspace acceptance → parity-gated dock retirement.

## References

- map — [frozen plan map](../toolbar-led-workbench/map.md), Destination and authority; Tickets and dependency order; Risks and deliberate limits
- architecture — [adopted architecture companion](architecture-toolbar-led-workbench/architecture-toolbar-led-workbench.md)
- architecture source — [frozen architecture](../toolbar-led-workbench/architecture.md), Shared module and adapters; Panel state and transitions; Draft owners, not merely hidden DOM; Responsive and keyboard contract; Motion and layering
- inventory — [frozen capability inventory](../toolbar-led-workbench/inventory.md), Shared shell and global surfaces; Layout tools and canvas; Layout Objects and Inspector; PCB adapter; Keymap adapter; Keycaps adapter; Case adapter; Parts adapter; Overflow routing proposal; Completeness boundary
- delivery — [frozen delivery contract](../toolbar-led-workbench/delivery.md), Milestones and authority; M1 acceptance and proof ownership; Coordination, verification and stop
- verification — [frozen proof and execution contract](../toolbar-led-workbench/verification.md), Reuse first and keep stories distinct; Mutation boundary; Closure and limits
- project — [ADR-0041](../../adr/0041-toolbar-led-workbench.md)

## Notes

- Decision: 2026-11-15 — User approved the three-epic split. Create only the 15 existing ticket identities in the approved order; do not add refactor-sweep tickets. Keep M2/M3 implementation behind their phase gates and the M1 hands-on stop.
- Open question: What should the View section do after a 3D round trip? The architecture does not specify whether it closes.
- Open question: What validation is required for shared Toolbar scroll-into-view across all consumers? The change affects every consumer; the recorded evidence covers Layout row bank/page stability.
- Decision: 2026-11-15 — Convert only toolbar-led-workbench into its sibling initiative folder; preserve all frozen source files unchanged, copy the architecture companion, and reference old ticket receipts instead of duplicating them. Do not change code, AGENTS.md or docs/agents/issue-tracker.md. Preserve all 15 old ticket identities; the approved split now authorizes epic envelopes, tickets.toml entries and plan status files.
- Note: 2026-11-15 handoff reports 001 resolved; 009 and 002 claimed; 003, 004, 005 and 006 ready; 010–015 and 007–008 awaiting decision. In 002, View keyboard item 6 is reported complete at receipt `6447c0e19`; remaining work includes proof-map receipt reconciliation, final Availability/Action-parity, native touch/trackpad scrolling, integrated candidate gate, fresh combined review, acceptance ticks and user approval. Earlier all-planned prose is historical and does not override these receipts.
