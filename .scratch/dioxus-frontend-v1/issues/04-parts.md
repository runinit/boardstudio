# F4: Parts and assembly frontend

**Status:** planned
**Blocked by:** F2; integrate with F3
**Category:** frontend behavior-preserving port of the pinned React reference.
**Reference:** React 5a472a94; Rust base f0ac0a19.
**Authority:** [parent spec](../spec.md), [roadmap](../../../docs/migration/DIOXUS-FRONTEND-V1.md).

## Work

Library/search/import/create/edit; parameter forms, footprint/model previews and errors; assembly/module UI using existing services.

## Exit evidence

All supported library actions and states; asset/ID preservation, same service inputs/results, keyboard/visual parity.

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

- `app/src/ui/AssemblyEditor.tsx`
- `app/src/ui/Ergogen2DPreview.tsx`
- `app/src/ui/GeneratorFields.tsx`
- `app/src/ui/InputProfileEditor.tsx`
- `app/src/ui/LibraryWorkspace.tsx`
- `app/src/ui/ModuleInspector.tsx`
- `app/src/ui/ModulePreview.tsx`
- `app/src/ui/ModuleProfileEditor.tsx`
- `app/src/ui/PartMechanicalProfileEditor.tsx`
- `app/src/ui/PartsInspectorPanel.tsx`
- `app/src/ui/PartsLibrary.tsx`

Existing regression coverage to transfer or retain:

- `app/src/ui/Ergogen2DPreview.test.tsx`
- `app/src/ui/libraryDisplay.test.tsx`
- `app/src/ui/partsCatalog.test.tsx`
