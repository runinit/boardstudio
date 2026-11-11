# 006: Quieter global topbar without action or save-state loss

Status: ready-for-agent
Type: build
Blocked by: [009](009-shared-toolbar-panel-host.md)
Parent: [map.md](../map.md)
Owner: `web/src/presentation/header.rs`, `web/assets/header.css`, shell status presentation

## Boundary

Apply the approved quieter hierarchy on all six workspaces and project/Export surfaces
where relevant. Preserve identity/Project actions, six workspace links/aria-current, Search
Ctrl/Cmd+K, local-save control/popover, Undo/Redo, Settings Theme/Appearance, labelled Export
and update-available notice. Preserve compact Workbench-menu actions and focusable save
status (current thresholds 1250px / save label 520px), not obsolete panel mechanisms.
Panel commands route to the target workspace's sole current host: new Layout or the
unmigrated legacy adapter. 005's Add entry uses existing commands, not new domain edits.
Own the three shared reveal sites in `export_workspace.rs:109/:118` and
`workspace_navigation.rs:42` from the central architecture table. Route then reveal with
existing generation/disclosure/focus checks; Export does not acquire canvas panels.
Project browser may retain its existing navigation suppression until a project is open.

Move Layout save-failure/recovery presentation outside the 42px row into shell status/alert
space: Retry, Download pending and recovery Reload remain visible/reachable, including
narrow mode. No failed write may show Saved. Camera controls stay footer. Styling must not
alter save/export readiness, route ownership or native file-picker behaviour.

## Acceptance criteria

- [ ] All named entries/actions retained at desktop and compact widths, including update notice.
- [ ] Project/Export receive global header treatment, not irrelevant Objects/Inspect panels.
- [ ] Shell status can grow without changing toolbar's 42px height or hiding blocking failures.

## Proof map

- Global reach — desktop/1250/520/320 widths with project open → traverse Project, workspace
  chooser, Search, history, Settings and Export → correctly named/current actions reachable
  by keyboard, same routes, including compact menu and panel commands · mounted · planned
- Separate pages — project browser and Export active → navigate header and return → correct
  menu availability, no ghost canvas utilities/panel focus, original editor lifetime retained · mounted · planned
- Explicit target reveals — Export Review wiring/Review case and InspectorActions shortcut
  before target workspace settles → navigate/reveal → correct current target host/field,
  unchanged selection-clearing and generation guards, no Layout ghost or obsolete pin
  influence on migrated content · mounted · planned
- Save honesty — failed storage write or recovery-required state before rerender → open status
  and Retry/Download pending/Reload → failed/pending truth and existing recovery actions,
  never Saved until actual successful write · mounted · planned
- Update — update available with pending local work → use existing notice actions → existing
  offline/client-protection behaviour unchanged · mounted · planned

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`,
`python3 scripts/run-wasm-tests.py --files web/src/presentation/header.rs` plus existing
status/shortcut page owners as affected. Save-honesty proof-gap mutation is mandatory if
existing evidence does not already demonstrate the row. Preserve tests/assertions, not CSS names.
