# F6C.1: Keycaps projection, shared selection, and physical 2D view

**Parent:** F6C.1 — `.scratch/dioxus-frontend-v1/workflows/F6.json`

**What to build:** In the Keycaps workspace, show the active board’s supported switch keys and matrices in a physical 2D view, with the same key selected from the shared canvas or searchable key list. Render effective keycap dimensions, color, and inherited or explicit legend. Keep this physical legend view distinct from Keymap’s active-layer binding labels.

**Blocked by:** INT.1 for implementation start. F3.1 remains the parent’s acceptance join for canonical tree/canvas selection and agreed board projection; it is not converted into a start blocker. No CAD bridge dependency for this 2D slice.

**Status:** ready-for-agent

- [ ] Use the existing accepted document/session and active board to project supported switch keys and matrices, retaining stable part IDs and the reference membership rules; handle no board, empty board, standalone switches, mixed matrices, and missing/overridden values without persisting defaults.
- [ ] Show the same canonical selected key when selection changes through the shared canvas or searchable list; preserve selection scope and do not add a second independent selection store.
- [ ] Render a physical 2D Keycaps view with effective dimensions, cap color, and legend inheritance/explicit blank behavior. Keymap’s separate active-layer label view remains distinct.
- [ ] Match the pinned React Keycaps behavior and visible states, including empty input; compare identical fixture/actions and document visible and selection results.
- [ ] Keep implementation private to the Dioxus frontend and existing session/contracts; introduce no engine, schema, public API, or visibility changes. Do not include keycap setting editors, fit-resolution UI, shared 3D viewer, or STEP export.
- [ ] Record exact source/fixture/check evidence and update the refactoring register, or record “No new refactoring takeaway observed.” Obtain the required independent Astra Standards and Spec review.
