# 008: Accessible Toolbar, Toggle Group, Slider and Toast

Status: ready-for-agent
Type: build
Blocked by: [003: Base component set](003-base-component-set.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [accessibility floor](../../rust-ui-migration/accessibility-floor.md), [shared contract](../map.md#shared-contract)

## What to build

The remaining controls with measured gaps pass the floor.

## Acceptance criteria

- [ ] Toolbar has one tab stop with arrow roving, and End reaches the last item
- [ ] Slider supports Home (minimum) and End (maximum) and PageUp and PageDown
- [ ] Toggle Group requires a group label and exposes a group role
- [ ] Toast items are announced with `alert` or `status`, not `alertdialog`, with a reachable dismiss button
- [ ] Repository browser tests cover the floor rows and pass
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
