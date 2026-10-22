# 002: Complete Theme system

Status: ready-for-agent
Type: build
Blocked by: [001: Walking skeleton: one copied Button, both Themes, one migrated control](001-walking-skeleton.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [theme contract](../../rust-ui-migration/theme-contract.md), [ADR-0034](../../../adr/0034-adopt-rust-ui-components-with-two-themes.md), [shared contract](../map.md#shared-contract)

## What to build

Both Themes are complete in light and dark: every token the copied components and the shell need, so later tickets only consume them. The workbench page gets the desk background that makes cards read, and shared control sizes replace upstream's fixed heights.

## Acceptance criteria

- [ ] Every token in the contract's core mapping, extras (`canvas`, `selected`, `selected-foreground`, `field`, `border-strong`, `toolbar`, geometry, grid, keycap, `--icon-stroke`) and authored values (success, warning, info fills, `destructive-foreground`, slider and scrollbar colours) exists for Original and shadcn in light and dark; Original's mapped values equal today's Graphite values per the contract
- [ ] `background` is the desk colour and `card` the card colour in both Themes, as in the contract's cards style
- [ ] Inter Variable (latin) is bundled with its OFL licence entry in the licence file; Source Sans 3 and Source Code Pro stay for Original
- [ ] The shared control sizes (40px default, 36px compact, 44px touch) are tokens identical in both Themes, and the copied `Button` reads them
- [ ] The 2D canvases follow the Theme through the geometry tokens; the 3D renderer keeps its own palettes and follows Appearance only
- [ ] `--wb-*` variables still exist (they retire in the Cleanup ticket); nothing in the running app regresses visually in Original
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
