# Complete Dioxus frontend port

> Scope update (2026-10-05): accessibility/axe/semantic-only and assistive-technology qualification are removed by user instruction. Ordinary keyboard, focus, Escape and functional controls remain in scope. Historical results are retained without counting excluded checks as passes.

**Triage:** ready-for-agent
**Authority:** 2026-10-01 request and clarification: 100% frontend; all existing TSX, theming and UI behavior ported to Dioxus.
**Execution:** F1/F3a are verified increments; the 2026-10-02 workflow specs originally planned F2–F9; the current 61-parent graph excludes VIK-only F4.7 with independent author/reviewer roles. See [execution plan](PLAN.md) and [ownership/dispatch](EXECUTION.md).

## Problem Statement

The Rust candidate's stacked demonstration interface does not resemble the React
application. The user wants visible frontend progress first and a complete plan
to replace every TSX workspace/component, including themes and interactions.

## Solution

Port the established interface to Dioxus, starting with the recognizable shell
and themes, then completing project/shared UI and every workspace. Temporary
placeholder tabs are allowed in the first increment; none satisfy final v1.
Keep existing domain engines and service providers underneath the frontend.

## User Stories

1. As a designer, I want to recognize the same application shell, typography, colors and spacing, so that I can continue my existing workflow in Dioxus.
2. As a designer, I want to navigate the exact Layout, PCB, Keymap, Keycaps, Case and Parts tabs and Export action, so that I can continue my existing workflow in Dioxus.
3. As a designer, I want to understand clearly which temporary tabs are unfinished, so that I can continue my existing workflow in Dioxus.
4. As a designer, I want to keep edits, selection and history while switching workspaces, so that I can continue my existing workflow in Dioxus.
5. As a designer, I want to use the project menu without losing canvas space, so that I can continue my existing workflow in Dioxus.
6. As a designer, I want to choose Light, Dark or System appearance and retain it, so that I can continue my existing workflow in Dioxus.
7. As a designer, I want to use all views at supported desktop and compact sizes, so that I can continue my existing workflow in Dioxus.
8. As a designer, I want to resize or open panels and restore their state, so that I can continue my existing workflow in Dioxus.
9. As a designer, I want to operate menus, tabs and controls with visible keyboard focus, so that I can continue my existing workflow in Dioxus.
10. As a designer, I want to see truthful loading, saving, error and recovery states, so that I can continue my existing workflow in Dioxus.
11. As a designer, I want to create, open, import and delete local projects, and save portable copies, so that I can continue my existing workflow in Dioxus.
12. As a designer, I want to browse demo copies and saved keyboards, so that I can continue my existing workflow in Dioxus.
13. As a designer, I want to import and save portable project copies, so that I can continue my existing workflow in Dioxus.
14. As a designer, I want to follow the setup guide and resume editing, so that I can continue my existing workflow in Dioxus.
15. As a designer, I want to select objects through the tree or canvas with the same semantics, so that I can continue my existing workflow in Dioxus.
16. As a designer, I want to create and edit matrices and component placement, so that I can continue my existing workflow in Dioxus.
17. As a designer, I want to transform, align, snap, mirror and constrain the layout, so that I can continue my existing workflow in Dioxus.
18. As a designer, I want to edit outlines, cutouts, linked refinements and supported script controls, so that I can continue my existing workflow in Dioxus.
19. As a designer, I want to preview, cancel, apply and undo edits consistently, so that I can continue my existing workflow in Dioxus.
20. As a designer, I want to navigate findings to their affected objects or geometry, so that I can continue my existing workflow in Dioxus.
21. As a designer, I want to search, browse, create and import parts, so that I can continue my existing workflow in Dioxus.
22. As a designer, I want to edit supported generator parameters using existing services, so that I can continue my existing workflow in Dioxus.
23. As a designer, I want to inspect footprints, models, assemblies and modules, so that I can continue my existing workflow in Dioxus.
24. As a designer, I want to configure the PCB layers and hardware instances, so that I can continue my existing workflow in Dioxus.
25. As a designer, I want to edit controllers, connectors, wiring, pins and jumpers, so that I can continue my existing workflow in Dioxus.
26. As a designer, I want to review electrical readiness and located findings, so that I can continue my existing workflow in Dioxus.
27. As a designer, I want to edit keymap layers, keys and supported behavior bindings, so that I can continue my existing workflow in Dioxus.
28. As a designer, I want to edit macros and encoder actions, so that I can continue my existing workflow in Dioxus.
29. As a designer, I want to preserve supported legacy bindings and stable identities, so that I can continue my existing workflow in Dioxus.
30. As a designer, I want to edit keycap profiles, sizes, legends and fit, so that I can continue my existing workflow in Dioxus.
31. As a designer, I want to inspect keycaps and assemblies in 2D and 3D, so that I can continue my existing workflow in Dioxus.
32. As a designer, I want to use every supported Case setting and construction control, so that I can continue my existing workflow in Dioxus.
33. As a designer, I want to generate, cancel and retry cases with accurate status, so that I can continue my existing workflow in Dioxus.
34. As a designer, I want to inspect camera, picking, materials, layers and mechanical findings, so that I can continue my existing workflow in Dioxus.
35. As a designer, I want to use the exact offered export formats, selected-board scope and portable-copy embedding option, so that I can continue my existing workflow in Dioxus.
36. As a designer, I want to see the reference export error and readiness feedback, retry by repeating an action, and never receive stale or internally cancelled output, so that I can continue my existing workflow in Dioxus.
37. As a designer, I want to return from Export to the prior workspace, so that I can continue my existing workflow in Dioxus.
38. As a designer, I want to complete the same project-to-export workflow entirely in Dioxus, so that I can continue my existing workflow in Dioxus.
39. As a designer, I want to reopen work offline through the existing host services, so that I can continue my existing workflow in Dioxus.

