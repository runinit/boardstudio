# 015: Case Preview presentation with one generation/command owner

Status: ready-for-human
Type: build
Blocked by: [013](013-case-workspace-adapter.md), [009](009-shared-toolbar-panel-host.md)
Parent: [map.md](../map.md)
Owner: coordinator for approval/routing; Case generation owner and render-only control views

## Boundary

M2 only, not a dependency of the first Layout delivery. Split from 013 so its tree/Inspector
migration cannot accidentally authorize a new generation interface. Before implementation,
approve a concrete contract for exposing the current CasePanel controls to its Preview
entry. `WorkspaceToolbarInput::Case` is currently a unit variant; CasePanel computes its
readiness, callbacks, automatic-generation effect and command registration inside the canvas.
Simply copying buttons or mounting CasePanel twice is not an acceptable extraction.

Proposed destination: a Case Preview entry in the single strip opens bounded detail with
existing Live preview / Update preview / Cancel / gated Export geometry controls. Keep 44px
touch targets there, not a silently reduced 32px export action. No second permanent action
row, backdrop, export policy or duplicate generation owner. This proposal, its cross-crate
interface and any alternative footer exception require the user's M2 decision.

The implementation must have exactly one automatic-generation observer and one registration
of each Case command, regardless of how many render-only views consume that owner. Accepted
revision/physical-instance/readiness checks remain at the existing action admission point;
closing any view cannot start/cancel generation or make a previous result exportable.

## Acceptance criteria

- [ ] Human approves the concrete interface and placement; Gortex verifies any signature
      change before mutation and visibility is not widened by inference.
- [ ] Both old and new presentation can be checked against one existing owner during
      extraction, without mounting duplicate effects or command registration.
- [ ] Current-result, Cancel, preview/error and exact-export semantics remain unchanged.

## Proof map

- Single observer — a valid Case auto-preview fixture with both render consumers mounted →
  enter Case and change a real generation input once → one generation request and one
  command registration; opening/closing views issues neither · mounted · planned
- Generation controls — ready/busy/current-exact fixtures → invoke Live preview, Update,
  Cancel and Export through new detail → same callbacks/enabled reasons and accepted
  generation result as the existing control owner · mounted · planned
- Stale exactness — old result visible before revision/instance change → attempt export or
  deliver captured old action → no output; current matching exact result still exports
  through the existing admission policy · mounted · planned

## Checks and mutations

Reuse CasePanel fixtures. Run affected quick/lint, `check.py browser --only case`, and
`run-wasm-tests.py --files web/src/presentation/case_workspace.rs` for page wiring.
Proof-gap mutants: mount a second automatic observer (runner: Case browser subset; oracle:
second generation/duplicate command count), and admit a foreign/previous result for export
(same runner; oracle: no delivery). Exact specs are created against implemented source and
run through `scripts/mutation-check.py`. Coordinator independently reruns the stale reject.
One combined Spec/Standards review and the ticket gate; no M2 work before phase approval.
