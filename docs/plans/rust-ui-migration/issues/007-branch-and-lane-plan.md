# 007: Branch, lane and test-rewrite plan for the single merge

Status: open
Type: grilling
Blocked by: [004](004-map-building-blocks-to-rust-ui.md), [005](005-theme-structure.md)
Parent: [Map](../map.md)

## Question

What lands first on the long-lived branch (scratch-proven components, `tw_merge`,
Themes), how per-workspace lanes split without colliding in `ui-shared`, how tests are
rewritten to rust-ui interaction while product behaviour is proven unchanged, and what
gate the branch must pass before its one merge.
