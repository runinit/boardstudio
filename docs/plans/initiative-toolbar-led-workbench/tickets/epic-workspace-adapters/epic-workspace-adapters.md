---
type: epic
title: "Adapt PCB, Keymap, Keycaps, Case and Parts"
parent: initiative-toolbar-led-workbench
sources:
  - docs/plans/initiative-toolbar-led-workbench/initiative-toolbar-led-workbench.md
companions:
  - docs/plans/initiative-toolbar-led-workbench/architecture-toolbar-led-workbench/architecture-toolbar-led-workbench.md
  - docs/plans/toolbar-led-workbench/map.md
  - docs/plans/toolbar-led-workbench/inventory.md
  - docs/plans/toolbar-led-workbench/delivery.md
  - docs/plans/toolbar-led-workbench/verification.md
covers: ["R1", "R3", "R5", "R6", "R7", "R8", "R11"]
after: [epic-layout-delivery]
risk: high
---

# Adapt PCB, Keymap, Keycaps, Case and Parts

## Description

After M1 is complete and the user approves M2 following hands-on review, adapt the five remaining workspaces to the shared toolbar and panel host. Preserve each domain's inventory, editor and selection ownership, update-safety behavior and action semantics; do not infer unanswered M2 policy from the shared shell.

## Outcome

Users can use PCB, Keymap, Keycaps, Case and Parts through the shared chrome while each workspace retains its own tools, editors, previews, guards and workflows.

## Requirements

R1 (parent R1): Integrate PCB, Keymap, Keycaps, Case and Parts with the host and global header supplied by M1. Demonstrate each adapter's applicable controls and retained domain authority; M3 owns final six-workspace acceptance.

R3 (parent R3): For the five M2 workspace adapters and their domain-specific editors/tasks, panel and task visibility follow one current workspace authority; selection and camera behavior remain domain-owned; hiding preserves the existing draft, error, disclosure and live update-safety registration lifetimes without synthetic commits, stale-owner replay or silent loss. Demonstrate captured-owner outcomes on the specified close, switch, workspace-return and session-replacement paths.

R5 (parent R5): The later adapters retain their named workspace capabilities and owners without introducing a universal object model or fabricated Layout actions. Demonstrate each adapter's inventory dispositions and stale-owner/admission behavior after its phase is approved.

R6 (parent R6): Preserve M2's identities, order 010, 011, 012, 013, 015, 014 and exact prerequisites. Begin implementation only after the M1 hands-on approval, resolve each named domain gate, and hand completed adapters to the separately authorized M3 epic.

R7 (parent R7): Each M2 adapter ticket maps each requirement to a distinct observable proof row with a faithful independent oracle, tier and status; existing evidence is reused only with a receipt, proof gaps name their boundary mutation, and unrun evidence remains planned. Verify the proof map and receipts at ticket closure.

R8 (parent R8): Each M2 ticket closes in this order: RED or recorded proof gap → fix → affected checks → named boundary mutations for proof-gap rows only → one fresh combined Spec+Standards reviewer → integrated ticket gate, followed by required human approval where the phase or ticket awaits it. Report unmet criteria and follow-ups distinctly; approval is not waived by a reviewer.

R11 (parent R11): Each of 010–015 demonstrates its applicable adapter-specific responsive, keyboard, target-size and motion behavior against the architecture contract, including Case Preview's separately approved task controls. Preserve the M1 shared mechanics and record measurements and limitations for 007's final six-workspace acceptance.

## Constraints

- This epic is gated by M1 completion and explicit user approval after hands-on review. Its entry plans retain separate domain questions; approval of the split authorizes this plan, not implementation.
- Do not invent a universal entity model, new Layout operations, or a new draft registry. Preserve existing workspace-specific identity, edit and update-safety owners.
- Classify commit triggers and update-safety needs for unregistered forms before changing their treatment; registration absence is not an explicit-submit decision.
- Create only the six assigned legacy ticket identities 010–015; no additional refactor-sweep tickets.

