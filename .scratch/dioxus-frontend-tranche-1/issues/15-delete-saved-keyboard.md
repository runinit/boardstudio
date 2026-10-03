# 15: Confirm and delete a saved keyboard

**Parent:** F2.2 — Project lifecycle and portable archive (`.scratch/dioxus-frontend-v1/issues/02-projects-shared-ui.md`).

**Status:** implementing

**Source pin:** React `app/src/ui/ProjectLibrary.tsx` at baseline `5a472a9426e6e38993361da402cd4ec730feb369` (delete control and native confirmation dialog, lines 48–53 and 85–105); `app/src/ui/Workbench.tsx` (delete callback handoff, lines 1256–1261); and `app/src/createProjectActions.ts` (current-project replacement and safe persistence ordering, lines 133–153).

**Implementation boundary:** The library uses the existing `Runtime`/`Session` open and persistence path, `OperationOutcomes` exact operation observation, and `BrowserStore` deletion. A private Runtime lifecycle owner holds the project-open route while a deletion is pending, opens the first alphabetically sorted remaining record (or the existing blank-keyboard creation path when none remain), and deletes the requested record only after the replacement operation reports `Completed` and its exact accepted document is current. No new store, Session, or public API is introduced.

- [ ] Saved cards in the Project menu expose a named Delete button. Its modal dialog names the selected keyboard, explains the local irreversible removal, supports Cancel and Escape, prevents duplicate submission, and returns focus to the invoking delete button on dismissal.
- [ ] Deleting a non-current keyboard removes only that saved record; the active accepted document and Session history remain unchanged. Deleting the current keyboard first opens and durably accepts the alphabetically first remaining saved record, or creates a fresh blank keyboard only when no records remain, and only then deletes the old record. Failure to open the first remaining record preserves the old saved record and current project without trying a later record.
- [ ] The dialog shows progress, keeps the selected record available on storage/replacement failure with a retryable alert, and restores focus to search after success. A stale or superseded open cannot authorize deletion. Opening another project is held while the private deletion lifecycle owns replacement and deletion.
- [ ] Mounted regressions cover cancel/Escape, non-current deletion, current replacement and deletion, no-remaining fallback, first-replacement failure, storage failure/retry, duplicate-submit prevention, route-owner currentness, and preserved accepted history for non-current deletion.
- [ ] Paired browser acceptance checks React and Dioxus against one fixture, including IndexedDB records and active-project ID, compact layout, focus, keyboard behavior and page errors.
- [ ] Record relevant evidence under existing RF-001/RF-002/RF-006/RF-008/RF-009. Keep F2.1/F2.2, shared accessibility/build, parent and tranche acceptance gates open.

**Dependency boundary:** The implementation can start from the existing saved-card ID and storage deletion APIs. This child does not close F2.1, F2.2, F9, or the tranche's complete browser journey.

**Implementation evidence:** [Mounted and static checks](../evidence/saved-keyboard-search-delete-20261003/implementation-checks.md). The slice adds no new deferred refactoring finding; preserve the existing RF-001/RF-006/RF-009 ledger and broader acceptance gates.
