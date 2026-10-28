# 011: Docks and narrow-screen sheet

Status: ready-for-agent
Type: build
Blocked by: [009: Dock primitives](009-dock-primitives.md), [010: Header card](010-header-card.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [structure decision](../../rust-ui-migration/issues/011-target-workbench-structure.md), [ADR-0035](../../../adr/0035-rebuild-the-workbench-shell-on-rust-ui-patterns.md), [shared contract](../map.md#shared-contract)

## What to build

Objects and Inspector are cards on the new docks, with pinned and collapsed modes, and below 980px every Workspace shows them as one Sheet with Objects and Inspector tabs, leaving the canvas visible and interactive.

## Acceptance criteria

- [ ] Objects (left) and Inspector (right) use the dock primitives as cards on the desk; widths and mode persist per side
- [ ] Auto-hide is removed; a saved auto-hide preference becomes pinned without losing the user's widths
- [ ] Below 980px every Workspace uses the same non-modal Sheet with Objects and Inspector tabs, replacing the separate drawer and scrim behaviour of the other Workspaces
- [ ] Layout's Add action still opens its create view inside the Objects dock
- [ ] Existing dock and panel tests are rewritten per the test rule and pass; the removed auto-hide tests are deleted explicitly
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
