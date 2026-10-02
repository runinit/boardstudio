# Dispatch contract — F7.2 authored Case workspace

## Ownership and seam

F7.2 starts after INT.1, which is accepted; it has no acceptance joins. Before implementation, identify the page-local Case mount and callbacks, accepted document/session/selected-board identity, existing CaseBody and `SetCase` contracts, and the Session edit/history path. Keep Case form/editor state private and scoped. Use a private feature module and focused styles; `presentation.rs`, `runtime.rs`, global CSS, build wiring, and ledgers remain coordinator-owned. No new public API, schema, engine, or visibility change is authorized.

## Source and behavior

Use canonical F7 workflow/spec and 62-parent graph. Compare `CaseInspectorPanel.tsx`, `useCaseWorkspace.tsx`, and Workbench composition against `core/src/model.rs`, `core/src/lib.rs`, `application/src/session.rs`, and the current Dioxus Case presentation. Preserve body list, defaults, supported body kinds, selection, empty/no-body state, dimensions, mounts, gasket fields, validation and keyboard behavior through existing `SetCase` and Session history.

Preserve the generated-mode branch: when the selected board has a matching active mechanical stack, show the generated assembly panel/note and retain saved authored bodies while hiding their editor. When configuration belongs to another board, show the mismatch explanation and “Show configured board” alongside the selected board's authored editor. Do not allow simultaneous generated-stack and authored-body editing. Mechanical configuration and disable behavior belong to F7.4. F7.7 owns physical-instance selection/projection; F7.6 owns generation/readiness/STEP. These are not F7.2 start blockers.

Draft and result actions must remain bound to the accepted document, board and session, plus instance where applicable; stale forms cannot write to a changed scope. Numeric drafts commit on blur/Enter, Escape restores accepted values, and invalid input remains visible with useful feedback. Compare public Dioxus behavior with the same pinned React fixtures and retain source, history, focus, compact/desktop and light/dark evidence.

## Graph and handoff

F7.2 `start_after: [INT.1]`; `acceptance_after: []`. Keep every other parent edge unchanged. This ticket does not close F7.2 parent acceptance or F7.4/F7.6/F7.7 work. Use shared acceptance at `../dioxus-frontend-tranche-1/ACCEPTANCE.md` and record “No new refactoring takeaway observed” unless implementation evidence supports an existing RF update.