## Done when

- The user has approved M2 after reviewing the M1 candidate, and all six assigned adapter entries have their exact source-derived work and unresolved decisions recorded.
- Each approved adapter retains every source-inventoried capability and domain owner, with its stale-owner, draft/update-safety and action parity proofs complete and receipts recorded.
- Every ticket follows RED/proof-gap → fix → affected checks → named proof-gap mutations only → one fresh combined Spec+Standards review → integrated ticket gate; phase approvals remain with the user.
- No undocumented M2 policy, capability removal or implementation claim remains; M3 starts only after the user authorizes it and the M2 epic is complete.

## Boundaries

- Touch point: PCB workspace Objects, toolbar and Selection/Routing & nets — entry 010; retain electrical/routing owners.
- Touch point: Keymap navigation, layer operations and binding/macro/encoder editors — entry 011; retain key/layer/input identities and existing controllers.
- Touch point: Keycaps defaults, overrides, fit findings and viewer controls — entry 012; preserve its pending navigation and settings owners.
- Touch point: Case Objects, Inspector and existing View controls — entry 013; Case Preview generation/command ownership remains separately gated in 015.
- Touch point: Case Preview controls and generation/command owner contract — entry 015; require concrete user-approved interface and one observer/command owner.
- Touch point: Parts catalog, authoring, preview, placement and return routes — entry 014; retain actual definition/module/assembly identity.
- Must not change: The completed M1 Layout candidate, old-dock retirement, geometry/output policies, or any M2 behavior left undecided by its source ticket.

Trace: M1 completion and hands-on review → user approval → 010 PCB → 011 Keymap → 012 Keycaps → 013 Case → 015 separately gated Case Preview → 014 Parts → M2 completion → user authorization for M3.

## References

- parent — [initiative Requirements](../../initiative-toolbar-led-workbench.md#requirements)
- architecture — [workbench architecture companion](../../architecture-toolbar-led-workbench/architecture-toolbar-led-workbench.md)
- map — `docs/plans/toolbar-led-workbench/map.md`, Tickets and dependency order; Remaining choices for approval
- inventory — `docs/plans/toolbar-led-workbench/inventory.md`, PCB, Keymap, Keycaps, Case, Parts and overflow-routing sections
- delivery — `docs/plans/toolbar-led-workbench/delivery.md`, M2 milestone and deferred decisions
- verification — `docs/plans/toolbar-led-workbench/verification.md`, adapter proof maps and closure contract
- legacy tickets — [010](../../../toolbar-led-workbench/issues/010-pcb-workspace-adapter.md), [011](../../../toolbar-led-workbench/issues/011-keymap-workspace-adapter.md), [012](../../../toolbar-led-workbench/issues/012-keycaps-workspace-adapter.md), [013](../../../toolbar-led-workbench/issues/013-case-workspace-adapter.md), [015](../../../toolbar-led-workbench/issues/015-case-preview-owner-contract.md), [014](../../../toolbar-led-workbench/issues/014-parts-workspace-adapter.md)

## Notes

- Decision: 2026-11-15 — After M1 hands-on approval, 010 is the first adapter tracer through shared Objects/tools, Inspect and retained placement/routing owners. Preserve order 010, 011, 012, 013, 015, 014; 015 requires 013's Case integration, and PCB/Parts retain their M1 workflow dependencies. Independent entries retain only their actual prerequisite links; shared routing remains coordinator-owned.
- Decision: 2026-11-15 — The approved migration retains exactly 15 existing tickets. Do not add a refactor sweep; retain cleanup and acceptance obligations within the existing ticket boundaries.

- Decision: 2026-11-15 — User approved planning this epic only; each M2 entry remains awaiting its named domain decisions and implementation phase approval.
- Open question: PCB, Keymap, Keycaps, Case and Parts draft forms retain their ticket-specific trigger classification and update-safety questions; settle each only through its owning ticket and user gate.
