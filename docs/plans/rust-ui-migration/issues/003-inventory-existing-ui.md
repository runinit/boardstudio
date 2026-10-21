# 003: Document the existing UI building blocks and control sites

Status: open
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
