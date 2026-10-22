# 007: Branch, lane and test-rewrite plan for the single merge

Status: claimed
Type: grilling
Blocked by: [004](004-map-building-blocks-to-rust-ui.md), [005](005-theme-structure.md)
Parent: [Map](../map.md)

## Question

What lands first on the long-lived branch (scratch-proven components, `tw_merge`,
Themes), how per-workspace lanes split without colliding in `ui-shared`, how tests are
rewritten to rust-ui interaction while product behaviour is proven unchanged, and what
gate the branch must pass before its one merge. Also decide: which behaviours must be re-proven when native `select` (105), `details`
and `dialog` become custom controls (typeahead, touch pickers, focus return, form semantics); and the order and rule for retiring
`m1.css` and the `.board-*` / `.m1-*` rules, including layout rules in them that are not component styling.
