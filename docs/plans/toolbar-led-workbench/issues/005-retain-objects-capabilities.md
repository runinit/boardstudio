# 005: Retain Add, setup, placement and legacy overflow workflows

Status: ready-for-agent
Type: build
Blocked by: [003](003-inspect-popout.md), [004](004-objects-popover.md), [006](006-topbar-simplification.md)
Parent: [map.md](../map.md)
Owner: Layout/PCB shared Add/setup entry mounts and existing overflow task hosts

## Boundary

Implement the M1 recommended [overflow routing table](../inventory.md#overflow-routing-proposal)
adopted by the execution instruction in [delivery.md](../delivery.md). Later domain-specific
entries remain with their M2 tickets, not an implied new policy.
Project menu → Add object… and Command palette reach the same contextual AddObjectEntry
commands; 006 owns final header styling, the coordinator owns its routing hunks. Creation
stays outside the strip and compact Objects. Reuse setup-guide/board/topology, MatrixSetupMount
with real canvas placement preview, mirrored-pair creation, ExistingHalfSetup, one-shot
Add row/column, AddObjectComponentChooser search/place/error and Browse all parts route.
Do not build a new setup wizard or replace accepted-state edit paths.

Legacy GeometryScriptsEditor moves to a bounded larger task surface, with existing execution,
errors, atomic apply and safety unchanged. Deprecated ADR-0029 remains out-of-product-scope;
this is access preservation pending separate approved removal, not new scripting support.
Board outline selects the active boundary; lifecycle/Findings remain in Inspect. Keep current
confirmations/file pickers and setup-guide lifetime/preview interactions. New task hosts do
not add a backdrop, permanent row or canvas resize.

This is the first real task/registered-owner integration. Keep MatrixSetup rows/columns/
preset signals and its `DraftOwnerGuard` alive together while hidden or parked; legacy script
text/error/pending ownership follows the same rule. Use keep-mounted retained editor scopes
rather than a new cross-crate draft API for M1 where current controllers permit it. Intentional
Cancel still cancels; ordinary utility hiding does not call Cancel. Existing pending Create
admission exempts only its submitted values and cannot be recreated by showing the task.
Off-screen update deferral must identify Layout and the actual editor/remedy, not just an
unlabelled field. Do not add a pending-close veto absent from the original controller.

## Acceptance criteria

- [ ] Every Layout M1 overflow entry ticked with new entry point and smallest successful
      workflow; later domain-specific entries remain explicitly assigned to M2 tickets.
- [ ] No Layout action reachable only from old `m1-add-object-trigger` / Inspector dock after
      Layout migration; other workspaces retain their sole current host until M2.
- [ ] Failed/cancelled/pending flows retain their errors/guards and do not create extra edits.
- [ ] PCB can reuse these entry/placement mounts without acquiring a different scope authority.

## Proof map

- Setup parity — valid empty/split fixture → use new entry for board/topology, matrix,
  mirrored pair and Mirror existing half workflows (equivalent fixtures parameterized) →
  accepted result matches existing intent, created branches/selects retained, one Undo per
  existing one-shot restores prior document · mounted · planned
- Context action — selected matrix with row/column admission → Add row/column through chooser →
  correct owner changes once, pending repeat blocked and Undo restores · mounted · planned
- Placement preview — matrix/part chooser open before commit → change fields/pick location →
  real preview and accepted pose agree; cancel produces no edit · mounted · planned
- Browse/return — query entered in migrated Layout chooser → Browse all parts using the old
  Parts shell, choose controller/return through old PCB and back to Layout → query/placement
  context survives, each command reaches the current host, browsing creates no placement · mounted · planned
- Hidden draft safety — actual MatrixSetup fields differ from their baseline before Create →
  hide for Inspector, switch workspace and request Update Now → existing Layout/editor remedy
  defers reload; return restores exact fields and Create/explicit Cancel clears only this
  blocker through existing rules · mounted · planned
- Legacy draft parked — saved script body edited but not applied before hide → visit Inspector/
  another workspace and return → exact script text/error and live update-safety remedy survive;
  hiding executes/applies nothing, later explicit Apply retains existing atomic admission · mounted · planned
- Registration scope — retained MatrixSetup owner before project/session replacement → replace
  session → old registration cannot block new session, current unresolved owner still does;
  no restored old-scope predicate or admission · mounted · planned
- Pending task admission — real MatrixSetup Create held pending → hide/show and attempt Create
  again → no second submission or refreshed admission; first result retains its existing
  captured target/settlement, and domain Cancel keeps its original semantics · mounted · planned
- Domain task close — real MatrixSetup shown → use its own Cancel or navigate via the existing
  accepted route → task visibility and host agree, no old panel auto-reopens or late focus;
  a domain refusal, when present, cannot be overridden by the host · mounted · planned
- Failure — rejected placement while flow open → submit → message stays visible at its new
  home, accepted document unchanged and retry remains possible · mounted · planned
- Legacy script access — existing supported saved script/action → open retained editor, execute
  and explicitly apply smallest existing result → current diagnostics/safety preserved,
  atomic accepted result/Undo unchanged; rejection never applies a partial result · mounted · planned

## Progress 2026-11-11 (Part A builder; no closure claim)

Part A rows (Add/setup/placement routing, bounded task surface, Reach and the
inherited field-preview red) are GREEN at their mounted tiers; Part B rows
(hidden draft safety, legacy script surface, registration scope, pending
admission, domain close) remain planned for a separate run and were not
attempted beyond what Part A needed. Known-red resolution: tracing the setup
guide's Add Thumb cluster and the Add chooser into `MatrixSetupMount` and its
preview publisher showed the route did reach the migrated shell; the canvas
ghost never painted because the page's `setup_preview_frame_px` measurement
in `web/src/presentation.rs` treated the first `[aria-label='Design view']`
(the 42px toolbar's View toggle, now above the canvas) as the retired
bottom-of-canvas overlay, clamping the clear height to zero, so
`MatrixSetupCanvasPreview` received a zero-height frame and returned nothing.
The measurement now only reserves space for controls whose rect actually
overlaps the canvas. A second redesign gap closed in passing: in the bare
accordion lane the inline contextual actions had lost their `Actions for …`
accessible group, restored with a `display:contents` group.

Routing: Project menu → Add object… (new `AddObjectRequest` counter in
ui-model, provided by the App) routes to Layout first, then opens the same
Objects/Add menu the existing command-palette entry opens; choosing Matrix…
or a component placement from that menu now closes the compact Objects popout
so the bounded workspace surface and real canvas preview are visible (no
backdrop, no permanent row, no canvas resize); Mirror existing half keeps its
surface inside the Add flow, and the geometry-scripts entry again reveals
its retained editor through Layout's sole Inspect host (its larger Part B
surface is untouched). New proofs: library
`project_menu_add_object_bumps_the_shared_request`; page
`add_object_request_reveals_the_add_menu_and_matrix_surface` (menu → Matrix… →
popout closed, 30-cell ghost, no revision, Cancel retires),
`add_row_and_column_through_the_add_menu_change_the_owner_once` (one owner
 change each, Undo restores; pending-repeat rests on the same
`PendingEditSignals` one-shot that disables the menu buttons while pending),
and field-file `browse_all_parts_keeps_the_chooser_query_and_creates_nothing`.
Setup parity/context/placement/failure reuse the layout-crate browser suite
(272 passed: setup guide, MatrixSetup create/preview, mirrored pair,
ExistingHalfSetup, add-matrix surface, part/matrix placement rejections
without mutation) and the parts-crate suite (106 passed: chooser,
Browse-all-parts query preservation, placement actions), plus the field
suite's split/unibody Create-and-land and preview-pose assertions. Existing
field-file assertions that referenced retired shell mechanics were migrated,
not weakened: Design-view bottom bound → full canvas containment (002 moved
Design view to the tool row), dock restore/wait selectors → the retained
workbench panels, focus flow now opens the Select section/Inspect popout a
real keyboard user opens first. Field suite is 7/7 (25/25 with
layout_layers). Limits: the Browse/return PCB leg steps through the
 workspace signal in the page fixture (the old Parts/PCB shells themselves
are covered by their crate/page suites); no mutations, reviews or full quick
gate were run (coordinator owns). Logs: `.evidence/toolbar-m1/005a-*`;
checkpoint `.evidence/toolbar-m1/005a-checkpoint.md`. `cargo fmt --all --check`
and `python3 scripts/check.py lint` pass on the stable tree.

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`, layout/parts browser
subsets and changed setup/placement page files. Reuse existing workflow/native safety proof;
add only new entry/host wiring. Named missing-entry and placement-error proof-gap mutations
must fail at reachability/error assertions, not compilation. Hidden-registration, old-scope,
pending-repeat and owner-close wiring mutants use the real page task suite named there.
Reuse MatrixSetup's existing `matrix_real_create_pending_exempts_only_submitted_draft` where
sufficient; no duplicate runtime pruning fixture. Do not remove setup/script guards.

## Known red inherited from M1 coordination (2026-11-11)

`presentation::layout_field_preview_tests` (6 tests; first failure
`new_matrix_sliders_preview_on_canvas_without_changing_accepted_state` / `new_matrix_canvas_preview_avoids_the_resized_setup_guide`)
fails 0/6 on the redesigned Layout shell: the New matrix canvas ghost never appears. Cause
unconfirmed by two diagnoses (`.evidence/toolbar-m1/field-diagnosis.log`, `field-objects-probe.log`);
opening Objects first did not help. Presumed owner: this ticket (Add chooser / `MatrixSetupMount`
route and preview publisher). `check.py quick` therefore stays red on this file until 005 restores
the real route; do not skip or weaken those tests. Trace first: setup-guide "Add Thumb cluster" and
the Add chooser into `MatrixSetupMount` and its preview publisher.
