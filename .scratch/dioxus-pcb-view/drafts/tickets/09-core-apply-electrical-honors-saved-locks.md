# 09: Revalidate Core ApplyElectrical with the selected board's saved locks

**Owner:** Core electrical workflow.

**Parent:** Existing Core `ResolveElectrical` / `ApplyElectrical` behavior used by F5.2/F8.2. This is a bounded defect correction, not a new frontend parent or a schema/API expansion.

**Observed defect:** `CoreEngine::handle(CoreRequest::ApplyElectrical)` checks the current base revision, then reconstructs a `ElectricalPlanRequest` from the incoming plan with `locks: Default::default()`. The React/Dioxus resolver request supplies the selected board's persisted `ElectricalBoardConfiguration.locks`. Resolver fingerprints include the resulting assignments, including their locked state. A valid lock-bearing plan therefore differs from Core's lock-free re-resolution and can be rejected as changed inputs even when the document, revision, board, mode, controller, and lock configuration are unchanged.

**What to fix:** When applying a current plan, obtain the lock map from `self.document.hardware.boards` for the exact `plan.board_id` after the existing base-revision check, and use that map in the authoritative Core re-resolution. Preserve the existing request shape, selected-board scoping, plan/document revision checks, exact fingerprint equality, `draft: false` diagnostic/pad validation, generated-net materialization, normal Core edit/history behavior, and all other boards. No new request field, generated contract, public visibility, document field, or lock inference from UI is permitted.

**Start gate:** The existing production `ResolveElectrical` and `ApplyElectrical` branches plus `electrical::resolve` / `electrical::materialize` and Core integration-test fixture are present. A Core author can implement after this issue and the exact test contract receive independent Spec/Standards clearance; no full F5.2/F8.2 or frontend parent completion is required.

- [ ] Add a regression in `core/tests/electrical_wiring.rs` that resolves a document with a selected-board row/column or signal lock and applies that exact plan with matching `base_revision`; reproduce current Error/fingerprint mismatch red, then require a normal Scene/committed document at exactly one next revision containing the plan and retaining the saved lock.
- [ ] Prove a lock on another board with the same lock key does not leak to the target board; exercise the selected board's actual stored map only.
- [ ] Preserve and test lock-free apply behavior, stale base-revision rejection, changed-input fingerprint rejection, non-draft blocking diagnostics, one normal undo entry and unrelated/manual-net preservation.
- [ ] Use Core's persisted selected-board configuration as the only lock source. Keep plan-selected board exact; do not substitute physical-instance scope or another board's locks.
- [ ] Run affected Core native tests, formatting and relevant strict checks; preserve exact regression red/green logs and source hashes. Do not use browser or source-presence evidence as a substitute for the Core request regression.
- [ ] Keep F5.2/F5.3/F8.2 and all parent acceptance joins open. Record exact commit/evidence and the RF-009 handoff; no new RF item is proposed.
