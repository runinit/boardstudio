# Draft F3.3d — Restore the single Layout command pill

**Parent:** F3.3 — Transforms, constraints and snapping. This is a composition-only integration join after the real leaf controls are available.

## Capability start

Start only after the existing F3.3a Select/Snap child and the Transform and Align leaves from F3.3b/F3.3c each have real current-scope behavior and root-owned intent callbacks. Do not start this ticket to draw disabled/decorative controls while those leaves are missing.

## Scope

- Compose one floating Layout command pill in the existing canvas toolbar, in React order: Select, Transform, Align, Snap. Keep the Design view selector (2D / 3D assembly / Footprints) in its separate group directly below/adjacent as in the pinned reference; do not merge view state into command state.
- Remove the separate Select/Snap placement and extra selection-context chip only if their useful context is preserved in the existing tree/Inspector/status. Do not hide or duplicate selection facts; this ticket is about the command surface and its controls.
- Root owns the shared composition and root-lifetime signals. Reuse the existing leaf components and callbacks; no additional selection, snap, transform, alignment, or document store.
- Relationships routes the selected Layout scope to the shared F3.5 Relations tab and is disabled without a current selection context. Keep it inside the Align menu as in React; do not add a separate toolbar state or imply that broader F3.5 acceptance is complete.
- RF handoff: no new refactoring takeaway observed. Keep accepted context, relation projection and tab state in the existing Editor/inspector owners; preserve RF-001 as the shared composition boundary.
- Preserve active transform finish/cancel affordance and open-menu dismissal semantics without pushing the pill outside compact viewport bounds.

## Acceptance

- On a same-build current public Dioxus route and pinned React route, compare the toolbar at 1280×577 and a compact viewport in Layout 2D. The four functional controls form one visible pill and the separate view-switch group remains outside it. Capture menu open states and keyboard/focus behavior in light and dark themes.
- Verify selecting each button opens its actual leaf menu, actions reach the right current context, opening another menu closes the previous one, Escape/focus restoration work, and board/workspace/view transitions do not leave stale open or active transform UI.
- Run the integrated public journeys from F3.3a/b/c sufficient to prove the composition did not disconnect handlers or duplicate state. This integration ticket does not replace leaf behavior acceptance and does not close F3.3/F3.7.

## Suggested graph

Proposed ID F3.3d. `start_after=[F3.3a,F3.3b,F3.3c leaf controls ready for composition]`; acceptance retains canonical parent joins. This ticket adds no new canonical dependency edge until the coordinator publishes its reviewed task graph.

## Candidate 34774 review follow-up — 2026-10-03

The consolidated candidate review identifies a remaining Properties shortcut route after selecting Relations. Repair and qualify this within the existing F3.3 issue; source finding and exact references live in [the candidate review](../../dioxus-frontend-v1/evidence/candidate-34774-20261003/consolidated-review.md). Browser qualification remains unexecuted; this does not reopen accepted F3.1 selection evidence.
