# 010: Header card

Status: ready-for-agent
Type: build
Blocked by: [002: Complete Theme system](002-complete-theme-system.md), [004: Vendored Lucide icons](004-lucide-icons.md), [007: Accessible action menus](007-accessible-action-menus.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [ADR-0035](../../../adr/0035-rebuild-the-workbench-shell-on-rust-ui-patterns.md), [structure decision](../../rust-ui-migration/issues/011-target-workbench-structure.md), [shared contract](../map.md#shared-contract)

## What to build

The header becomes a 56px borderless card with the project identity and Project menu, the Workspace links, the Command search button, save status, undo and redo, one theme control and Export. A user can switch Theme and Appearance from the header.

## Acceptance criteria

- [ ] Project and Settings are Dropdown Menus passing the floor; the Appearance and Theme choices are two radio groups in one theme control, replacing the temporary placement from ticket 001
- [ ] Workspace links are ghost buttons keeping `aria-current` on the active one; Export is the primary button; undo and redo and save status keep their behaviour
- [ ] The Command search button is present and inert until ticket 012 wires it
- [ ] The header holds up at desktop, mid and narrow widths with the mobile menu; existing header tests pass after the rewrite rule
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
