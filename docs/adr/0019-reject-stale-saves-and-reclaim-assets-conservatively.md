---
status: accepted
---

# Reject stale saves and reclaim assets conservatively

Allow multiple editor tabs, but reject a save if the stored project has changed since that editor observed it, even when revision numbers match. Check the expected stored content atomically with the project-and-asset write. Preserve the losing editor's pending work for explicit recovery rather than silently merging, overwriting or discarding it. The user confirmed this policy on 2026-10-19, choosing occasional conflicts over a single-editor restriction or silent data loss.

Automatic asset reclamation must defer when an open editor might still need assets for Undo, a project is unreadable, or safe editor coordination cannot be established. This deliberately trades prompt disk reclamation for preserving potentially live data; it does not prohibit explicit user-directed deletion. Save failure and retained-result retry follow [ADR-0013](0013-save-before-publication-and-explicit-recovery.md), and retry does not bypass conflict checks.

Evidence: [atomic browser persistence](../../web/crates/host/src/host/storage.rs) and [storage regression tests](../../web/crates/host/src/host/storage_lifetime_tests.rs), including `stale_tab_save_preserves_the_other_tabs_document`, `startup_reclaims_only_unused_assets_after_editors_close`, `startup_keeps_assets_when_a_legacy_window_is_present` and `unreadable_project_aborts_cleanup_and_can_be_recovered_or_deleted`. The current behavior is implementation evidence; this record does not invent an earlier choice among concurrency protocols.
