# 013: Case tree, Inspector and View with physical-instance ownership

Status: awaiting-decision
Type: build
Blocked by: [009](009-shared-toolbar-panel-host.md)
Parent: [map.md](../map.md)
Related: [015](015-case-preview-owner-contract.md)
Owner: Case workspace tree/Inspector and existing shared-viewer transient owner

## Boundary

M2 only; phase approval and the concrete View/editor interface must precede implementation.
Compact Objects retains assembly/physical-instance roots, generated stack/gaskets, legacy
bodies, PCB/components, disclosure/visibility and previous-geometry/fit labels. Keep Case's
TreeAction, CaseSelection and DisplayRequest, not a Layout TreeContext adapter. Inspector
retains physical assembly/transport/PCB-design/flip, MechanicalSettings, display color/reset/
visibility and CaseBodyInspector, with current linked/shared/local rules.

Inventory the existing shared viewer rail: Assembled / Exploded / Section, Shaded /
Wireframe / Hybrid and Hidden lines. Proposed destination is Case's View section/detail in
the single strip, using the same viewer transient owner and no second permanent wrapping
rail. Keep camera controls footer. Do not create another viewer state or observer merely to
render controls elsewhere; obtain approval before widening a cross-crate interface.

Generation controls and their observer/commands/export contract are **015**, not an implied
render-only change in this ticket. Do not move or duplicate CasePanel, its generation effect,
command registration or exact-result authority while migrating tree/Inspector/View.

Draft-owner preflight for M2: classify MechanicalSettings and CaseBodyInspector's actual
field commit triggers and unresolved forms. No DraftOwnerRegistration currently exists in
the Case crate; do not presume those forms are explicit-submit. Prefer hoisting only typed
raw draft/error identity above selection/visibility switches, without keeping the complete
CasePanel generation lifecycle active in unrelated workspaces. The concrete mechanism and
update-safety treatment for retained unresolved work must be recorded/approved before this
ticket becomes ready. Use the existing registry if required; off-screen blockers
identify Case/editor/remedy. No second draft registry or silent reload loss.

## Acceptance criteria

- [ ] Every Case inventory row owned by 013 has a named destination; 015's separate gate is
      explicit and does not permit omitting those controls from the final six-workspace work.
- [ ] Tree/display/view operations retain their actual scoped identity and view-only meaning.
- [ ] Typed settings/body draft/error survives owner/panel/workspace return with no foreign
      display/edit or synthetic commit; actual blur triggers retain exactly-once settlement.
- [ ] View controls do not create a second row, remount the viewer or alter generation policy.

## Proof map

- Case tree/display — two physical assemblies, generated/gasket or legacy body fixture →
  disclose/select/toggle visibility via Objects/Inspector → same scoped assembly/body/layer
  and display result; ordinary pick never auto-opens Inspector · mounted · planned
- Construction lifetime — actual unsubmitted settings/body draft A before its commit trigger →
  change instance/layer/panel/workspace and return → A text/error retained, no B edit;
  explicit supported linked construction edit retains existing one-Undo behaviour · mounted · planned
- View rail parity — known transient view with model loaded → operate Assembled/Exploded/
  Section, display mode and Hidden lines from View/detail → same viewer state and rendered
  result, no accepted revision, second permanent rail or view-induced generation · mounted · planned
- Update-safety classification — retained unresolved form established by the M2 audit →
  hide/switch then Update Now → approved protection names Case/editor/remedy; settled/pending
  values are not falsely treated as unresolved drafts · mounted · planned

## Checks and mutations

Use affected quick/lint, `check.py browser --only case`, and
`run-wasm-tests.py --files web/src/presentation/case_workspace.rs` for page integration.
Reuse existing display/tree/edit fixtures. View proof-gap mutant: route Section to Exploded
(same Case browser subset, oracle: expected Section transient/rendered state); page wiring
uses its page suite. 015 owns duplicate-observer and stale-exact-export mutations. A new
interface/commit policy or M2 phase requires human approval, not a shell implementation guess.
