# Independent Standards: Keymap controller closure

Reviewed `34af9174df789137b358ee8e6e97ae2626cb6fe5...a8c9524ba6084aca1e777bdb76b6c3ff0fd6bc94` in the Keymap worker; controller blob `e3409ed39b2403161b81c507de58b0d72b557efd`. One commit: “Key layer errors by accepted state.”

No new material Standards finding in this delta. Rejected rename feedback now retains only the admitted target name and requires that same accepted name when deciding whether the failure still applies. An unrelated accepted update can preserve the error; a changed accepted name makes the old draft/error context obsolete. Virtual Base uses the explicit existing Base fallback. This preserves bounded ownership without copying a Keymap or predicting Core mutation.

Completed mismatch feedback now follows its acknowledgement token and the actual requested result, independently of the ordinary terminal-failure relevance rule. In particular, Add followed by a later rename no longer loses mismatch feedback merely because the added target exists. Feedback expires when that acknowledgement snapshot is replaced. Existing full Scope/generation/operation targeting and resolved-layer admission are unchanged.

Two focused tests cover admitted-name invalidation and acknowledgement-token Add mismatch. Their source supports the intended boundaries; execution, mounted hook behavior, repeated retries, persistence recovery and public acceptance are not established by this review. The previously noted temporary Signal read-guard integration/compiler follow-up remains outside Standards counting; this worker was not compiled here.

RF: no new refactoring takeaway observed. Bounded admitted-name data is appropriate under CONSTRAINTS.md ownership/copy rules; no performance claim follows. Diff whitespace passed. No source edits or Cargo/browser execution.

## Viewer documentation closure

The previously reported stale model/prepared/cached-module claims are corrected throughout the current private contract and extension documentation. Contract SHA-256 `acc0de11dfcce5356c61887d9f1e76a8783f7484c5c6ba8da145a8c38e5e2b40`; extension SHA-256 `7c2e4a521cae4aeb8fa7714fb8c91b9c750ad2820bd054105bde22aecfae6f36`. They accurately distinguish the current checked full-scene path from open prepared/patch/model adoption gates. The prior documentation finding is closed. Root-reported 24 native tests and strict native/WASM checks remain bounded compiler/native evidence; public viewer, lifecycle/accessibility and five-consumer acceptance gates remain open.
