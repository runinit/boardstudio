# 005: Retain Add, setup, placement and legacy overflow workflows

Status: ready
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

## Progress 2026-11-11 (Part B builder; no closure claim)

Part B rows are GREEN at their mounted tiers. Production changes (all native
edits; Gortex read the ticket, indexed-file reads/edits fell back to native):
`use_matrix_setup` no longer retires its owner on a workspace round trip
(only scope/generation change — an accepted route into a new owner — closes
it); the Editor keeps `AddMatrixSurface` mounted in a hidden/inert wrapper
while parked for a compact panel or another workspace, so MatrixSetup's
rows/columns/preset signals and its `DraftOwnerGuard` live together; the
legacy `GeometryScriptsEditor` moved out of the Layout Inspector into a
keep-mounted bounded task surface in the Editor (absolute, bounded width,
no backdrop/permanent row/canvas resize; mounted on first reveal, then
retained), and `open_geometry_scripts` closes the compact panel instead of
revealing Inspect — ADR-0029 access preservation only, execution/errors/
atomic Apply unchanged. Ordinary utility hiding parks; only the domain
Cancel/Back or owner retirement closes.

New page proofs in `layout_draft_safety_tests.rs` (reusing the existing
FakeUpdateSignals/`mount_owner` fixtures; 38/38, `.evidence/toolbar-m1/005b-final.log`):
`matrix_hidden_draft_survives_inspector_park_workspace_switch_and_update`
(Rows 6 + Hotswap preset survive park + workspace switch, Update Now names
Layout/New matrix/"Create the matrix or cancel the setup", explicit Cancel
clears only this blocker), `matrix_registration_scope_retires_with_the_session`
(after session replacement the poll names exactly one blocker — the current
owner; an old-scope registration would appear twice),
`matrix_pending_create_admits_once_while_hidden_and_shown` (hide/show while
preparing/pending cannot resubmit; the first Create lands its captured
6x6 matrix once; the in-flight exemption itself reuses the crate proof
`matrix_real_create_pending_exempts_only_submitted_draft`),
`matrix_domain_cancel_and_accepted_route_close_the_task` (own Cancel and
generation-invalidating route close it; returning reopens nothing),
`legacy_script_draft_parks_across_inspector_and_workspace` (edited-but-
unapplied body parks across Inspector/workspace with the Geometry scripts
remedy live; hiding applies nothing), and
`legacy_script_apply_is_atomic_and_undoable` (enabled invalid body rejected
with diagnostics and no partial geometry; smallest valid `rect(...)` result
applies atomically; one Undo restores). RED sequence and restructures are in
`.evidence/toolbar-m1/005b-red*.log` and `005b-checkpoint.md` (notably: the
Core-gate row deadlocked in the page harness and was rebuilt on the real
preparation/pending path; a vacuous-park assertion was caught and replaced
with a hidden-while-away oracle, which produced the honest controller RED).
Regressions: layout crate browser suite 272/272 (`005b-layout-crate.log`);
workbench_shell + layout_layers + layout_field_preview +
layout_component_inspector 80/80 (`005b-regress2.log`). One existing field
assertion was migrated, not weakened: "leaving Layout retires the form" is
superseded by this ticket's park/restore row and now asserts park + ghost
gone while away, exact restoration on return, and Cancel retirement.
Limits: pending-exemption timing stays at the layout-crate tier (named
above); the script surface's packaged visual fit and the full Editor
Update-Now click path through the real header/toast chrome are not newly
proven here (the gate is driven at both opener entry points in the same
fixture family as 003/011). `cargo fmt --all --check` and
`python3 scripts/check.py lint` pass on the stable tree (`005b-lint2.log`).
No mutations, reviews or full quick gate were run (coordinator owns); this
ticket is not closed.

## Progress 2026-11-12 (proof-gap closure; no closure claim)

The two surviving M1 boundary mutants are now killed at named assertions
(evidence: `.evidence/toolbar-m1/005-gaps-checkpoint.md`,
`.evidence/mutations/005-error-gaps/`, `005-pending-gaps-try1/`,
`005-pending-gaps-try2f/`; all `restored_exact=true`).

- Failure row: new page test
  `rejected_placement_message_stays_visible_in_objects_and_retry_remains_possible`
  (`layout_layers_tests.rs`) mounts `Objects` with
  `placement_error=Some("The selected component changed before placement.")`
  (the rejection/no-mutation half remains proven at the placement tiers) and
  asserts the destructive Alert text directly after the tree mounts, the
  accepted revision unchanged, and the Add entry reopening for a retry.
  Mutant `drop-failed-placement-message` dies at that Alert assertion
  (`got: None`), not a wait.
