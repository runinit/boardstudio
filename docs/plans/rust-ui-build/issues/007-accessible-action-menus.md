# 007: Accessible action menus

Status: claimed
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

## Estimate (2026-10-22)

The copied source now exists and was inspected at local base `ec6318c6d`. It forwards to pinned
`dioxus-primitives` DropdownMenu. That primitive's menu context and ordered focus collection are
private; no public hook lets the copy request initial focus or typeahead. It also couples open state
to that private collection's focus. Merely overriding ARIA attributes cannot meet the floor.

Estimated scope: keep the copied styling and public props, use the existing public Popover primitive
for overlay lifecycle/dismissal, and add menu-specific focus/navigation in this copy. Checkbox and radio
items can use the existing item's role/checked attributes with caller-owned selection. No dependency,
visibility change, Select/Combobox edit or application menu migration is needed. This is a coherent
007-sized adaptation if the representative mounted floor passes. The permitted fallback remains the
existing application Popover menus if that cannot be demonstrated; it is not a claim those menus
already pass the action-menu floor.
