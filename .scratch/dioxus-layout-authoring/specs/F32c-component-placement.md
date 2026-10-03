## Problem Statement

The Parts catalogue can be browsed in Dioxus, but a designer cannot generally put one of its components on the keyboard or apply it to a selected key. The only existing placement button belongs to the Setup Guide's Wiring stage and accepts controllers only. The React application has two distinct actions: Add object → Parts always starts standalone canvas placement, while choosing a definition in the Parts workspace applies an eligible definition to a selected key or starts standalone placement for other selections.

## Solution

Port both React actions with the same labels, destinations, placement preview, keyboard and pointer behavior, accepted edits, selection result and history behavior. Keep placement available as an ordinary editor action independent of the Setup Guide. Preserve the controller-specific Wiring action as a separate entry path.

## User Stories

1. As a keyboard designer, I want Add object → Parts to show searchable, grouped available definitions so that I can find a component without leaving Layout.
2. As a keyboard designer, I want choosing any definition from Add object to start standalone placement even when a key is selected, so that the explicit Add object intent is not mistaken for an attachment to that key.
3. As a keyboard designer, I want to choose Board / ungrouped or a layout on the active board so that a committed part belongs to the destination I chose.
4. As a keyboard designer, I want the pending component to appear near the current canvas view center, snapped using current Layout preferences, so that it is immediately visible and aligned.
5. As a keyboard designer, I want pointer movement, Alt free placement, arrow-key nudging, Enter commit and Escape cancel so that component placement works with the same pointer and keyboard interaction as React.
6. As a keyboard designer, I want reversible Ergogen definitions normalized for the current construction before placement so that saved parameters and generated footprint geometry agree.
7. As a keyboard designer, I want a successful standalone placement to add the definition, part, active-board membership, matching board outline-envelope membership and optional layout membership in one accepted edit so that the component is immediately valid and undoable.
8. As a keyboard designer, I want a committed component selected with its Inspector visible so that I can continue editing it without searching for it again.
9. As a keyboard designer, I want a failed or stale placement to leave the accepted project and history unchanged so that a board, project, scope, workspace or destination change cannot commit an old preview.
10. As a keyboard designer, I want choosing a switch or input-capable definition while a matrix key is selected to set that key's assembly so that the selected key receives the intended behavior.
11. As a keyboard designer, I want an ineligible definition to retain the reference disabled action and explanation to clear key selection for standalone placement, so that the port preserves the existing eligibility rule.
12. As a keyboard designer, I want key insertion to retain the exact selected matrix, row and column, and to preserve linked-layout local-override behavior so that the edit affects the intended key and only the intended linked half.
13. As a keyboard designer, I want Parts selection with a non-key scope to start standalone placement so that choosing a component remains useful outside key selection.
14. As a keyboard designer, I want the Parts workspace action and Add object → Parts to retain distinct intent so that standalone placement never accidentally replaces a selected key assembly.
15. As a keyboard designer, I want an invalid, rejected or no-longer-available definition to produce the same recoverable result without changing accepted content so that I can correct the action and retry.
16. As a keyboard designer, I want both entry paths, search states, preview controls, contextual panels, focus, compact layout, and theme behavior to match the same React fixture so that I can switch applications without relearning placement.

## Implementation Decisions

