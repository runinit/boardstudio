---
status: accepted
---

# Save before publication and require explicit recovery

A computed Core document is not the **accepted document** until the host reports successful completion of its project-and-asset storage transaction. If saving fails, Session retains that exact result, blocks dependent edits, and retries persistence only; it must not replay the edit. Retries remain subject to storage's atomic asset and conflict checks, not authority to overwrite a newer saved document. If the Core executor fails with an unknown outcome, there is no result safe to retry, so require an explicit reopen/recovery choice. Once Session has emitted a save, a panel unmount may retire its observation but must not cancel the save.

The removed ownership ADR records the trade-off: persist-first requires an engine/history transaction redesign; publish-before-save can diverge the presented state from storage; immediately reopening the prior saved document loses Undo. Fail-stop with save-only retry was the recorded choice. Retrying preserves in-memory Undo only while the same Core remains alive; executor failure or reopen carries no Undo-survival promise. The original was recorded as user-approved on 2026-10-01; this reconstruction and its fail-stop policy were confirmed by the user on 2026-10-19.

**Evidence**

- [Session transitions](../../application/src/session.rs#L1781), [retry test](../../application/tests/durable_session.rs#L403), and [unknown-outcome test](../../application/tests/durable_session.rs#L1044) cover delayed publication, retry without Core replay, and explicit recovery.
- [Browser persistence](../../web/crates/host/src/host/storage.rs#L513), [completion/abort tests](../../web/crates/host/src/host/storage_lifetime_tests.rs#L156), and the [stale-tab conflict test](../../web/crates/host/src/host/storage_lifetime_tests.rs#L917) cover transaction commit, rollback, and conflict protection.
- [Runtime unsubscribe](../../web/crates/runtime/src/runtime.rs#L763) only removes the observer; its [effect driver](../../web/crates/runtime/src/runtime.rs#L1805) continues the owned save. No dedicated unmount-during-save test was found.
- Historical rationale: removed ownership ADR, `323967ff3a67b9b62db4f201b477396d62b55ccd:docs/adr/0003-rust-application-ownership.md`, §§2 and 6; headless Session implementation provenance: `f2494e5714deb726b2e92a392a86bce27de1695a`.
