# Independent Standards: latest Case editor/controller

Reviewed editor `56323af5...03cacf16` at integration HEAD `75139d74bd164695c84683a3174b8ec705ec2763`; final source blobs: `case_bodies.rs` `ff666cdc0e47838e51fad23df168c33fc59c65f3`, untracked `case_controller.rs` `6cac7c868c70b528d4d4788b6db7ff1081f09b51`, `operation_outcomes.rs` `21e5e15c689001314bacf0e5f8a6663879dc3c70`, `runtime.rs` `7b3758c9a11db84139961022d60b6bbf78686d67`. Contract: `/tmp/frontend-run/case-private-contract.md` SHA-256 `fa85c627c5afef07f91badb65e910cead19ac1e7f0eadd66b5803b66f1f95a48`.

**No material Standards finding in these corrected sources.** Prior bounded findings are resolved:

- `case_controller.rs:227–237` keys the complete child by editor identity/full Scope, discarding busy/pending/draft state on navigation. Token/revision are deliberately excluded, preserving same-scope dirty drafts. Hooks and projection memo precede early returns; removed bodies/mounts and generated-stack hiding unmount their fields.
- `case_bodies.rs:579–613` reconciles clean fields with accepted changes, including Undo; dirty drafts survive unrelated accepted updates. Saved acknowledgement is consumed once. Failure settlement depends on request identity plus state, permitting repeated identical failed submissions. Escape clears local draft/error/dedup state and suppresses that field’s displayed failure.
- Synchronous shared `submission_busy` blocks a second submission before rerender; matching full request feedback releases it. Controller rejection of a competing request preserves the admitted operation’s feedback. This closes the prior acknowledgement-stealing path.
- Controller observes a fresh OperationId before submission, and Runtime stores its exact terminal outcome before notification. Global error-string comparison is gone. Saved acknowledgement additionally checks the accepted body and Ready/Saved state; stale scopes discard observation. Weak slots avoid retaining unmounted consumers or terminal history.
- Effective projection is memoized by accepted token/full Scope and shared through Rc. Approved body props remain owned snapshots; fresh callback admission and nested edits use the authoritative accepted document. No public library API changed.

These follow `CONSTRAINTS.md` state/lifecycle ownership and deliberate reactive dependency/copy rules. RF: no new material Fowler smell; the operation-correlated private seam addresses the demonstrated feedback need.

Read-only review; tracked diff whitespace check passed. No Cargo/browser verification performed. Native slot tests do not establish WASM mount, actual repeated-error retry, Undo/draft, navigation, persistence recovery, or accessibility behavior. Keep those public/compiled gates open; stage/hash new files before production provenance capture.
