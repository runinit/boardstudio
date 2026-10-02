# Draft F7.5: Edit supported Case mounts and gaskets in the viewer

**Parent:** F7.5 — Case viewer direct-manipulation preview edits.

**What to build:** A designer can use the existing Case viewer's supported gasket and mount handles, preview a constrained move, unlink a paired gasket support, and commit or cancel one coherent edit to the currently selected physical assembly.

**Blocked by:** F7.3 shared viewer interaction seam and completion of F7.4 mechanical configuration (all F7.4 child slices 03–06). This preserves the canonical F7.5 `start_after: [F7.3, F7.4]`; fixture-level renderer work cannot claim integrated direct-manipulation acceptance before these gates.

**Parent acceptance joins:** None beyond F7.5's own slice acceptance. Do not infer F7.8 or F7 viewer closure.

**Status:** draft for independent review; not published or counted.

- [ ] Expose only the existing Case actions: edit gasket support positions/lengths, unlink the selected paired support, and edit authored/generated mount positions where the current viewer offers them. Do not add arbitrary 3D object manipulation. Controls are available only when current exact/prepared geometry and the source readiness conditions allow the relevant handles.
- [ ] While dragging, use scoped transient preview data and existing handle/constraint logic; show valid and invalid/provisional states. Invalid or out-of-bounds samples never partially commit. Pointer release commits exactly once through the existing `SetMechanical` or `SetCase` path; successful gasket unlink is one normal persisted edit.
- [ ] Cancel/rollback on pointer-cancel, Escape, explicit cancellation, viewer unmount, physical instance/document/board Scope change, accepted token/revision change, or invalid final geometry. Release pointer capture/renderer drag resources and clear transient preview so no draft leaks into another instance.
- [ ] Admit a gesture commit only when selected-instance identity is current, Runtime lifecycle is Ready, no competing display preview/gesture or configuration/body edit is pending, accepted revision is saved, and full Scope/token still match the captured gesture. Serialize against field edits; apply the final narrow mount/support patch to the latest matching accepted configuration so concurrent whole-config edits cannot erase one another.
- [ ] Resolve the specific terminal operation outcome; clear the gesture draft only after the expected accepted revision is saved. On failure/busy/stale rejection, retain the accepted configuration, show actionable failure/retry guidance and restore original handles; a late response for an old physical instance cannot settle the current gesture.
- [ ] Honor live/manual preview policy after a valid commit: request/update preview according to the existing setting, keep previous geometry marked stale while newer work is pending/fails, and suppress stale CAD/renderer results. Do not add preview or geometry algorithms.
- [ ] Paired public browser evidence covers selected linked/unlinked gasket movement and unlink, supported mount movement, accepted versus rejected constraints, pointer capture/cancel/Escape, scope/instance switch and unmount during gesture, operation busy/failure, live/manual preview, and a successful single Undo/Redo edit. Verify actual browser native document/archive payload and save/reopen preserve the intended instance/configuration without cross-instance mutation.
- [ ] Use only the F7.3 private viewer handle/pick/lifecycle seam, current CAD/mechanical constraints, existing `SetMechanical`/`SetCase`, Runtime operation outcomes and existing preview flow. Do not widen renderer public visibility, add a public event, change CAD geometry, or create a second draft/history owner.
- [ ] **Ownership:** F7 owns the viewer gesture component/adapter files and Case consumer wiring; coordinator owns Runtime/operation admission and shared presentation integration; F7.4 children own form fields; F5 owns physical-instance setup data. Verify the precise private callback with both the F7.3 and coordinator owners before edits.
- [ ] **Profile:** Luna High author and verifier; Astra independent reviewer. Pointer capture, async operation settlement, invalid geometry and physical-instance changes are lifecycle-critical.
- [ ] **Refactoring handoff:** No new refactoring takeaway observed in this source pass. Preserve RF-006 (physical-instance/canonical context) and RF-001 (private viewer host/service boundary); add findings only if implementation demonstrates a distinct issue.

