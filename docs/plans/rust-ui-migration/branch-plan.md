# Branch, lane and test plan

Ticket: [007](issues/007-branch-and-lane-plan.md). Decided with the user, 2026-10-21 (all eight proposals accepted). The migration
lands in one merge to `dev` ([ADR-0034](../../adr/0034-adopt-rust-ui-components-with-two-themes.md)); no old and new controls coexist
on `dev`. Inputs: [inventory](inventory.md), [mapping](mapping.md), [theme contract](theme-contract.md).

## 1. Branch topology

0. **Docs first.** Merge the docs on `chore/mcp` (22 commits ahead of `dev`, docs, glossary and PRODUCT.md only) into `dev`.
   This is a user action; the plan does not perform it.
1. Cut `feat/rust-ui-migration` from `dev`. Lanes branch from it and merge back into it.
2. At every lane merge, merge `dev` into the migration branch and resolve conflicts there, so drift stays small.
3. One merge of `feat/rust-ui-migration` into `dev`, after the merge gate in section 5.

## 2. Lanes

Each lane has its own worktree and its own `target` directory (AGENTS.md: one cargo or wasm-pack run per target, lane worktrees use
`~/.cache/boardstudio-target`). `scripts/check.py` runs up to `BOARDSTUDIO_CHECK_JOBS` (default 4) steps at once.

| Lane | Scope (files from the [inventory](inventory.md)) | Raw / Board |
| --- | --- | --- |
| **Foundation** (serial, first) | copy the base `ui/` components, add the `tw_merge` and `strum` dependencies and the vendored Lucide icon components; tokens, both Themes, `data-appearance` / `data-theme`; the shared size tokens; the role-and-name test helper; the removal of built-in borders from the copied components (the cards style, [theme contract](theme-contract.md) section 8); the accessibility fixes listed in [accessibility-floor.md](accessibility-floor.md) section 3 (Select, Dropdown Menu, Toolbar, Slider, Toggle Group, Toast) with their tests; the old-CSS check (section 4) | `ui-shared` 8 files, 22 / 7 |
| **Shell** | `web/src/presentation.rs`, `workspace_navigation.rs`, `workbench_shortcuts.rs`, `workspace_composition.rs`, `inspector.rs`, `canvas_status_footer.rs`, `empty_board_canvas.rs`; header, docks (Sidenav, Sheet), Command palette, Toasts; fallback for saved auto-hide settings; `ui-shared/panels.rs` | app shell share of 73 / 32 |
| **Layout** | `crates/layout` (18 files) and the Layout files in `web/src/presentation` (`layout_*`, `inspector/layout_component_inspector.rs`, `inspector/companion_placement.rs`) | 54 / 77 plus its share of the shell |
| **Case** | `crates/case` (8 files; `mechanical_settings.rs` alone has 86 `Board*` uses) and `presentation/case_workspace.rs` | 49 / 121 |
| **Parts** | `crates/parts` (17 files) and `presentation/parts_workspace.rs` | 65 / 64 |
| **PCB, Keymap, Keycaps** | `crates/pcb` (9), `crates/keymap` (5), `crates/keycaps` (4) and their `presentation/*_workspace.rs` | 49 / 84 |
| **Library and Export** | `crates/library` (1), `presentation/export_workspace.rs`, `zmk_firmware_export.rs`, `setup_workspace.rs` | 19 / 12 plus its share |
| **Cleanup** (last) | delete `Board*`, the remaining `.board-*` / `.m1-*` / `--wb-*`, update DESIGN.md and the README | n/a |

**Order.** Foundation first; then Shell and Layout (Layout editing is the stated priority) with Case as the third parallel lane, at
most 3 lanes at once; then Parts, the small workspaces, and Library and Export; Cleanup last.

**Ownership.**
- A lane only adds files under `ui-shared/src/ui/`; it edits one the foundation created only with a note in its description.
- Each lane deletes only the old CSS rules that its own files used. Rules shared between lanes are left to Cleanup.
- `presentation.rs` belongs to the Shell lane; other lanes touch it only for their own workspace.
- Until Cleanup, `Board*` stays on the migration branch (so lanes can merge one at a time); it never reaches `dev`.

## 3. Test rewrite

- Tests find controls by **accessible role and name**, reusing today's `aria-label`s (115 lines in 12 files), through the foundation's
  helper. Never by `.m1-*` or `.board-*` classes (144 lines in 15 files today) or by tag name (about 78 lines select `input`).
- Behaviour assertions stay as they are, except for the changes already decided: native `select` and `details` become custom
  controls, dock auto-hide is removed, and the Command palette and Toasts are new. Each lane lists such changes in its description.
- `scripts/check-native-tests-baseline.json` guards test names; a lane that renames or removes a test runs
  `check.py test --update-native-baseline` in the same commit. The wasm baseline file is currently `{}`.
- **Accessibility floor.** Every adopted interactive control (Select, Combobox, Collapsible, Dialog, Alert Dialog, Popover, Dropdown
  Menu, Tabs, Toggle Group, Checkbox, Slider, Tooltip) must pass the keyboard and ARIA checks from
  [accessibility-floor.md](accessibility-floor.md) (roles, arrows, Home/End, Enter/Space, Escape, focus return, a trap
  where needed) before its lane merges.

## 4. Old-CSS retirement

A repository check (added by the Foundation lane to `scripts/check.py`, so it runs in the normal verification) fails while any
`.board-*`, `.m1-*` or `--wb-*` remains in `web/`. Today: 622 selectors and 80 variables. It is expected to fail until Cleanup finishes,
so it gates the final merge only, not the lanes.

## 5. Merge gate for the single merge to `dev`

1. Full `scripts/check.py` passes.
2. The browser tests pass.
3. The accessibility checks from 010 pass.
4. The user reviews every workspace in both Themes and both Appearances at desktop (above 1150px), mid (981–1150px) and narrow
   (980px and below), with screenshots.
5. One two-axis review (`code-review`: Standards and Spec) of the whole branch.
6. The old-CSS check passes.
