# 140: Refine the BoardStudio workbench with Impeccable

Status: claimed
Type: build
Blocked by: none

## Outcome

Deliver a sleeker, compact, clear workbench: coherent Solar iconography, useful descriptions, deliberate color, responsive controls, contextual Layout actions, better Case controls and previews, and purposeful interaction/motion. This is an independent UI effort for later integration; it does not block the mirrored Core chain or demo retirement.

The execution plan is [UI design lane](../../ui-design/plan.md). Coordinator chat: `01a12c41-1a51-7101-a555-188f1c85edce`. Worktree: `/home/chris/.codex/worktrees/2182/ts-boardstudio2`, branch `codex/ui-impeccable`, starting integration commit `53970a404d2ddfcd07be55c5de682f1a832dcd22`.

The user authorizes this coordinator to ask design questions directly in its own chat, plan bounded children, delegate implementation and one independent acceptance review per coherent unit, and proceed through approved design batches. Root owns integration and shared tracking. The reviewed UI branch was merged into dev on 2026-10-15 at the user's request; visual acceptance remains open.

Direct user steering in L7 additionally authorizes a persistent private frontend build/serve loop and adoption of supported Dioxus/Tailwind styling for the redesign. The [workflow exception](../../../agents/agent-workflow.md#standing-lane-authority-and-continuity) defines private output isolation and remaining shared acceptance scheduling. This permits scoped frontend build/config changes; Core/CAD algorithm and unrelated configuration boundaries remain intact.

## Acceptance

- [ ] Current rendered UI is critiqued and audited; a concise design brief and prioritized coherent batches are agreed with the user in the lane chat.
- [ ] Shared controls, typography, spacing, semantic colors and Solar icon mapping form one consistent system, with license/attribution preserved and accessible names for actions.
- [ ] Layout selection/actions and Case controls/preview improve task clarity using actual existing operations; precision, undo/redo and revision/readiness safeguards are preserved.
- [ ] Desktop and narrow layouts, keyboard/focus behavior, contrast, overflow, empty/loading/error states and reduced motion are checked on real mounted UI.
- [ ] Each completed batch has bounded visual evidence, affected WASM/typecheck/browser checks, one fresh independent Sol medium acceptance and complete source-bound graph evidence. Source-only critique is not visual verification.
- [ ] Lane commits remain independently reviewable; all overlaps with active Core/web/retirement work are recorded for later integration, with no resurrection of demos or discarded regressions.

## Boundaries

Frontend presentation, styling, UI-local interactions and tests, reusable icon assets, lane-local design documentation are in scope. Core schema/solver/mirror behavior, CAD kernel algorithms, generated goldens, demo-board code, scripts/check.py, shared maps, ADRs and CONTEXT are excluded. A missing backend capability becomes a separate scoped ticket. Root schedules the one GLOBAL HEAVY slot.

## Comments

- Unclaimed 2026-10-12 at the user's request; status back to `ready-for-agent`. Existing work on this ticket's branches or worktrees was not touched.

- Resumed 2026-10-14; UI work integrated into dev on 2026-10-15. Visual acceptance remains open.
