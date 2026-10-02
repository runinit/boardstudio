Decision: one material lifecycle finding remains; earlier four findings are cleared at source level.

Reviewed `case_bodies.rs` from 03cacf16, SHA256 191cffc4f1f641fa783d8691ee1433add230112dec5c3b4e1d89e21fbda6dbe1, and untracked `case_controller.rs`, SHA256 f6e3a09509037555b1cdac07d41081d10105a40f48fd714c8c03530a2af84c90, against approved fa85c627 contract. No source edits, compiler or browser work.

**P1 — scope changes can permanently retain the child’s busy lock.** `case_bodies.rs:172,238-265` keeps submission_busy in CaseBodies component state and clears it only on matching terminal feedback. `case_controller.rs:75-83` correctly discards pending/feedback when full Scope changes, but its CaseBodies mount is unkeyed. The child survives, rejects its old-scope feedback, and never resets its busy flag. Reproduction: submit an edit, then change board/instance before settlement; new-scope bodies remain disabled even after Session becomes Ready/saved. Session navigation permits that change while accepted data exists (`application/src/session.rs:640-658`). The contract requires “Drop/reset stale field drafts on document, session, board or applicable instance changes.” Key/reset the entire CaseBodies request/busy state by editor identity plus full Scope, rather than only the inner numeric subtree. Do not key it by every accepted token: that would discard valid failed drafts. Verify delayed completion cannot acknowledge the new scope.

Cleared corrections:

- Clean fields now reconcile accepted changes/Undo; dirty failed drafts survive, and matching Saved acknowledgement is consumed once.
- Failed feedback effects include full request identity plus state, permitting repeated identical immediate failures/retries; Enter/blur dedup and Escape restoration remain.
- Exact OperationId observation is registered before submit. Identical core rejection reasons settle independently; full editor/Scope/document/epoch guards discard obsolete ownership. Completed additionally requires the expected canonical body and current saved revision.
- SetKind supplies missing 14/2 wall defaults while preserving existing values.
- Child single-flight state prevents replacing the admitted pending request. Root rechecks live Ready/saved/no-preview/no-gesture/token/revision, patches fresh canonical bodies, preserves siblings and returns exact created IDs.

Root composition, configured-board guarded navigation, generated/mismatch joins, compiler and paired public busy/recovery/history/keyboard evidence remain open. The stricter Ready/saved restriction has no parity waiver. No public API/schema expansion. Refactoring: no new takeaway.
