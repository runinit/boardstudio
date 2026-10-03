# 06: Own physical-setup intents through Editor acceptance and selection

**Parent:** F5.6, “Physical-board instance setup and Case handoff.” This ticket satisfies the Editor-lifetime admission/settlement/selection-owner gate recorded in the existing F5.6a child.

**What to build:** The physical-setup controls emit typed intents to an unconditional Editor-lifetime owner. The owner validates the current accepted document, session/board/instance scope and live selection; observes the exact edit outcome before submission; retains feedback if the panel hides; and updates board/instance selection only after that exact proposed document is accepted.

**Blocked by:** 05: accepted-physical-setup-proposals. Start also requires the coordinator to assign the private Editor-owned mount/typed-intent slot and accepted source/scope callback within the existing INT.1 shell. INT.1 is the existing parent start edge; it does not itself supply the feature-specific root owner or callback. No F5.1 canvas, F5.2 wiring, or INT.2 dependency.

**Status:** ready-for-agent

**Contract:** [F5.6 physical-setup prerequisite spec](../drafts/F5.6-physical-setup-prerequisites-spec.md). This ticket does not change the existing F5.6a start edge or acceptance joins. The existing paired public browser/history/reopen acceptance remains required.

- [ ] The Dioxus setup controls retain the TypeScript contextual placement, labels, checked/pressed states, reversible copy, and keyboard behavior, and emit typed topology/transport/reversible intents rather than whole captured hardware/document snapshots.
- [ ] One Editor-lifetime owner captures and admits accepted document/session/board/instance Scope, token, revision, generation, and live selected-instance identity; it remains mounted while setup controls are hidden or the workspace changes.
- [ ] The proposal is built once from the current immutable accepted snapshot through ticket 05. After any async normalization, recheck the full owner identity and exact source snapshot before submission.
- [ ] The root registers/observes the exact `OperationId`/`OutcomeSlot` before submitting the existing normal Edit event, then settles that operation’s terminal outcome afterward. It distinguishes submission rejection, terminal failure/stale results, accepted-document advancement, and durable persistence.
- [ ] Failed, stale, superseded, busy, identical retry, hidden-leaf, selection-removal, and pending-durability cases retain or settle feedback by exact request identity. They never navigate to a proposed ID or orphan an accepted edit while save is pending.
- [ ] The primary board/instance preference is reconciled only after the matching proposal is accepted and its captured scope remains current; no second document or selection authority is introduced.
- [ ] Supply owner/operation traces, accepted proposal assertions, and controls needed for the one existing F5.6a paired browser journey, including its Case-context, Undo/Redo, and save/reopen evidence. Do not duplicate the paired suite; F5.6a remains the sole user-visible acceptance join and stays open until its full criteria pass.
- [ ] Record root integration ownership, exact operation traces, evidence references, checks not run, and RF-001/RF-006/RF-009 handoff; F5.6a records the shared browser/build/fixture identity and paired screenshots once.

## F5.6-C03 Case instance controls extension (2026-10-03)

Add private Case PCB-design and flip intents to the existing Editor owner. Build the proposal from its accepted snapshot, change only the selected instance (including its mechanical board association), and use the ordinary edit/history path. Reconcile navigation to the same instance on its new board only after exact proposal acceptance. Retain scope/operation guards and all original acceptance; paired reassignment/flip, Undo/Redo/reopen and unrelated-instance preservation remain the finish condition.
