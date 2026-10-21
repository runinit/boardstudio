# 009: Give Layout ownership of selection and Inspector orchestration

Status: ready-for-agent
Type: build
Blocked by: [003: Keymap workspace ownership pilot](003-keymap-workspace-ownership.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Make Layout own its selection-dependent controller/projection assembly and Inspector
choices, so users can edit the same matrix, key, component, relation and outline
contexts without stale results or unrelated workspace details leaking into Editor.

## Scope and constraints

- Consolidate Layout tree/Inspector admission, board/matrix/key-size/transform/
  alignment/outline-controller wiring, accepted-versus-preview projection and
  construction of relevant Objects/toolbar/Inspector inputs in the existing module.
- Keep tree-cell anchors, component Inspector lifetime/bindings and relation highlight
  attached to their current ownership rules. Shared selection authority remains in
  the shell; do not introduce a second selection store.
- Include Layout findings' selection/navigation/resume coordination and cross-board
  destination checks through shared routing. Preserve stable route-slot refresh.
- Leave creation/placement workflow ownership for ticket 010 and transaction canvas/
  gesture dispatch for ticket 012, using bounded existing capabilities between them.

## Acceptance criteria

- [ ] Layout owns Inspector priority, accepted/preview projection, admitted selection/edit actions and relevant surface-input construction; Editor no longer assembles its individual Inspector controllers.
- [ ] Actual tree selection shows the correct board, matrix, key, component, relation or outline Inspector, including existing tabs, anchors and eligibility behavior.
- [ ] Real field edits retain queueing, accepted normalization, one-shot disabling, inline failures and exact Undo boundaries. View-only selection does not edit the accepted document.
- [ ] An unrelated accepted edit refreshes clean fields without overwriting a dirty draft; blur/commit uses the latest valid admission owner.
- [ ] Hold a result while changing selection/workspace/board, then release; obsolete observations cannot write failures, accepted values or selection into a newer binding. Newer drafts survive older outcomes.
- [ ] Preview findings remain distinct from accepted findings, reach the expected footer/Inspector and clear with the preview rather than authorizing accepted edits.
- [ ] A real cross-board Layout finding reaches the intended board/selection and supports its existing return route; stale destinations/resume callbacks are refused.
- [ ] Relation highlights and tree-cell anchors invalidate at the same owner changes as before, without resetting unrelated shared selection/view state.
- [ ] Provide narrow selection/setup/transaction capabilities for later Layout slices; do not replace the old monolith with a workspace-wide exposed controller bag.

## Verification

Run documented quick checks, page-only WASM compilation, the page-bin browser suite
and relevant Layout Inspector/remainder/layers/relation tests. Add production
selection and cross-board finding coverage only where existing tests do not exercise
the real composition. Follow the plan's shared contract.
