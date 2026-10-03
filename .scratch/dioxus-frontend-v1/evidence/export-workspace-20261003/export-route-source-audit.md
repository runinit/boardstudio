# Export route control source audit (2026-10-03)

Source-only comparison against pinned React `5a472a9426e6e38993361da402cd4ec730feb369` and the current Dioxus private Export presentation. This is not public-route qualification.

## Export-owned controls

React `app/src/ui/Workbench.tsx` renders seven design output rows in order:
KiCad board, draft KiCad, ZMK firmware, KiCad footprints, SVG outline, DXF
outline, and Case STEP. The generated mechanical package is conditional on the
selected Case projection. The route also has the current selected-board name,
readiness/reason state, Review wiring/Review case links, portable project copy,
and the optional used-model checkbox.

Dioxus `web/src/presentation/export_workspace.rs` has the same row set, order,
conditional generated package, current board name, review links, and portable
copy option. ZMK uses its source-owning guarded row adapter. The authored STEP
row uses accepted canonical bodies; the generated package row uses the
selected-instance effective projection. Its body-readiness gate is now scoped
to saved bodies belonging to the selected board, and outline fallback uses the
same aggregate scene readiness as the React route when per-board readiness is
absent.

## Controls owned by other workspaces

The pinned React Export route does not contain a keycap print action or a
second Keycaps STEP row. `app/src/ui/KeycapPanel.tsx` owns the local “Export
keycap STEP” action, disabled when there are no keys. The matching Dioxus
control is in `web/src/presentation/keycaps_workspace.rs`; it remains F6-owned.
Neither route defines a separate physical printer control.

The pinned Export route also has no board-copy/repeat control. The visible
“Duplicate design as variant” action is in React's matrix inspector
(`app/src/ui/MatrixInspector.tsx`), with the Dioxus equivalent in
`web/src/presentation/objects/matrix_inspector.rs`. These controls remain with
their workspace owners; adding them to Export would invent a reference action.

## Source delta and architecture observation

This packet changes only Export row readiness in
`web/src/presentation/export_workspace.rs`: authored bodies are counted for
the active board, and selected-board outline readiness falls back to the same
aggregate value used by React. Shared Runtime/provider wiring was already
joined in earlier commits and is outside this packet. The private Export
presentation remains a narrow consumer of existing artifact owners; no second
keycap, board-copy, or geometry authority was introduced.

No tests, build, browser journey, or review were run. Existing route receipts
remain historical and do not prove the updated readiness edge. F8 output,
responsive, keyboard/accessibility, and cross-workspace joins remain open.
