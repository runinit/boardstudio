# 007: Accessible action menus

Status: ready-for-agent
Type: build
Blocked by: [003: Base component set](003-base-component-set.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [accessibility floor](../../rust-ui-migration/accessibility-floor.md), [mapping](../../rust-ui-migration/mapping.md), [shared contract](../map.md#shared-contract)

## What to build

Dropdown Menu is usable for the Project, Settings, Transform and Layout command menus. This is the largest unknown in the plan: estimate it first and record the finding in this ticket.

## Acceptance criteria

- [ ] The menu uses `menu` and `menuitem` roles (and the checkbox and radio variants), focuses the first item when opened, supports arrows, Home, End and typeahead, activates with Enter and closes
- [ ] Escape and item activation return focus to the opener
- [ ] Repository browser tests cover the floor rows and pass
- [ ] If the fix proves larger than one ticket, this ticket records that and the documented fallback (keep the `Popover`-based menus used today) is adopted, with the decision noted in the plan map
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
