# F3: Complete Layout frontend

**Status:** planned
**Blocked by:** F2
**Category:** frontend behavior-preserving port; F1 intentionally replaces the M1 demo layout with the reference shell and authorized temporary placeholders.
**Reference:** React 5a472a94; Rust base f0ac0a19.
**Authority:** [parent spec](../spec.md), [roadmap](../../../docs/migration/DIOXUS-FRONTEND-V1.md).

## Work

Tree/canvas/inspectors, matrices/components, transforms/constraints/relations, snapping, outlines/cutouts/refinements/scripts, layers/findings and 2D/3D.

## Exit evidence

Reference UI action matrix through existing public APIs, matching edit/history/geometry, previews and save/reload.

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

- `app/src/ui/CanvasLayers.tsx`
- `app/src/ui/CanvasObjects.tsx`
- `app/src/ui/CommandMenu.tsx`
- `app/src/ui/ConstraintEditor.tsx`
- `app/src/ui/ExistingHalfSetup.tsx`
- `app/src/ui/FindingList.tsx`
- `app/src/ui/InspectorControls.tsx`
- `app/src/ui/InspectorSection.tsx`
- `app/src/ui/MatrixInspector.tsx`
- `app/src/ui/MatrixInspectorPanel.tsx`
- `app/src/ui/MatrixSetupPreview.tsx`
- `app/src/ui/MirroredPairSetup.tsx`
- `app/src/ui/OutlineControlOverlay.tsx`
- `app/src/ui/OutlineDraftPreview.tsx`
- `app/src/ui/OutlineFeatureEditor.tsx`
- `app/src/ui/OutlineInspector.tsx`
- `app/src/ui/OutlineSnapGuides.tsx`
- `app/src/ui/OutlineTools.tsx`
- `app/src/ui/WorkbenchLayers.tsx`
- `app/src/ui/WorkbenchTree.tsx`
- `app/src/ui/useOutlineEditor.tsx`
- `app/src/ui/useScriptEditor.tsx`

Existing regression coverage to transfer or retain:

- `app/src/ui/OutlineControlOverlay.test.tsx`
- `app/src/ui/Workbench.performance.test.tsx`
- `app/src/ui/WorkbenchLayers.test.tsx`
- `app/src/ui/useWorkbenchNavigation.test.tsx`
