# Outline lifecycle bounded repair — 2026-10-02

Original source: 88025712633b7a0d9de64b6fd57c558811ee7ce0. Current branch also contains independent bridge projection commit121de7c99442a24466cb785dd386baa96932b0ef, which this repair does not modify or clear.

Six regression failures were reproduced against the original production controller, mounted in a real Dioxus VirtualDom: retained Copy and Delete intents after callback refresh; Completed with absent accepted snapshot; Completed after Session epoch change before replacement readiness; Completed with current recovery failure; exact observed slot lost after Editor unmount. Production projection action construction was extracted before the red run without changing admission/settlement behavior. The fake Runtime implements only observable submit/model/outcome ports; the actual controller, Inspector, OutcomeRegistry and Dioxus effects run. Async scheduling uses a deterministic detached executor/timer facade. This does not claim browser unmount coverage.

Repair captures render generation in actual Copy/Delete payloads, retires terminal absent/stale/failed owners before current revision/readiness waits, and retains the exact observed slot in a detached task until terminal without accessing component signals after await. Hidden same-owner settlement, rejection/retry and unrelated-context rejection remain covered. No public/member visibility widening or suppression was added.

Verification: expanded original source 6 failed/4 passed for expected assertions; final mounted harness11/11 passed; strict harness Clippy and strict WASM page all-target Clippy passed. Full native attempt is recorded but initially blocked by the separate author's newly auto-discovered web/tests/presentation.rs facade (E0433); author notified to relocate it. Logs accompany this report.

Bounded repair requires independent source reack. The complete ticket is not cleared: SelectOutline activation still uses the direct root tree submit path without this exact observed Ready/Saved owner. That is being implemented as the next private composition change in this same isolated tree. Bridge projection change requires separate source reack. Exact packaged paired browser/history/reopen and parent joins stay open; existing successful880 browser evidence remains historical source provenance only.

RF carry-forward proposal: existing RF-006 owner/currentness and RF-009 evidence provenance cover these failures; no new category or shared-ledger edit.
