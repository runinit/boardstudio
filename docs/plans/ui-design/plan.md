# UI design lane

Parent: [140](../typed-core-edits/issues/140-ui-design-impeccable.md)
Status: active discovery; independent implementation branch, later merge

## Goal and authority

Make BoardStudio a sleeker, compact and clear expert workbench. Preserve precise direct editing and existing product truth while improving iconography with Solar, descriptions, hierarchy, color, responsiveness, contextual interactions and useful motion. Improve Layout editing and Case generation controls/visuals through existing shared operations. This is Operate-mode UI work, not a new keyboard implementation or a Core rewrite.

Coordinator: `01a12c41-1a51-7101-a555-188f1c85edce`; worktree `/home/chris/.codex/worktrees/2182/ts-boardstudio2`; branch `codex/ui-impeccable`; base integration `53970a404`. The coordinator asks the user questions directly in its own chat. It owns discovery, scoped child plans, Luna implementation and one fresh independent Sol medium finish/behavior review per coherent batch. Root owns shared tracking, index refresh, heavy scheduling and later integration. No recurring polling automation or routine permission roundtrips.

## Delivery sequence

Initial user priority is fast Layout editing: fewer clicks for common toolbar actions, discoverable Snap, and relevant item/row/column actions. Case follows. Apply foundations and shell improvements as needed for these coherent batches; they do not create another prerequisite chain.

1. **Observe and shape.** Run Impeccable context once per session; use critique/audit against real representative screens and task states. Reuse existing source critique packets where useful, but inspect rendered truth. Ask only consequential unresolved design questions in the lane chat. Establish a compact brief and agreed visual direction within current product constraints.
2. **Foundations and iconography.** Establish consistent spacing/density, type hierarchy, semantic color roles, focus/selection/disabled states and compact controls. Select a coherent Solar style and reusable accessible icon wrapper. Verify upstream provenance/license, local asset strategy and attribution; avoid importing unused sets or unrelated dependencies.
3. **Shell and Inspector.** Clarify navigation, project identity/save feedback, sections, descriptions and selection context. Reduce wasted space while keeping controls understandable and reachable; preserve accepted Inspector wording and Export blocker behavior. Handle narrow layouts without silently losing actions.
4. **Contextual Layout interactions.** Make current selection, available transforms and relevant actions clear and close to the task. Improve discoverability/feedback for existing matrix, stagger, offset, rotation, deletion and assembly controls. Preserve exact operation semantics, undo and measurement accuracy; do not invent future mirror APIs.
5. **Case workflow and visuals.** Improve generation parameter hierarchy, dimension/relationship explanations, preview readability and interaction feedback. Clearly distinguish active/pending/stale/failed geometry. Use existing case operations; CAD kernel or geometry defects require a separately reproduced scoped issue.
6. **Responsive, accessible and motion finish.** Check desktop and narrow viewports, keyboard/focus, overflow, touch targets, contrast, loading/errors and empty states. Use purposeful transitions with reduced-motion support. Avoid animations that delay editing or obscure geometry.

Each batch has exact paths, small acceptance criteria and an evidence packet. Use relevant Impeccable playbooks on demand (clarify, distill/layout/typeset, colorize, adapt/harden, animate, polish); do not run every command as a ritual. Read craft-floor immediately before UI edits. Follow the skill's bounded batched visual passes and stop open-ended polishing. Combine its fresh finish review with repository behavior acceptance when the same reviewer receives the complete evidence and rubric.

## Reused interface crosswalk

The [source-grounded commitment crosswalk](../delivery-queue.md#required-interface-commitments) records required scope, owners and proof limits. Reuse these interfaces; this adds no serial gate or public solver/helper visibility.

| UI concern | Existing interface and scope boundary |
| --- | --- |
| Selection and contextual actions | TreeContext/ScopedTreeContext, admitted canvas selection and ToolbarInput already carry item/row/column and transform context. Reuse existing IDs, selection modes and admission checks. |
| Layout transforms and assembly choice | MatrixTransformField/build_operation and existing matrix field/preset resolvers supply transforms, deletion and assembly operations. Preserve precise geometry, membership and Undo; do not implement mirror policy in the browser. |
| Mirror and selective exceptions | L1/B owns the private canonical operation through apply, both resolvers and reconcile, and 109 lifecycle/111 activation. L3/E ticket 21 consumes accepted MakeLocal/Omit/Rejoin/Unlink operations for independent parts and companions; L7 coordinates visual overlap only. 139's accepted model and narrow driving lookup serve 114; format 6 production activation remains 111. |
| Case editing and generation | Existing ApplyMechanicalEdit/CaseBodyEdit, scoped generation requests and geometry-readiness states support the UI. Preserve token/revision ownership, previous-versus-current geometry and cancellation semantics. |

## Ownership and independence

Standing UI lease: `web/assets/` styling and narrowly selected Solar assets; presentation modules under `web/src/presentation/`; visual components/controls in the `web/crates/` UI crates; UI-local tests and lane-local design briefs. The coordinator records exact changed paths per batch before its writer starts. Shared business logic, runtime/transport, Core/model/solver/schema, CAD kernel, footprint generators, root goldens, scripts/check.py, maps/ADRs/CONTEXT are excluded. Lane-local DESIGN.md/Impeccable design artifacts may evolve after direct user design approval; main checkout's dirty documentation must remain untouched.

F138 is deleting demos and porting shared tests. Do not edit F-owned retirement test hunks, bring back demo dependencies or borrow their geometry as an oracle. Use independently authored synthetic user projects. E21/41 owns future mirrored web adapters/companion controls: reserve its actual API wiring for that lane; coordinate shared visual component overlap through root. Preserve accepted Export137 synthetic regression. Read the central lane registry before starting a batch and escalate actual ownership conflicts, not routine private UI decisions.

Read-only commands and disjoint source work can proceed concurrently. One writer per worktree. Builds/tests/browser/timing/index refresh require the single explicit GLOBAL HEAVY grant; request whole bounded check cycles. Use this worktree's real target/cache/pkg directories; do not share mutable build outputs with other lanes. Root alone bootstraps/refreshes its unique GitNexus index. Before bootstrap, use the explicitly named integration index as provisional graph evidence with source confirmation, never a false clean gate.

The user's subsequent direct L7 approval permits an independent persistent private dev server, initial private provider build and frontend-only rebuild/visual inspection loop, as specified in [the workflow exception](../../agents/agent-workflow.md#standing-lane-authority-and-continuity). That exception supersedes the previous paragraph for the private dev loop. Full acceptance and integration suites still use root scheduling. Record processes, unique port and private output paths; refresh provider artifacts only when their inputs change. The redesign also includes researching current official Dioxus/Tailwind support and adopting that supported styling pipeline in this lane, with scoped frontend build/config changes, attribution and affected dependency checks. Preserve existing semantics and record deliberate compatibility decisions rather than a blanket CSS rewrite.

## Checks and later merge

Behavior fixes need meaningful expected RED then GREEN. UI presentation changes need actual WASM typecheck/browser validation, affected repo/lint checks and valid screenshots at the inspected viewports. Reuse sufficient source-bound evidence rather than repeat full suites per tweak. Capture exact revisions, changed bytes, paths, test results and open findings. Final complete graph delta and one independent review bind each coherent completed batch.

Keep reviewed commits on `codex/ui-impeccable`; do not merge or push. Root will choose a later integration boundary, sync with accepted Core/demo retirement changes, resolve real overlaps, preserve both regression sets and run one full integration batch. Report meaningful completion, failures and decisions; avoid constant progress messages. Parent140 remains open until its agreed scope is accepted and integrated.
