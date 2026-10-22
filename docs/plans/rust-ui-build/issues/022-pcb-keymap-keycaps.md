# 022: PCB, Keymap and Keycaps

Status: ready-for-agent
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The three smaller Workspaces together, with Tabs for the keymap layers. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The PCB, Keymap and Keycaps rows of the inventory and their Workspace hosts have zero raw controls and zero `Board*` uses
- [ ] Keymap layer navigation is Tabs and passes the floor; existing tests pass after the rewrite rule
- [ ] Each Workspace works in both Themes and both Appearances at the three widths
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
