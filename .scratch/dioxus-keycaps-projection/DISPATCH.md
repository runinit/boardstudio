# Dispatch contract — F6C.1 Keycaps projection and physical 2D

## Ownership and seam

INT.1 is the implementation start edge; its private Library/Objects/Inspector extraction does not supply a Keycaps mount or projection contract. Before editing, identify the actual page-crate mount, accepted-document and active-board identity, existing keycap resolver/read path, stable part IDs, and the shared selected-part callback. Keep the implementation in private Dioxus frontend modules and coordinator-owned shared composition files with the coordinator. Reuse canonical selection; do not create a second store. F3.1 remains an acceptance join and is not promoted into a start blocker. This slice needs no CAD bridge.

## Source and behavior

Parent requirements and source accountability remain `.scratch/dioxus-frontend-v1/workflows/F6.json`, `.scratch/dioxus-frontend-v1/tasks.json`, and the F6C.1 specification. The reviewed React sources are `app/src/ui/KeycapPanel.tsx`, `app/src/ui/createKeymapWorkspace.tsx`, `app/src/ui/KeymapLayout.tsx`, and the shared selection controls. Keycaps physical legends and Keymap active-layer binding labels are distinct views.

Dispatch must attach exact source-derived projection/default formulas and fixture identities. Preserve reference membership: include supported direct matrix members as well as supported matrix switch members; do not narrow the set to switches alone. Preserve stable IDs, matrix membership/order, effective dimensions and color, and the distinction between inherited legend and an explicit blank. Apply generated defaults for display without persisting them. Mixed matrices, standalone switches, missing values, overrides, no board, and empty board need deliberate states.

Selection from the canvas and searchable list must call the existing shared selection callback and stay scoped to the active board/document. The reference SVG key controls support pointer selection and keyboard activation with Enter and Space; verify both keyboard actions, focus behavior, and the same selected key in both surfaces. Search/no-match and empty input should not mutate the document, revision, history, or stored defaults.

## Checks and profile

Use the shared acceptance at `../dioxus-frontend-tranche-1/ACCEPTANCE.md` and retain the existing build, browser, keyboard/focus/axe, lifecycle, and independent Standards/Spec review gates. Compare matching pinned React and public Dioxus fixtures/actions, including light/dark and desktop/compact where affected. Keep editor controls, fit-resolution UI, shared 3D viewer, CAD and STEP export out of this ticket. Do not introduce engine, schema, public API, or visibility changes.

Before dispatch, provide a concrete feature contract and packet naming source files, callable inputs, scope identity, fixtures, browser session, checks, and evidence destination. Keep F3.1 and parent acceptance open until the existing evidence joins pass. Update the existing refactoring register or record “No new refactoring takeaway observed.”
