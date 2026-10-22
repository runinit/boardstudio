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

## Comments

**2026-10-21, findings (grilling in progress; claim resumed after an interrupted session).**

- **Branches.** `dev` is the integration branch (1,354 commits ahead of `main`, which is 1 ahead of `dev`). `chore/mcp`, where the
  plan docs live, is 22 commits ahead of `dev` and contains only docs, GLOSSARY.md and PRODUCT.md.
- **Process.** `docs/agents/verification-process.md` retired lane leases and mandatory review gates on 2026-10-11, so none apply.
  AGENTS.md still governs builds: one cargo or wasm-pack run per `target` directory, `BOARDSTUDIO_CHECK_JOBS` (default 4) bounds
  `scripts/check.py`, lane worktrees use their own target under `~/.cache/boardstudio-target`.
- **Tests.** 653 wasm-bindgen tests and 428 native tests. 35 test files; 144 lines in 15 files select by `.m1-*` classes, 36 lines
  in 7 files mention `board-*`, while 115 lines in 12 files use `aria-label`, which survives the migration. Tests select native
  `input` (78 lines), `button`, `select` (14) and `details`, which become custom. `scripts/check-native-tests-baseline.json` guards
  test names (2,178 lines); `scripts/check-wasm-tests-baseline.json` is currently `{}`.
- **Size per workspace (inventory):** case 8 files (49 raw, 121 `Board*`), parts 17, layout 18, app shell 18, pcb 9, keymap 5,
  keycaps 4, ui-shared 8, library 1.
