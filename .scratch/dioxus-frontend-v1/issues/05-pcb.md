# F5: PCB and hardware frontend

**Status:** planned
**Blocked by:** F3, F4
**Category:** frontend behavior-preserving port; F1 intentionally replaces the M1 demo layout with the reference shell and authorized temporary placeholders.
**Reference:** React 5a472a94; Rust base f0ac0a19.
**Authority:** [parent spec](../spec.md), [roadmap](../../../docs/migration/DIOXUS-FRONTEND-V1.md).

## Work

PCB composition/layers, module overlays/inspectors/profiles, wiring/controllers/connectors/pins/jumpers, readiness/reference/findings UI.

## Exit evidence

Matched multi-board/physical module workflows, controls and public service calls; state/selection/readiness parity.

- [ ] Port all scoped TSX/UI-hook behavior through existing service contracts.
- [ ] Match reference visuals, themes, responsive and interaction states.
- [ ] Retain affected checks/browser evidence and resolve independent reviews.
- [ ] Update inventory, integrated source, demo and remaining limits.

A visible placeholder does not complete F2–F9. Preserve original projects and
reference source. Backend implementation/format/API changes require separate
scope; never silently remove a frontend behavior when an adapter is missing.

## Reference ownership

The [file ledger](../evidence/tsx-inventory.json) pins exact reference hashes. These
are source responsibilities, not permission to mark whole shared files complete.

- `app/src/ui/BoardReferencePanel.tsx`
- `app/src/ui/HardwareInstancesPanel.tsx`
- `app/src/ui/HardwareReadiness.tsx`
- `app/src/ui/ModulePcbOverlay.tsx`
- `app/src/ui/WiringPanel.tsx`
- `app/src/ui/usePcbWorkspace.tsx`

Existing regression coverage to transfer or retain:

- `app/src/ui/ModulePcbOverlay.test.tsx`
