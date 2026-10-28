# 023: Library and Export

Status: ready-for-agent
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [mapping](../../rust-ui-migration/mapping.md), [shared contract](../map.md#shared-contract)

## What to build

The project library and Export page, the project-delete confirmation as an Alert Dialog, and the setup guide with a vertical Stepper inside its four accordion sections. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The library, Export, firmware export and setup-guide rows of the inventory have zero raw controls and zero `Board*` uses
- [ ] Deleting a project uses an Alert Dialog that starts on the safe action and returns focus; the setup guide's current and ready steps use Stepper and stay freely navigable
- [ ] Existing tests pass after the rewrite rule; Export blockers stay inline
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
