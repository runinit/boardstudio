# 024: Delete the old layer

Status: ready-for-agent
Type: build
Blocked by: [014: Shell glue sweep](014-shell-glue-sweep.md), [016: Layout sweep](016-layout-sweep.md), [018: Case settings, part 2: mounts, openings, battery, manufacturing and pairs](018-case-settings-2.md), [019: Case rest](019-case-rest.md), [020: Parts library and inspector](020-parts-library-inspector.md), [021: Parts editors](021-parts-editors.md), [022: PCB, Keymap and Keycaps](022-pcb-keymap-keycaps.md), [023: Library and Export](023-library-and-export.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [branch plan](../../rust-ui-migration/branch-plan.md), [ticket 005](005-old-css-check.md), [shared contract](../map.md#shared-contract)

## What to build

The `Board*` wrappers and the legacy styling are gone, because no caller remains.

## Acceptance criteria

- [ ] `Board*` and its module are deleted; every `.board-*`, `.m1-*` rule and `--wb-*` variable is removed, and layout rules that were not component styling live in the new styling
- [ ] The old-CSS check in strict mode passes
- [ ] The full build and the native, browser and page checks pass; the native test-name baseline is updated deliberately
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
