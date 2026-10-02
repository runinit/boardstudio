# F7: Case and 3D frontend

**Status:** planned
**Blocked by:** F3, F4; F5 hardware-dependent views
**Category:** frontend behavior-preserving port of the pinned React reference.
**Reference:** React 5a472a94; Rust base f0ac0a19.
**Authority:** [parent spec](../spec.md), [roadmap](../../../docs/migration/DIOXUS-FRONTEND-V1.md).

## Work

Case inspector/settings/choices/generation, construction/mechanical assembly/hardware panels, readiness/findings, camera/picking/material/layers.

## Exit evidence

Matched settings/live/manual/cancel/retry/error paths, scoped canvas ownership and public service results, themes and disposal.

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

- `app/src/ui/AssemblyScene.tsx`
- `app/src/ui/AssemblyViewer.tsx`
- `app/src/ui/CaseChoice.tsx`
- `app/src/ui/CaseGenerationControls.tsx`
- `app/src/ui/CaseInspectorPanel.tsx`
- `app/src/ui/MechanicalAssemblyPanel.tsx`
- `app/src/ui/ModelPreviewBoundary.tsx`
- `app/src/ui/useCaseWorkspace.tsx`

Existing regression coverage to transfer or retain:

- `app/src/ui/AssemblyScene.mounts.test.tsx`
- `app/src/ui/AssemblyScene.test.tsx`
- `app/src/ui/MechanicalAssemblyPanel.test.tsx`
- `app/src/ui/MechanicalAssemblyPending.test.tsx`
- `app/src/ui/ModelPreviewBoundary.test.tsx`
- `app/src/useCaseGeneration.live.test.tsx`
