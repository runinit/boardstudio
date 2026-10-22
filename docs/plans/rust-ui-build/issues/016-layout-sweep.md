# 016: Layout sweep

Status: claimed
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [015: Layout toolbar strips](015-layout-toolbar-strips.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [mapping](../../rust-ui-migration/mapping.md), [shared contract](../map.md#shared-contract)

## What to build

The rest of the Layout workspace is migrated. The Add view and Mirror existing half become in-dock views instead of non-modal dialogs. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] Every Layout row of the inventory (the layout crate and the Layout files in the app shell) has zero raw controls and zero `Board*` uses
- [ ] Add and Mirror existing half work as in-dock views with the same operations, undo and measurements as today
- [ ] Layout works in both Themes and both Appearances at the three widths, with screenshots in the outcome
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Ownership (016 lineage)

Bound lane: `/home/chris/.t3/worktrees/ts-boardstudio2/t3-721450bb`, branch
`feat/rust-ui-016-layout-sweep`; clean initial `b237903` fast-forwarded to migration
`f11e6bd84456b94b2d7657f57a0d05b42184c4e7` before claim `2ff0be1`.

- Layout production files: `board_inspector.rs`, `layout_findings.rs`, `layout_viewer.rs`,
  `objects.rs`, `objects/{existing_half,keycap_size,matrix_inspector,matrix_setup,matrix_transform_inspector,mirrored_pair}.rs`,
  `outline/inspector.rs`, `setup_guide.rs`. The 015 toolbar/align files and canvas/3D drawing surfaces retain their behavior.
- Shell Layout files: `presentation/{layout_workspace,layout_key_selection_actions,layout_matrix_relations}.rs`
  and `presentation/inspector/{layout_component_inspector,companion_placement}.rs`.
  `presentation.rs` only Layout placement/splay guidance or Layout-specific action markup if needed;
  no command registry, callback ownership or other Workspace hunks.
- Shared lease: `ui-shared/src/canvas_layers.rs` only `LayerPanel` and `LayerToggleButton`
  plus their tests. Existing props/targets/visibility owners stay intact; 020 reuses the landed adoption.
  Case `shared_viewer.rs` / `DesignViewToolbar` is outside this lease.
- Tests: affected Layout crate mounted tests and Layout presentation mounted tests, their role/name
  locators, new adopted-control coverage; native test-name baseline only if a native name changes.
- CSS: only selectors exclusive to these files. Shared legacy selectors wait for 024.
- Proposed shared event-forwarding props and the mapping's missing `NumberField` composition await
  coordinator authorization before edits. No dependencies or member visibility changes proposed.

Exclusive generated target: `/home/chris/.cache/boardstudio-target/016-layout-sweep-lineage`;
ignored `.claude/settings.json` and CAD caches copied before builds. No Gortex or CodeGraph directory.
Historical workflow documents are references only, per the current user instruction.
