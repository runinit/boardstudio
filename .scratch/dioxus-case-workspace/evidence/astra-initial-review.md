# Independent planning review

**Decision: publish after the following bounded corrections. Planning only; no implementation or acceptance approval.** Read canonical tasks/workflow JSON, accepted T1-01, authority, pinned retained React controls/navigation, and current Rust renderer/Case/export seams. No repository edits or builds.

The canonical graph contains 62 parents. F7.1 has no start/acceptance blockers; F7.2 and F8.1 start only after accepted INT.1 and have no parent acceptance joins. The drafts preserve downstream responsibilities except this error:

1. **Draft 01:** its final paragraph incorrectly calls INT.2/BND.1 F7.3 “start blockers.” F7.3 starts after **F7.1**; INT.2 and BND.1 are **acceptance joins**. Correct this without delaying fixture-backed implementation. The shared-viewer planning scope otherwise matches F7.1, including all five consumers and crate reachability.

2. **Draft 02:** explicitly preserve `useCaseWorkspace.tsx`'s generated-mode branch: a selected-board mechanical configuration shows the generated assembly panel/note and keeps authored bodies saved but hides their editor until the stack is disabled. A configuration belonging to another board shows the mismatch explanation/“Show configured board” affordance alongside that board's authored bodies. Assign configuration/disable behavior to F7.4 while preserving this presentation join; do not expose simultaneous authored editing absent from React. The body/mount/gasket fields and defaults otherwise match `CaseInspectorPanel.tsx` and `useCaseWorkspace.tsx`.

3. **Draft 03:** replace “leaving Export through workspace navigation restores previous workspace” with exact behavior: Escape, active Export toggle and WorkflowReturn restore `lastDesignMode`; choosing another workspace navigates there. `lastDesignMode` records Design/PCB/Keymap/Keycaps/Case, not Parts/Export. Review wiring clears semantic/part selection and reveals Inspector; Review case reveals Inspector. Explicitly include the conditional **Embed used models** checkbox and its existing preference callback, not just an unspecified portable option. Rows/readiness labels match the source.

Existing seams are sufficient for planning: public RendererHost mount/scene/camera/dispose plus existing WASM state/handles/pick operations; binary-local adapters for missing mappings; accepted snapshot + existing SetCase/Edit/history for authored bodies; private Export row inputs/callbacks using existing board readiness, wiring/CAD state and Runtime delivery owners. Generated readiness needs current generation/assembly inputs, not a document-only guess. No new engine, format, public API or visibility expansion is justified. Keep all F7/F8 downstream acceptance joins unchanged.

RF: no new takeaway; RF-002/RF-012 already cover renderer mapping and crate boundaries.
