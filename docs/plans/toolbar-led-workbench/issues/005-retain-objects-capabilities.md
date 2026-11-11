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

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`, layout/parts browser
subsets and changed setup/placement page files. Reuse existing workflow/native safety proof;
add only new entry/host wiring. Named missing-entry and placement-error proof-gap mutations
must fail at reachability/error assertions, not compilation. Hidden-registration, old-scope,
pending-repeat and owner-close wiring mutants use the real page task suite named there.
Reuse MatrixSetup's existing `matrix_real_create_pending_exempts_only_submitted_draft` where
sufficient; no duplicate runtime pruning fixture. Do not remove setup/script guards.
