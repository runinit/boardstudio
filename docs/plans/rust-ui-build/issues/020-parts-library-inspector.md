# 020: Parts library and inspector

Status: claimed
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The Parts library, previews and inspector. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The Parts library, chooser, preview and inspector rows of the inventory have zero raw controls and zero `Board*` uses
- [ ] Existing Parts tests pass after the rewrite rule; the workspace works in both Themes and both Appearances at the three widths
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Claim and ownership (2026-10-22)

- Bound T3 lane: `feat/rust-ui-020-parts-inspector`, `/home/chris/.t3/worktrees/ts-boardstudio2/t3-1e8d1a7b`.
- Initial clean base `b237903705244a266b07a64e5f23b8a3b5219c03` fast-forwarded to local integration `f11e6bd84456b94b2d7657f57a0d05b42184c4e7` before claim.
- Own `web/crates/parts/src/parts.rs` library/chooser/inspector controls, `parts/preview.rs` preview controls, `parts/modules_catalogue.rs` inspector controls, `parts/modules_catalogue/module_attachment.rs` attachment controls, and `web/src/presentation/parts_workspace.rs` Parts-only composition. Their existing tests and a scoped repository browser test belong to this slice.
- Keep editor implementations and their draft/validation/commit owners (021), canvas/rendering internals, Layout (016), and Case settings (017) outside this slice. No public API, visibility or dependency changes are planned.
- Shared-hunk inventory before implementation: preserve `parts.rs` module declarations, imported editor bindings and child editor calls; only change its owned controls and relevant tests. Shared legacy CSS and shared controls stay for 024/other owners unless a concrete overlap is established.
- Exclusive build target: `/home/chris/.cache/boardstudio-target/020-parts-library-inspector-lineage`; no inherited/root/015 target. Hold local integration landing for coordinator Standards + Spec approval of a clean implementation pin.
