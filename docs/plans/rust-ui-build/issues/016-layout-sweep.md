# 016: Layout sweep

Status: ready-for-agent
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
