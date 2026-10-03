# F6C.1: Keycaps projection, shared selection, and physical 2D view

**Parent:** F6C.1 — `.scratch/dioxus-frontend-v1/workflows/F6.json`

**What to build:** In the Keycaps workspace, show the active board’s supported switch keys and matrices in a physical 2D view, with the same key selected from the shared canvas or searchable key list. Render effective keycap dimensions, color, and inherited or explicit legend. Keep this physical legend view distinct from Keymap’s active-layer binding labels.

**Blocked by:** INT.1 for implementation start. F3.1 remains the parent’s acceptance join for canonical tree/canvas selection and agreed board projection; it is not converted into a start blocker. No CAD bridge dependency for this 2D slice.

**Status:** the targeted empty-key projection guidance is implemented in `web/src/presentation/keycaps_workspace.rs`; paired candidate acceptance and all remaining projection/selection criteria stay open.

- [ ] Use the existing accepted document/session and active board to project supported switch keys and matrices, retaining stable part IDs and the reference membership rules; handle no board, empty board, standalone switches, mixed matrices, and missing/overridden values without persisting defaults.
- [ ] Show the same canonical selected key when selection changes through the shared canvas or searchable list; preserve selection scope and do not add a second independent selection store.
- [ ] Render a physical 2D Keycaps view with effective dimensions, cap color, and legend inheritance/explicit blank behavior. Keymap’s separate active-layer label view remains distinct.
- [ ] Match the pinned React Keycaps behavior and visible states, including empty input; compare identical fixture/actions and document visible and selection results.
- [ ] Keep implementation private to the Dioxus frontend and existing session/contracts; introduce no engine, schema, public API, or visibility changes. Do not include keycap setting editors, fit-resolution UI, shared 3D viewer, or STEP export.
- [ ] Record exact source/fixture/check evidence and update the refactoring register, or record “No new refactoring takeaway observed.” Obtain the required independent Astra Standards and Spec review.

## Bounded empty-state leaf

The pinned React Keycaps panel shows `Add switches in Layout to create a keymap.` when the board-scoped key projection is empty (`app/src/ui/KeycapPanel.tsx:28`). Its saved-fixture reference state was also observed in an isolated browser profile against `http://127.0.0.1:5173/`: create a new project, open Keycaps, and the message is visible while the key picker has no choices and `Export keycap STEP` is disabled. Reference screenshot: `/home/chris/.local/share/boardstudio/reviews/layout-batch-32a57ed3-sol-20261003/keycaps-empty-state/react-empty-keycaps.png` (SHA-256 `22d978f1bd2d09378a567db92124ea2d663402ca3b7fcf2fdd94ed3f1b8c1d82`).

The Dioxus inspector now renders that same next-action message only when an accepted Keycaps view exists and its physical key list is empty. The no-board/unavailable-projection message remains a separate state. This leaf changes no edit owner, projection, saved data, or API. Source diff check passed; no package build or test was run for this routine UI copy/state addition. No new refactoring takeaway observed for this leaf. Paired Dioxus candidate evidence and the other acceptance/review gates above remain open.
