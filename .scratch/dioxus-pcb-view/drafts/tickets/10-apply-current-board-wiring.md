# 10: Apply the current board-level wiring plan

**Parent:** F5.2 electrical resolver and wiring summary. Keep F5.2/F5.3, F5.1/INT.2 and all 62 parent acceptance joins open.

**Contract:** [F5.2d current-board Apply spec](../F5.2d-apply-current-board-wiring.md).

**Start gate:** The Editor-lifetime current accepted board/source and plan identity, `ElectricalPlan` Core preview, existing Core `electrical::materialize`, and production `ReplaceDocument`/strict-revision/operation-outcome path are all present. The bounded private route is source-supported without waiting for parent closure or Issue 09; paired public acceptance still requires the actual current packaged PCB route. Sol Standards/Spec review of the exact frozen spec and packet remains required before implementation.

- [ ] Match the React board-level Apply action placement; make it actionable only for the exact current Ready/Saved accepted plan and quiescent PCB scope, and disabled/unavailable for stale, pending, failed, diagnostic-error or wrong-board state.
- [ ] Revalidate the full rendered-source and board-plan identities, then call the existing Core `electrical::materialize` with the exact current plan on a clone of the accepted document. Never optimistically update Session or re-resolve a different plan in this action.
- [ ] Submit exactly one normal target-board `ReplaceDocument` commit through the production owner. Observe its exact terminal outcome; only show Saved after the exact proposal reaches the accepted Session snapshot at the next revision and is durable.
- [ ] Preserve other boards/manual nets and rely on Core-owned generated electrical net/terminal materialization. Do not duplicate the algorithm or add ApplyElectrical request/API/schema fields. Do not claim direct ApplyElectrical lock-bearing behavior until Issue 09 is independently fixed.
- [ ] On stale source/revision, invalid plan, Core materialization failure, rejected/cancelled/executor/persistence/recovery failure, retain the accepted source and truthful feedback; after success invalidate the old plan and let the current resolver request the newly accepted revision/mode.
- [ ] Verify production mounted owner/currentness/error cases, exact one revision/history edit, Undo/Redo, save/reopen, second-board and unrelated-net preservation, then the paired exact packaged browser action/result against React on the same archive and pinned build/source.
- [ ] Record RF-001/RF-006/RF-009, exact source/build/archive identity and checks not run. The paired acceptance and all parent joins remain open.