- Keep the canonical F3.2 parent and its 62-parent graph unchanged. F3.2c is the existing child ticket; this spec does not create a duplicate ticket or close any parent.
- Keep the Setup Guide controller action separately gated by its live project and Wiring stage. Generic placement uses an explicit user action and never clears a selected-key intent before deciding which route applies.
- Use the existing accepted Snapshot, Session `Scope`, monotonic workspace generation, private canvas arbiter and current Runtime operation owner. A start captures project/session/revision, board, scope/generation, selected definition and placement destination; asynchronous completion rechecks the captured owner before it can create a preview or submit an edit.
- Reuse the current catalogue and construction-normalization owner. Resolve the selected ID from the accepted project/catalogue precedence, then validate that the resolved definition still matches the captured source before applying it.
- Reuse the current `PartPlacement` canvas owner for the pending preview and pointer/keyboard routing. Keep its lifecycle independent of Setup Guide policy; do not add another canvas gesture system or mutable document store.
- Standalone placement submits one existing accepted document edit that adds the definition, part, board membership, board outline-envelope membership and optional selected-board layout membership together. Validate the destination again at commit. The edit remains owned by the existing Runtime/Session/Core history and save path.
- Assign stable part IDs through the existing browser identity source and assign the React reference prefix/counter policy by definition kind. Do not add public ID-generation authority.
- Parts-workspace key insertion captures a current key context and uses the existing SetMatrix edit path. Public selected-key eligibility follows the React `matrixInputAvailable` rule, including for switch definitions. The private placement helper separately uses `kind === switch || matrixInputAvailable` to choose replacement after admission. The advisory input predicate accepts an explicit independent press pair from the input profile, matrix terminals, supported legacy rotary-encoder contacts or legacy `one`/`two` pads; it verifies that resolved row/column pads exist, are disjoint, and do not overlap rotary contacts. Definitions outside that predicate remain disabled by the public Parts Inspector action, matching its explicit reference explanation. The private helper append fallback does not establish a public route and must not be used to expand UI eligibility. Existing Core validation remains authoritative. Preserve linked/local component semantics and do not invent a separate component identity or edit operation.
- The root owns accepted-definition projection, the ID source, typed edit translation, final selection/navigation, the canvas mount and shared CSS. The feature owns private placement state, catalogue action presentation, validation/reconciliation and focused tests. Root composition changes remain narrow and are integrated serially.
- Keep custom footprint authoring/import, 3D model attachment, generator-setting editing, matrix/cell Inspector editing, component transforms and broader Parts-library completion in their existing slices.

## Testing Decisions

- Test externally visible accepted behavior rather than helper implementation details. Include mounted production-owner tests for async load, stale project/scope/workspace/generation/destination and selected-key changes, no ghost after rejection, one-operation acceptance, selection settlement, and exact canvas-owner release on cancel/commit.
- In the browser, use the same saved multi-board/layout fixture and matching React actions. Verify both entry paths, selected-key switch/input replacement and disabled ineligible-definition behavior, board/layout destination, pointer and keyboard commit/cancel, linked-half local behavior, Undo/Redo and saved archive reopen. Compare empty, loading, error and no-match states plus focus, compact layout and both themes.
- Keep native operation tests for board/layout/envelope membership, ID/reference uniqueness, definition consistency, matrix-key bounds and linked-local behavior. These support the mounted and browser seams; they do not replace them.
- Prior art is the existing mounted PartPlacement owner/settlement coverage, matrix SetMatrix inspector tests, paired React/Dioxus browser journeys, and saved-fixture history/reopen checks.

## Out of Scope

- Changing the canonical F3.2/F3.3/F3.7 graph, any public Rust API/member visibility, serialized project format, Core operation set, generator rules, or shared history authority.
- Setup Guide controller-flow redesign, linked-layout creation, key Inspector replacement/removal, component transform tools, Parts import/authoring/model workflows, and the later refactoring phase.

## Further Notes

This is a capability-level child start. Its start requires the source-confirmed accepted Snapshot/Scope and selection context, catalogue resolver, existing `ReplaceDocument`/`SetMatrix` edit and operation-observation path, canvas event owner, and shared arbiter. Full F3.1 and F3.2 parent acceptance remain unchanged acceptance joins and are not newly required before this child starts. Carry existing RF-001 forward for root-mount integration; do not create a new refactor identity for missing UI or for guide-specific policy.

## Oracle correction retained

The original public attached-member criterion inferred reachability from a helper branch. Review of the pinned Parts Inspector’s explicit disable and explanation corrects that interpretation: preserve the `matrixInputAvailable` UI restriction. This does not waive existing attached-member Inspector criteria or alter any canonical parent edge. A fresh linked-half switch/input replacement must mark `assemblies_local` using the existing matrix edit; Core otherwise restores inherited assemblies. Verify the accepted result and history with an initially non-local linked cell.

### Exact Inspector predicate correction — 2026-10-02

Pinned Parts Inspector admission uses `matrixInputAvailable` alone, including for Switch definitions. Preserve its explanatory disabled-action copy. The private helper uses `switch || matrixInputAvailable` after admission; it does not widen the public action. Earlier switch-bypass interpretations above remain historical evidence and are superseded by this distinction. Delayed loader failures and kind mismatches must retain the captured accepted-token/revision guard, just like successful deliveries.
