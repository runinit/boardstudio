# 017: Case settings, part 1: dimensions, construction and profiles

Status: claimed
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The first half of the Case mechanical settings: dimension fields, construction controls, profile guidance and the profile geometry editors. The settings file is the largest single sweep (86 `Board*` uses), so it is split here. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The dimension, construction, profile and resolved-stack groups of the settings have zero raw controls and `Board*` uses, with precise numeric entry unchanged
- [ ] Existing tests for these groups pass after the rewrite rule
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Claim and ownership

- Bound lane: `feat/rust-ui-017-case-settings`, `/home/chris/.t3/worktrees/ts-boardstudio2/t3-111fcd9a`; clean initial `b237903705244a266b07a64e5f23b8a3b5219c03`, fast-forwarded before claim to local migration `f11e6bd84456b94b2d7657f57a0d05b42184c4e7`.
- Exclusive target: `/home/chris/.cache/boardstudio-target/017-case-settings-1-lineage`, linked as `target`; copied ignored sandbox settings, `cad/.cache` and `cad/wasm/pkg` per lane protocol. No inherited 015/root target.
- Owned production hunks in `web/crates/case/src/mechanical_settings.rs`: `ConstructionControls`, `MechanicalLegacyConstructionControl`, `ProfileGuidance`, `ProfileGeometryEditor`, profile polygon/volume editors, `DimensionControls`, `ResolvedStack`, and their enclosing profile/resolved disclosures.
- Shared internal hunks: imports plus `DimensionField` and `TextDraftField` markup only, keeping their domain targets, draft/settlement controllers, parsing and callbacks intact for 018 callers. No public API/visibility/dependency changes planned.
- Owned checks: Case mounted tests and a role/name repository browser test of the adopted production controls. Any shared test registration or CSS coexistence hunk will be inventoried before editing. Shared legacy CSS stays for 024; 018 groups and 019 viewers/rest stay outside this ticket.
- Concurrent lanes: 016 Layout and 020 Parts library. Parked extraction is complete; only an actual matching active writer blocks work. Landing waits for one substantive coordinator Standards+Spec closure at a clean source pin.
