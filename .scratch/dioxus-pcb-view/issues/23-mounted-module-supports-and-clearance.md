# 23: Edit mounted-module service clearance and supports

**Parent:** F5.5. This child continues the [mounted-module placement Inspector](22-mounted-module-placement-inspector.md) through its existing scoped selection, local draft, accepted Session snapshot and Core edit path.

**Reference:** React `app/src/ui/ModuleInspector.tsx` at `5a472a9426e6e38993361da402cd4ec730feb369`, specifically its extra service-clearance input, case support rings / board standoffs section and accepted resolved-support preview. The matched VIK fixture and its source pin are recorded in Issue 22.

**Bounded behavior:** Edit `MountedModule.service_clearance` and author/remove `ModuleSupport` rows in the selected instance's existing local draft. Source hole choices come from that accepted module definition's mounting holes. Each support records source hole, outer/hole diameters and module-midplane Z/height. Render the accepted `ResolvedModule.mount_supports` as read-only geometry detail. Save the combined draft through Issue 22's same guarded `SetMountedModule` operation so Core remains the only support-validation and geometry-resolution authority. Keep the existing Board/Case attachment choice and use it to label PCB standoffs versus case rings.

**Start:** Issue 22's scoped selected-module owner and accepted `SceneDelta.module_scenes`, `MountedModule.service_clearance`, `MountedModule.mount_supports`, `ModuleDefinition.mounts`, `ResolvedModule.mount_supports` and Core `SetMountedModule` validation/resolution are present. No additional public API or durable UI state is needed.

**Acceptance:**

- [ ] Extra service clearance is an editable nonnegative millimetre value in the same module draft.
- [ ] The support editor lists source mounting holes from the selected module definition and captures outer diameter, hole diameter, Z from module midplane and height.
- [ ] Add requires finite values, outer diameter greater than hole diameter, hole diameter at least the source drill, positive height and one support per source hole. Remove affects only the unsaved draft.
- [ ] The section labels the configured supports as case rings or PCB standoffs according to the selected attachment. The accepted resolved geometry is displayed read-only when available.
- [ ] Save submits exactly one ordinary guarded `SetMountedModule` edit. Existing connection and all unrelated module data survive; invalid Core physical-contact/attachment combinations remain Core errors rather than being reported as validated by the form.
- [ ] Changed paired journey demonstrates one clearance edit, one support add/remove or edit, save, Undo/Redo and reopen against the same VIK fixture. Preserve archive identity and parent criteria in the root candidate receipt.

**Explicitly open:** manufacturer-specific support dimensions, source readiness, full case/PCB contact qualification, VIK connection/circuit work, component editing, module-definition/profile editing, remaining F5.4/F5.5 behavior and all canonical parent joins. Designer-selected values and a displayed resolved solid do not establish vendor specifications or manufacturing readiness.

**Architecture/RF:** No new store, Core geometry implementation, or public contract. View-local textual entry state composes into the accepted module draft, then reuses Core validation and Session history. This adds evidence under RF-001 shared Editor composition, RF-006 project/module/physical-instance scope, and RF-009 paired feature accounting.
