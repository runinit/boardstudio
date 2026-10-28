# 009: Dock primitives

Status: ready-for-agent
Type: build
Blocked by: [002: Complete Theme system](002-complete-theme-system.md), [003: Base component set](003-base-component-set.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [ADR-0040](../../../adr/0040-rebuild-the-workbench-shell-on-rust-ui-patterns.md), [spec](../../rust-ui-migration/spec.md) section 3, [shared contract](../map.md#shared-contract)

## What to build

The two building blocks of the new docks exist and work on their own: a non-modal Sheet and an in-flow Sidenav dock with its own resize.

## Acceptance criteria

- [ ] Sheet opens from the edge it is given with no backdrop and no scroll lock, so the canvas stays visible and interactive; Escape closes it and returns focus
- [ ] Sidenav's `collapsible: None` form is an in-flow dock that sits under the header (not fixed to the viewport) at the desktop widths
- [ ] A dock resizes by pointer drag and by arrow keys, within its minimum and maximum, and each side saves its own width (no shared key)
- [ ] Repository browser tests cover resize, persistence per side and the Sheet's non-modal behaviour
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
