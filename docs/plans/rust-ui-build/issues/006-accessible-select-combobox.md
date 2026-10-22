# 006: Accessible Select and Combobox

Status: ready-for-agent
Type: build
Blocked by: [003: Base component set](003-base-component-set.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [accessibility floor](../../rust-ui-migration/accessibility-floor.md), [mapping](../../rust-ui-migration/mapping.md), [shared contract](../map.md#shared-contract)

## What to build

Select and Combobox pass every row of the accessibility floor, so the 105 selects in the app can migrate to them.

## Acceptance criteria

- [ ] Select returns focus to its trigger after Escape and after choosing an option, and its open listbox has an accessible name taken from the Field label
- [ ] Select opens with Enter, Space and ArrowDown; arrows, Home, End and typeahead move; Enter selects; its trigger shows the option's label, not its raw value
- [ ] Combobox keeps focus on its input, moves `aria-activedescendant`, filters on typing, selects with Enter and closes with Escape
- [ ] Repository browser tests using the role-and-name helper cover every floor row for both controls and pass
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
