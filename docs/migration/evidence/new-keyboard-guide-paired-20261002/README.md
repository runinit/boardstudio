# New keyboard and setup guide: React oracle packet

This packet records the React half of the paired New keyboard/setup-guide
journey. The Dioxus half is pending root's integrated release candidate and is
not claimed here.

## Session and preservation

- Pinned React source: `5a472a9426e6e38993361da402cd4ec730feb369`.
- Browser URL: `http://127.0.0.1:5175/`.
- Named isolated agent-browser session: `new17-c8a853a97c72`.
- Viewport: initially the agent-browser default desktop viewport; compact
  checks used 640×900.
- Before interaction, localStorage was empty and the accessible library showed
  `Your keyboards 0`. No existing project data was in this isolated session.
  No storage was cleared. The created test project remains saved in the named
  session so the paired Dioxus run can compare behavior without deleting it.
- New-project identity: `16904bd7-baf5-472e-8cf8-908954e55f3f`.
- Board identity: `d2ae3a6c-e59c-4998-934b-baf2fbb38436`, name `Main board`.
- Project was named `New17 Oracle`; `boardstudio:v2:setup-guide:<id>` stored
  `open: true` and the current step. Initial New action showed `Untitled
  keyboard`, Project & hardware, `Main board`, an empty canvas, and the One / Split /
  Reversible controls.

## Source-backed defaults

`app/src/createProjectActions.ts` `blankProject()` uses `emptyProject` with a
fresh project UUID and `Untitled keyboard`, then creates one fresh-ID `Main
board` (1.6 mm), one empty part-envelope outline (margin 4, add operation,
default outline settings), and PLA material (3 mm). Parts, matrices, and case
bodies start empty. The physical setup chooser begins unresolved; its Project
stage detail asks the user to choose one or split keyboard until a physical
instance is accepted. `react-00-library.png`, `react-01-project.png`, and
`react-created-state.json` record the public initial state and generated ID.

## Journey observations

The New keyboard action created the project and opened the Project & hardware
guide. Enter committed `New17 Oracle`; the board remained `Main board`. From
Project, Continue to layout kept the guide open, set `currentStep` to `layout`,
and selected Design/Layout. The Layout screen exposed Add key matrix, Edit
existing objects, Continue to wiring, and Previous step. Continue to wiring
kept the guide open at `wiring` and selected PCB; its unresolved detail was
`Resolve the controller and board wiring.` Continue to case selected Case and
showed the Case stage; Continue to review selected Export and showed Review &
export. The guide persisted each selected stage. Finish guide set it closed;
Project → Setup guide reopened at the saved Review stage without resetting the
stage. The currentness route and stage state are recorded in the `react-24` to
`react-28` snapshots and JSON files.

The remaining stages' actual React content is captured in the matching `react`
DOM/snapshot packets: Wiring explanatory copy and action names, Case settings
copy and route, and Review readiness/copy plus Export options. Stage detail is
readiness-dependent: the empty layout reports two layout findings, wiring asks
for controller/board resolution, Case can report blocked mechanical findings,
and Review says to complete required steps. A candidate must not claim a stage
is ready just because a board exists.

## Name and compact panel cases

The Project name field is repeatable. Enter submitted `Transient Name`, then
Undo restored the field to the accepted `New17 Oracle` value, as shown by
`react-17-rename-transient.snapshot.txt`,
`react-18-undo-name.snapshot.txt`, and `react-18-undo-field.txt`.

At 640×900, selecting Case shows the Objects panel and a closed Case settings
panel. `Open case settings` closes Objects and opens Case settings. This is the
reference behavior the Dioxus guide action must retain on compact layouts. See
`react-19-compact-case-stage.*` and `react-20-compact-case-settings.*`.
An active-element probe after the action still reported `BODY` despite visible
enabled inspector controls. Because the clicked guide control is unmounted by
the compact panel transition, treat source-driven focus as uncertain until a
dedicated public focus check confirms it; panel visibility is the reliable
observation in this packet.

## Stale-base observation (RF-006)

During the first rapid Project-stage attempt, Split keyboard followed
immediately by checking Reversible layout produced the visible React alert
`Error: Stale base revision`. That first attempt did not record its exact
pre-action saved document revision, so the revision must not be inferred from
later database reads. This is one observed event, not proof of a persistent
bug or its cause.

After settling a saved Unibody/None project at revision 11, the same rapid
Split-then-Reversible sequence completed to revision 14 with split/wireless
physical instances and no alert. A separate settled Reversible toggle advanced
revision 7→8 without an alert. These repeats do not reproduce the earlier
failure. Treat an in-flight/stale-event race as a hypothesis only; do not waive
candidate stale-owner checks on the strength of this reference behavior. Exact
revision probes and snapshots are in `react-pre-race2.json`,
`react-post-race2.json`, and `react-23-settled-reversible-toggle.json`.

## Artifacts

`react-*.png` are screenshots. `react-*.snapshot.txt` are accessibility-tree
captures. `react-*.dom.txt` contain rendered stage copy and surrounding
workspace text. `react-*.json` files record storage/project identity and
read-only IndexedDB summaries. They all come from the named isolated browser
session above.
