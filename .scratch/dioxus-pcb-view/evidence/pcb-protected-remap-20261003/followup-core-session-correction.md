# Issue 12 Session/Core correction

This follow-up supersedes the direct `ReplaceDocument` behavior proposed by source commit `2fb5a63d691225d93f2bfc2f21cd280cb51bcbca` and recorded in [the original receipt](results.md), SHA-256 `3d55d4cc0d62329ce0e775ddc9825d5de88e73182cb1123370682b5ad1f503e6`. Both earlier commits and their evidence remain in Git history. The corrected source is `78a2b0985018b5a3d271f6be55631796bb9486bc`, following implementation commit `2819c941764007e517f8c9875bcaaaa0396e502c`.

## Confirmed operation boundary

Core's existing `ReviewElectricalRemap` request checks the accepted base revision and exact protected-handoff fingerprint, clears only the requested board's baseline, increments the document revision, and returns a committed scene. The ordinary `CoreEngine::edit` path calls `electrical::preserve_handoff` before saving. A `ReplaceDocument` that removes a downloaded baseline therefore returns a document with the old baseline restored; ordinary edit is not an equivalent implementation of the explicit review action.

The correction adds a strict-revision `Session::Event::ReviewElectricalRemap` carrying operation ID, accepted base revision, board ID, and expected fingerprint. Session sends the existing `CoreRequest::ReviewElectricalRemap` and handles its `CoreReply::Scene` through its normal persistence and accepted-snapshot flow. No Core request wire shape, generated contract, persistence schema, or second document/history authority changed. The Session event is not serialized or exported to TypeScript. Core's review branch does not add an Undo snapshot; the correction preserves that established behavior rather than claiming the explicit review is itself undoable.

## Executed evidence

The actual Application integration test `protected_handoff_review_uses_core_operation_after_normal_edits_preserve_it` drives `Session` and `CoreEngine` together. It first submits a real ordinary replacement with protection removed and observes Core restore the baseline. It then submits the dedicated event, verifies the emitted request variant, saves the returned revision 2 document, and confirms only board A's protection was cleared while its locks, assignments, and key bindings and board B's full configuration remain unchanged. `protected_handoff_review_rejects_a_stale_fingerprint_without_saving` verifies that a stale Session revision is rejected before Core dispatch and that a stale fingerprint is rejected by Core without a persistence effect or accepted-document change.

The two focused actual Session/Core tests passed (2/2); immutable log: [application-session-core-board-preservation.log](application-session-core-board-preservation.log), SHA-256 `83e95bf90758dea914ca0393ef1b75f500381764b71c07039f61b9dd64dc9b6b`.

The three mounted owner tests passed (3/3): [mounted-owner-tests-final.log](mounted-owner-tests-final.log), SHA-256 `24f6a7f6a3a29c1dfe485416cb35e7d346f86fe5fd8fb50277a94eb310e74e1a`. These use the production owner hook in a native VirtualDom harness with a Runtime stub and manually supplied outcomes; they do not replace the Session/Core integration test and do not claim a browser journey.

Formatting checks for the application and web crates, `git diff --check`, and JSON parsing of the RF register passed. The existing `CoreRequest` and generated Core wire contract were reused unchanged. A fresh WASM/root package build has not been run from this corrected source. Paired packaged browser behavior, history/reopen, compact/focus, final root join, and all 62 parent acceptance joins remain open. The separate actual-package visibility/layout and Apply behavior tickets are not addressed here.

## Prior packet status

The original source packet's direct-clear proposal is superseded because it conflicts with the verified Core preservation invariant. Its proposal test/build artifacts and receipt are retained unchanged as historical evidence; they are not current proof of the corrected route. The corrected Session/Core test is the evidence for that route. No new RF identifier is introduced: carry this operation-ownership correction under RF-001 and the source/evidence correction under RF-009, alongside the existing RF-006 board-scope distinction.