41. As a maintainer, I want every TSX component and UI hook/style dependency mapped to its replacement, so that no functionality disappears during the port.
42. As a maintainer, I want frontend regression coverage transferred before React removal, so that the replacement retains meaningful checks.
43. As a maintainer, I want a reviewable entrypoint switch and rollback, so that the frontend can be adopted safely.

## Implementation Decisions

- Preserve the reference visual design and exact UI labels; port the frontend instead of redesigning the product.
- Dioxus owns transient presentation state. Reuse the current engine/session/provider APIs and existing storage compatibility; do not duplicate domain algorithms.
- Port React hooks/controllers that implement UI state with their screens. Retained service adapters stay documented and do not count as migrated frontend components.
- Match fonts/icons/tokens, light/dark/system themes, responsive panels, shortcuts, focus, draft lifetimes and loading/error/recovery behavior.
- Workspace switches preserve the application session and safely settle or retain active drafts. Case canvas lifetimes remain explicit.
- Placeholders are temporary, clearly labeled and non-editing. Complete reference workflows replace them before frontend v1.
- Keep service API/schema/visibility and data-writer cutover decisions separate; frontend scope does not authorize changes to them.

## Testing Decisions

- Reuse the accepted public session and real-browser seams, with paired React/Dioxus fixtures and captures. No private API/test hooks are required for the shell.
- Test externally visible actions, selected state, previews/cancellation/history, menus/focus, themes, responsive layout, errors and outputs through existing providers.
- Native/WASM checks cover affected Rust modules; browser checks cover actual Dioxus UI. Transfer useful React test cases before retiring their implementation-specific harness.
- Retain visual, ordinary keyboard/focus and affected performance/resource evidence. Preserve existing budgets and label blocked/unperformed checks accurately.

## Out of Scope

Backend engine/generator/CAD/kernel rewrites, new project formats, native-host
migration, unsupported feature expansion and immediate production data cutover.
This is the complete frontend migration, not a rescheduling of backend projects.

## Further Notes

The inventory and milestone roadmap cover all production TSX plus tests/benchmarks
and UI-owned TS/CSS/assets. Existing M1 limits remain recorded. Backend services
remain the foundation; any missing service capability is an explicit dependency,
not permission to silently drop or permanently placeholder a frontend feature.

The 2026-10-02 source audit corrects the earlier broad roadmap: project rename/duplicate and visible export progress/cancel/dedicated retry are not pinned-reference affordances. Preserve the actual existing UI. Case generation does have progress/cancel/retry controls. The detailed workflow specs and [source coverage](coverage.json) retain every required frontend responsibility.

## Refactoring takeaways during implementation

The user requested a living record of architectural, design, theoretical and general software-quality issues encountered during the rewrite. Update [post-port takeaways](../../docs/migration/POST-PORT-REFACTOR.md) and the [RF register](refactor-findings.json) at every workflow handoff/review, or state that no new takeaway was observed. Record evidence and uncertainty, impact, current mitigation, later proposal and validation. F9 carries the accumulated register into the major post-port refactoring phase. Required correctness stays in the current slice; broader structural redesign is deferred without waiving acceptance gates.


## Delivery record refinement — 2026-10-03

The user needs implementation progress without duplicate JSON/issue/review upkeep.
One live operations record references the canonical graph, receipts and finalized
candidate review. Counts and the readable RF report are derived views. Authors refine
existing specs/tickets and return one commit/receipt; the coordinator updates state
and accepts criterion-complete parents against preserved joins. Historical records
remain reachable. Validate record consistency and reference/hash integrity at this
existing tracker seam; this change adds no application test or review handoff.
See [frontend delivery records](../../docs/agents/issue-tracker.md#frontend-delivery-records).
