# 018: Case settings, part 2: mounts, openings, battery, manufacturing and pairs

Status: ready-for-agent
Type: build
Blocked by: [017: Case settings, part 1: dimensions, construction and profiles](017-case-settings-1.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The second half of the Case mechanical settings: mounting and mount collections, openings, battery, manufacturing and the paired-feature controls. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The remaining groups of the settings file have zero raw controls and `Board*` uses
- [ ] The settings file as a whole shows zero in the inventory's counts; existing tests pass after the rewrite rule
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
