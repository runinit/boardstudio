# 05: Row and key Shift / Bend handles

Status: ready-for-agent
Type: build
Blocked by: [03](03-handle-rail.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/layout-controls-iconography` (e91865f43), `?variant=D`.
Review: web-only behaviour on existing Core ops; RED/GREEN evidence and a diff read.

## What to build

The rail gains the handles that variant D adds. A row offers **Shift** and **Bend** next to its shipped Offset. A key offers
**Shift** and **Bend**. Each drag edits only its own field through the existing placement change
(`PlacementParameter::RowBend` / `KeyBend` and the matching Shift parameters in
`layout_transform_toolbar.rs`), and each commit is one Undo entry. Hover marks the keys that
would move in amber, using the same key sets as the Inspector field preview
(`matrix_relations::field_preview`: row Shift/Bend carry downward, key Shift/Bend carry upward).
`transform_tools` gains the row and key cases. Hidden-handles mode keeps only the active tool.

## Seam and first RED / GREEN slice

Seam: `transform_tools` (pure) plus the existing drag-to-placement-change mapping.

RED: a native test that `transform_tools` for a Row context returns `[Offset, Shift, Bend]` and for a
Key context returns `[Shift, Bend]`. Today a row returns `[Offset]` and a key returns nothing.
Then a mounted test drags Row Bend by a known amount and asserts one revision with the row's
`bend` changed by the expected angle.

## Acceptance

- [ ] Native `transform_tools` and drag-mapping tests for row and key, including hidden handles.
- [ ] Mounted tests: row Bend drag, key Shift drag, the hover amber set matching
  `field_preview` for the same field, and one Undo entry each.
- [ ] Mirrored keys follow Core's sync rules (Local keys stay put), as with the field preview.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser).
  `check.py test --update-native-baseline` only for deliberate test renames.

## Carried point

- Row Offset (`PlacementParameter::RowOffset`) and Row Shift (`RowShift`) are separate fields
  (`web/crates/layout/src/matrix_transform_operation.rs`). Variant D's row rail shows only Shift
  and Bend. Whether to drop the shipped Offset token is undecided. By default the row rail is
  Offset, Shift, Bend; ask before removing a shipped tool.
