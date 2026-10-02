# Complete Dioxus frontend port

**Triage:** ready-for-agent
**Authority:** 2026-10-01 request and clarification: 100% frontend; all existing TSX, theming and UI behavior ported to Dioxus.
**Execution:** F1 now; all later frontend phases planned and dependency ordered.

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
11. As a designer, I want to create, open, rename, duplicate and delete local projects, so that I can continue my existing workflow in Dioxus.
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
35. As a designer, I want to choose supported export formats, scopes and options, so that I can continue my existing workflow in Dioxus.
36. As a designer, I want to receive correct progress, cancellation, error and retry feedback for exports, so that I can continue my existing workflow in Dioxus.
37. As a designer, I want to return from Export to the prior workspace, so that I can continue my existing workflow in Dioxus.
38. As a designer, I want to complete the same project-to-export workflow entirely in Dioxus, so that I can continue my existing workflow in Dioxus.
39. As a designer, I want to reopen work offline through the existing host services, so that I can continue my existing workflow in Dioxus.
40. As a designer, I want to use assistive technology without losing supported controls, so that I can continue my existing workflow in Dioxus.

41. As a maintainer, I want every TSX component and UI hook/style dependency mapped to its replacement, so that no functionality disappears during the port.
42. As a maintainer, I want frontend regression coverage transferred before React removal, so that the replacement retains meaningful checks.
43. As a maintainer, I want a reviewable entrypoint switch and rollback, so that the frontend can be adopted safely.

## Implementation Decisions

- Preserve the reference visual design and exact UI labels; port the frontend instead of redesigning the product.
- Dioxus owns transient presentation state. Reuse the current engine/session/provider APIs and existing storage compatibility; do not duplicate domain algorithms.
- Port React hooks/controllers that implement UI state with their screens. Retained service adapters stay documented and do not count as migrated frontend components.
- Match fonts/icons/tokens, light/dark/system themes, responsive panels, shortcuts, focus, draft lifetimes and loading/error/recovery behavior.
- Workspace switches preserve the application session and safely settle or retain active drafts. Case canvas lifetimes remain explicit.
- Placeholders are temporary, clearly labeled, non-editing and accessible. Complete reference workflows replace them before frontend v1.
- Keep service API/schema/visibility and data-writer cutover decisions separate; frontend scope does not authorize changes to them.

## Testing Decisions

- Reuse the accepted public session and real-browser seams, with paired React/Dioxus fixtures and captures. No private API/test hooks are required for the shell.
- Test externally visible actions, selected state, previews/cancellation/history, menus/focus, themes, responsive layout, errors and outputs through existing providers.
- Native/WASM checks cover affected Rust modules; browser checks cover actual Dioxus UI. Transfer useful React test cases before retiring their implementation-specific harness.
- Retain visual, keyboard/axe/actual assistive-technology and affected performance/resource evidence. Preserve existing budgets and label blocked/unperformed checks accurately.

## Out of Scope

Backend engine/generator/CAD/kernel rewrites, new project formats, native-host
migration, unsupported feature expansion and immediate production data cutover.
This is the complete frontend migration, not a rescheduling of backend projects.

## Further Notes

The inventory and milestone roadmap cover all production TSX plus tests/benchmarks
and UI-owned TS/CSS/assets. Existing M1 limits remain recorded. Backend services
remain the foundation; any missing service capability is an explicit dependency,
not permission to silently drop or permanently placeholder a frontend feature.
