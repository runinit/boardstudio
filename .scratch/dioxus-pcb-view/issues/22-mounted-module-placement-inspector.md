# 22: Edit an existing mounted module placement from PCB

**Parent:** F5.5. This child uses the accepted module-definition, resolved-scene, physical-instance and typed Core edit seams already in the canonical F5 parents.

**Reference:** React `app/src/ui/ModuleInspector.tsx` at `5a472a9426e6e38993361da402cd4ec730feb369`. The paired mounted-module fixture is `vik-module-review.boardstudio`, SHA-256 `952b26565a4df3ae1122fd0716017560279c938ee19fb6c15af966c88c1d35b0`.

**What this bounded slice supplies:** select a real mounted module from its PCB scene with pointer or Enter/Space, route that selection through the current scoped tree-context owner, and show the corresponding module source entry in Parts, matching the pinned React route. The source Inspector offers a guarded return to the same exact accepted project placement's PCB Inspector. Edit its host board, physical instance, host/facing face, X/Y/yaw, gap, attachment and detached state. Save through one ordinary `SetMountedModule` Core edit, preserving all untouched module fields; remove through `RemoveMountedModule`. Project/session/board/module identity and accepted token/revision guard each action. Module source artwork remains read-only and owned by its module definition.

**Start:** Existing `ProjectDoc.modules`, `module_definitions`, `hardware.instances`, resolved `SceneDelta.module_scenes`, `Runtime::scope`, selected scoped tree context and Core `SetMountedModule`/`RemoveMountedModule` are the consumed capabilities. F4's whole module-definition/profile/readiness editor is not a start gate.

**Acceptance for this child:**

- [ ] The settled VIK fixture exposes each mounted module as a named, keyboard-focusable SVG selection target; pointer and Enter/Space open that exact definition's Parts source Inspector without fabricating host `Part` IDs.
- [ ] When the source entry came from a selected PCB placement, its “Edit selected mounted placement” action returns to the same guarded placement Inspector; selecting another source variant hides/rejects the stale placement route.
- [ ] Inspector identifies the selected source and placement, and offers only current host boards and that board's physical instances.
- [ ] Face, pose, gap, attachment, detached state and physical-instance changes remain a draft until Save.
- [ ] Save submits one revision-guarded `SetMountedModule` edit for the selected existing instance and preserves connection, service-clearance and support fields unchanged.
- [ ] Remove submits one revision-guarded `RemoveMountedModule` edit for that instance.
- [ ] Changing project/session/board/module selection or accepted token/revision rejects a stale Save/Remove. Normal accepted-document save, Undo/Redo and reopen are proved in the changed paired journey.
- [ ] Changed root/subpath candidate journey, exact archive identity, actual resulting module pose/instance/document comparison and one consolidated candidate check are retained.

**Explicitly open:** adding or editing module definitions; source profiles/readiness/library assets; VIK connection or connector creation; embedded circuits; daughterboard component editing; cross-board/split physical-instance navigation; F5.4 all-geometry/selection criteria and full F5.5/F4.7/F5 parent acceptance. Service-clearance and support-ring/standoff authoring are refined in [Issue 23](23-mounted-module-supports-and-clearance.md). The paired geometry and current layer work remain evidence inputs, not completion of those gates.

**Retained placement evidence:** [Save/reopen archive comparison](../evidence/22-mounted-module-inspector-20261003/placement-save-reopen.md) and [settled Undo/Redo projection](../evidence/22-mounted-module-inspector-20261003/undo-redo-settled-34765.md) qualify this bounded placement path on candidate 34765. They do not cover Issue 23's new fields or close the F5.5 parent.

The bounded route result is recorded in [the 34769 paired receipt](../evidence/24-module-source-route-20261003/receipt.md): pointer, Enter and Space each opened the selected splitter-above source entry, and its return action reopened that exact placement. Before/after project exports were byte-identical at revision 6. This qualifies the selected instance on this fixture only; variant-change stale-route rejection, every mounted target, and the changed edit/history journey remain open.

**Architecture:** this adds `TreeContext::MountedModule { board_id, module_id }` to the existing scoped selection owner. It never overloads a source footprint ID as a host part. Placement writes stay in Core/Session accepted-document history; transient form values remain view-local. This extends RF-001's shared composition evidence and RF-006's need to keep project, board and physical-instance scopes distinct; it does not add a data store or public API.