- Pending task admission row: new layout-crate test
  `matrix_create_on_create_twice_while_pending_admits_only_the_first_submission`
  (`matrix_setup.rs` draft-safety tests) invokes the controller `on_create`
  twice for the same owner while the first Create is held pending at a Core
  gate, bypassing the rendered disabled button. It asserts both calls were
  delivered, the refusal is silent, admission still carries only the first
  (6x6) submission, and exactly one 6x6 matrix lands at revision +1. Both
  `clear-pending-create-admission` (try1) and its operative try2 variant die
  at the silent-refusal assertion: with admission cleared, the second call
  falls through to `setup_source`, which returns `None` while the first edit
  is pending (durability is not `Saved`) and surfaces "The selected board is
  no longer ready for matrix creation." Recorded honestly: that readiness
  check is a second, deeper layer that also prevents a second submission in
  this fixture; the admission guard is what keeps the refusal silent and is
  the layer this test now pins. Earlier attempts that only asserted the
  landed document or a non-ticking admission probe survived (the
  duplicate-id resolver retires a second landing, and cached preparation
  transitions land between rendered frames); the final probe re-reads
  admission on its own ticker.

`cargo fmt --all --check`, `python3 scripts/check.py lint`, the
`layout_layers_tests` page suite (19/19) and the layout browser suite
(273 passed) are green on the final tree. No production code changed; this
ticket is not closed.

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`, layout/parts browser
subsets and changed setup/placement page files. Reuse existing workflow/native safety proof;
add only new entry/host wiring. Named missing-entry and placement-error proof-gap mutations
must fail at reachability/error assertions, not compilation. Hidden-registration, old-scope,
pending-repeat and owner-close wiring mutants use the real page task suite named there.
Reuse MatrixSetup's existing `matrix_real_create_pending_exempts_only_submitted_draft` where
sufficient; no duplicate runtime pruning fixture. Do not remove setup/script guards.

## Progress 2026-11-12 (M1 usability defect: Matrix setup mobile clipping; no closure claim)

The M1 acceptance walkthrough (390x844) found the 'New matrix' setup cut off
at its panel bottom with Create/Cancel not reachable by ordinary scrolling,
and the Single/Mirrored-pair mode toggle floating over the tool row. A new
real-browser oracle section in `scripts/test-workbench-viewport.py`
(`matrix_setup_tests`, run at 320x480 and 390x844 alongside every existing
oracle) opens the surface through Project → Add object… → Matrix… and
asserts: panel rect inside the viewport with width ≤ viewport−16px,
`overflow-y: auto`, `overscroll-behavior: contain`, stale Add chooser and
Objects popout gone, no backdrop, mode toggle not overlapping the tool row,
canvas rect unchanged and not fully covered, scrolling to the end makes
Create matrix and Cancel fully visible and hit-testable, Tab from the
surface's initial focus reaches Create matrix and scrolls it into view, and
no page overflow. RED on the pre-fix build
(`.evidence/toolbar-m1/matrix-mobile-red-320.log`): overscroll-behavior was
`auto` and the mode toggle (y 80–128) overlapped the tool row (y 72–114);
the scroll/Tab reachability assertions already passed programmatically and
are retained as guards (agent-browser wheel cannot drive nested scrollers
in this harness — a plain test div also does not wheel-scroll — so native
wheel/touch scrolling remains unproved, as recorded for 002/009).

Production fix (minimal, no redesign): the stacked New matrix rule in
`web/assets/shell.css` now declares `overflow-y: auto` and
`overscroll-behavior: contain`; `MatrixSetup`, the mirrored-pair form and
the legacy GeometryScriptsEditor task surface (same bounded pattern) gained
`overscroll-contain`; the Add-matrix mode toggle moved from `top-2` to
`top-[52px]` so it sits between the tool row and the setup panel instead of
floating over the row. The intentional stacked 50% cap (canvas preview
stays visible below the form on narrow screens) is unchanged; the canvas
ghost remains visible below the panel at 320/390 (after-screenshots
`.evidence/toolbar-m1/matrix-mobile-390-after*.png`). GREEN:
`test-workbench-viewport.py` passes at 320x480, 390x844, 728x1024 and
1440x900 with all pre-existing oracles (`matrix-mobile-green-*.log`); page
suites `layout_layers_tests`, `workbench_shell`,
`layout_field_preview_tests` pass 32/0 (`matrix-mobile-page-suites.log`);
`cargo fmt --all --check` and `python3 scripts/check.py lint` pass. This
ticket is not closed.

## Known red inherited from M1 coordination (2026-11-11)

`presentation::layout_field_preview_tests` (6 tests; first failure
`new_matrix_sliders_preview_on_canvas_without_changing_accepted_state` / `new_matrix_canvas_preview_avoids_the_resized_setup_guide`)
fails 0/6 on the redesigned Layout shell: the New matrix canvas ghost never appears. Cause
unconfirmed by two diagnoses (`.evidence/toolbar-m1/field-diagnosis.log`, `field-objects-probe.log`);
opening Objects first did not help. Presumed owner: this ticket (Add chooser / `MatrixSetupMount`
route and preview publisher). `check.py quick` therefore stays red on this file until 005 restores
the real route; do not skip or weaken those tests. Trace first: setup-guide "Add Thumb cluster" and
the Add chooser into `MatrixSetupMount` and its preview publisher.
