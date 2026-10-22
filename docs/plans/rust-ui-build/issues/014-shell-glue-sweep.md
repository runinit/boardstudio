# 014: Shell glue sweep

Status: ready-for-agent
Type: build
Blocked by: [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md), [012: Command palette](012-command-palette.md), [013: Toasts](013-toasts.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The remaining shell files have no raw controls and no `Board*` left.

## Acceptance criteria

- [ ] The inventory's app-shell rows that belong to the shell (workspace navigation, shortcuts, composition, inspector host, status footer, empty canvas) have zero raw controls and zero `Board*` uses
- [ ] Messages are classified by the mapping's rule; icon-only buttons have Tooltips; selects follow the 12-option rule
- [ ] The shell works in both Themes and both Appearances at desktop, mid and narrow widths, with screenshots recorded in the outcome
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
