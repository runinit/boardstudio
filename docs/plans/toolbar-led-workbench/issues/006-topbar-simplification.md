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

## Progress 2026-11-11 (builder; no closure claim)

All five proof rows are GREEN at their mounted/packaged tiers on this tree; the
ticket is not closed (no full quick gate, mutations or review were run, per
builder instructions). Production changes: quieter header hierarchy
(non-current workspace buttons recede to muted text in `header.css`; all named
actions, routes, `aria-current`, thresholds 1250px/520px and the focusable
save status are retained), Export/project-browser separation (Editor no
longer composes Objects/Inspector content for Export, whose fall-through
previously supplied Layout Objects; header panel controls are suppressed
while the project browser is open, and the hidden Editor keeps its lifetime),
route-first Export reveals (Review wiring/Review case set the target
workspace before revealing its current legacy Inspector authority, with the
existing selection/anchor clearing unchanged; the InspectorActions shortcut
keeps its workspace/generation/disclosure/focus checks), and save honesty
(the Layout toolbar's save-failure/recovery block moved to a new
`SaveRecoveryNotice` in shell status space below the global header, fed by
the Runtime's durability/lifecycle/conflict state with the existing
RetrySave/download/recover handlers; failed writes show "Save failed", never
Saved). Row receipts: Global reach — new mounted header traversal
(`006-final-mounted.log`, 8/0 across header/export/shortcut owners) plus
packaged agent-browser checks at 1440/320 (header 56px, tool row 42px, no
overflow, compact Workbench menu contains all workspaces/Search/history/
panel/Settings/Export, save label hidden at 320); Separate pages — mounted
Export assertion of no panel hosts plus packaged Export (no dock/panel/tool
row, header present) and return to Layout (editor retained, no panel or
ghost focus); Explicit reveals — new mounted
`export_review_routes_first_then_reveals_target_inspector` and the existing
shortcut test in `006-final-mounted.log`; Save honesty — RED
`006-save-red.log` then GREEN (`006-save-green1.log`, final rerun in
`006-final-mounted.log`), conflict state offers Download without a blind
Retry; Update — the existing header update-notice tests pass unchanged and
the App's `begin_update(Header)` wiring is untouched. Packaged:
`006-build.log`, `scripts/test-workbench-viewport.py` PASS at 320
(`006-viewport-320.log`) and 1440 (`006-viewport-1440b.log`; first 1440
attempt timed out on the new-project guide and passed on retry,
`006-viewport-1440.log`). Affected mounted owners rerun:
`006-affected-green.log` (layout_workspace + workbench_shell, 9/0).
`cargo fmt --all --check` and `python3 scripts/check.py lint` pass
(`006-fmt-final.log`, `006-lint-final.log`, under the build lock). Limits:
1250px exactly was not separately driven in the packaged browser (compact
behaviour proven at 320 and via the existing 1250px match-media/CSS);
project-browser packaged traversal relied on the mounted header proof and
the pre-existing header suppression of workspace navigation. No closure
claim.

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`,
`python3 scripts/run-wasm-tests.py --files web/src/presentation/header.rs` plus existing
status/shortcut page owners as affected. Save-honesty proof-gap mutation is mandatory if
existing evidence does not already demonstrate the row. Preserve tests/assertions, not CSS names.
