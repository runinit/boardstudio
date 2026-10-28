# 012: Command palette

Status: ready-for-agent
Type: build
Blocked by: [004: Vendored Lucide icons](004-lucide-icons.md), [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [010: Header card](010-header-card.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [command palette](../../rust-ui-migration/command-palette.md), [palette decision](../../rust-ui-migration/issues/012-command-palette-contents.md), [shared contract](../map.md#shared-contract)

## What to build

Ctrl/Cmd+K opens a searchable palette of the workbench's existing actions and objects, as specified.

## Acceptance criteria

- [ ] The palette follows the command palette spec: the groups, object search by name, hidden commands for other Workspaces, disabled-with-reason for unavailable ones, and no destructive or numeric edits
- [ ] It opens from the header button and from Ctrl/Cmd+K anywhere, including inside a field, which blurs as on a click-away; Escape closes it first
- [ ] Undo and redo show their existing hints; the Alt, Shift and Ctrl pointer modifiers are documented as `Kbd` hints in tooltips on the controls they affect
- [ ] Built on Dialog and Combobox behaviour with no script; it passes the floor; every command dispatches the same event or UI change as the control it mirrors
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
