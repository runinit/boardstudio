# 015: Layout toolbar strips

Status: ready-for-agent
Type: build
Blocked by: [007: Accessible action menus](007-accessible-action-menus.md), [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [mapping](../../rust-ui-migration/mapping.md), [structure decision](../../rust-ui-migration/issues/011-target-workbench-structure.md), [shared contract](../map.md#shared-contract)

## What to build

Layout's two fixed strips above the canvas are rebuilt on Toolbar, Toggle Group and Dropdown Menu, with Snap staying in the canvas toolbar. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The selection scope strip and the contextual action strip behave exactly as today (scopes, Position, Stagger, Offset, Splay, Align, Transform, Snap and Geometry snap) and keep the Stationary Controls Rule
- [ ] Soft filled groups replace outlined buttons (cards style); the strips do not switch density or scroll horizontally at the supported widths
- [ ] The Layout toolbar tests are rewritten per the test rule and pass; the strips pass the floor
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
