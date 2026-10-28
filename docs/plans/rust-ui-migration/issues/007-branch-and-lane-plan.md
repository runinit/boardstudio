# 007: Branch, lane and test-rewrite plan for the single merge

Status: resolved
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

## Answer

Decided with the user, 2026-10-21; all eight proposals accepted. Written in [branch-plan.md](../branch-plan.md):

- Merge the docs on `chore/mcp` into `dev` first (a user action), cut `feat/rust-ui-migration` from `dev`, merge `dev` into it at every
  lane merge, and merge it into `dev` once.
- A serial Foundation lane, then Shell and Layout first with Case as the third parallel lane (at most 3 at once), then Parts, the small
  workspaces, and Library and Export, then a Cleanup lane. Each lane has its own worktree and target directory.
- A lane only adds files under `ui/`, deletes only the old CSS its files used, and leaves shared rules to Cleanup; `presentation.rs`
  belongs to the Shell lane.
- Tests find controls by role and name, assertions stay the same except for decided changes, native name baselines are updated in the
  same commit, and every adopted interactive control passes [010](010-measure-keyboard-and-aria-conformance.md)'s checks first.
- A check fails while any `.board-*`, `.m1-*` or `--wb-*` remains (622 selectors, 80 variables today); it gates the final merge.
- Merge gate: full `check.py`, browser tests, the 010 checks, your review of every workspace in both Themes and Appearances at three
  widths, one two-axis code review, and the old-CSS check.
