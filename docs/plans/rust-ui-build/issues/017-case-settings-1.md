# 017: Case settings, part 1: dimensions, construction and profiles

Status: ready-for-agent
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The first half of the Case mechanical settings: dimension fields, construction controls, profile guidance and the profile geometry editors. The settings file is the largest single sweep (86 `Board*` uses), so it is split here. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The dimension, construction, profile and resolved-stack groups of the settings have zero raw controls and `Board*` uses, with precise numeric entry unchanged
- [ ] Existing tests for these groups pass after the rewrite rule
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
