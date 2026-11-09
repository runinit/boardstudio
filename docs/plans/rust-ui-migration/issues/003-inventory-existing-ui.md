# 003: Document the existing UI building blocks and control sites

Status: resolved
Type: task
Blocked by: None
Parent: [Map](../map.md)

## Question

Produce `docs/plans/rust-ui-migration/inventory.md`: (1) every general-purpose building
block (`Board*` controls, panels, layer toggles, `ThemePicker`, dock, project menu…) with
its props, behaviour it owns, and call-site count; (2) every plain HTML control site
(`button`, `input` by type, `select`, `textarea`, `details`, `dialog`, `label`) grouped by
file and recurring pattern; (3) the app-specific components per workspace as a sweep
checklist, marking canvas/3D ones out of scope. Starting point: 225 components in 85 files
(`.evidence/component-inventory.txt`, gitignored, regenerable with `grep -A3 '#\[component\]'`).

## Answer

Written to [inventory.md](../inventory.md): totals, the 11 `Board*` controls and the other shared
components with what each owns, the raw HTML control sites, the recurring patterns that each need one
mapping decision, the canvas and 3D components ruled out of scope, and a 88-file sweep checklist by
workspace (case, keycaps, keymap, layout, library, parts, pcb, ui-shared, app shell).

Headline facts later tickets depend on: 210 production components (14 test fixtures excluded); 397
`Board*` uses and 331 raw control elements, so wrappers cover only about half of the controls
(buttons 357 in total, selects 105, all native); `case/mechanical_settings.rs` alone holds 86 `Board*`
uses; `Board*` is built on native `select`, `details` and `dialog` as well as `dioxus-primitives`;
status and alert text (`role="status"` / `"alert"`) appears 246 times with no shared component.

Method correction worth knowing: counting without excluding test code gives 225 components and
inflated control counts, and the obvious shortcut of cutting each file at its first `#[cfg(test)]`
is also wrong (32 files have test code early and production code after it). The inventory masks only
`#[cfg(test)] mod … { }` blocks. Canvas classification is by component name and is flagged for
[011](011-target-workbench-structure.md) to confirm.
