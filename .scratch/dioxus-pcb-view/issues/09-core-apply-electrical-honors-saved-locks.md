# 09: Cover saved board locks through Core ApplyElectrical

**Owner:** Core electrical workflow.

**Parent:** Existing Core `ResolveElectrical` / `ApplyElectrical` behavior used by F5.2/F8.2. This is a bounded coverage child, not a confirmed defect, new frontend parent, API change, or schema expansion.

**Source finding:** `CoreEngine::handle(CoreRequest::ApplyElectrical)` checks the current base revision then reconstructs the review request with `locks: Default::default()`. `electrical::resolve` begins from the current selected board's persisted `ElectricalBoardConfiguration.locks` and extends those locks with request locks, so the empty map does not drop saved board locks. The current source therefore supports lock-bearing Apply by construction. Existing `resolver_scopes_board_and_honors_controller_and_locks` exercises request locks, but there is no direct `ApplyElectrical` request test for persisted config locks. The review audit is [`core Apply lock source audit`](../../evidence/apply-current-plan-20261002/core-apply-lock-audit.md).

**Start gate:** `CoreRequest::ResolveElectrical`/`ApplyElectrical`, authoritative selected-board config lookup, Core fingerprint check and the Core electrical integration fixture are present. After independent review of this exact contract, a Core author may add the narrow request-level coverage; no full F5.2/F8.2 or frontend parent completion is required.

- [ ] Add a Core integration test with a lock stored in the target board's `ElectricalBoardConfiguration.locks`; resolve using that accepted document, then send `ApplyElectrical` with the exact plan and matching `base_revision`. Require a normal committed Scene at one next revision, successful materialization and preserved lock. Current source is expected green; do not label this as a regression red without observing one.
- [ ] Add a same-key lock on another board and prove only the target board's configuration affects its plan; locks never leak across boards.
- [ ] Preserve coverage for lock-free Apply, stale base-revision rejection, changed-input fingerprint rejection, non-draft blocking diagnostics, unrelated/manual-net preservation and the normal single undo history step.
- [ ] Keep Core's existing authoritative selected-board lookup, resolver lock merge, fingerprint equality and materialization. Add no request field, generated contract, public visibility, persisted field or UI inference.
- [ ] Run affected Core native tests, formatting and relevant strict checks; preserve exact logs/hashes. If behavior fails, retain the observed red and return for defect classification before changing the contract.
- [ ] Keep F5.2/F5.3/F8.2 and all parent acceptance joins open. Record exact commit/evidence and RF-009 handoff; no new RF item is proposed.

**Execution status (2026-10-02):** Planning cleared by Sol Standards/Spec review of `73b687ba`; implementation underway in the isolated PCB stream. Core characterization coverage is independent of the UI start gate. Public acceptance remains open.
