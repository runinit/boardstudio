# 19: Rename the current project from the Project menu

**Parent:** F2.2 — Project lifecycle and portable archive (`.scratch/dioxus-frontend-v1/issues/02-projects-shared-ui.md`). This child leaves all 62 canonical parent records and graph edges unchanged.

**What to build:** In the loaded Project menu, show React's `Current project` name editor. Repeatedly rename the accepted active document through the existing Runtime/Session `ReplaceDocument` path, preserving normal history and persistence. Match React's draft, trim, blur, Enter, Escape, blank, and unchanged behavior. This is a separate menu entry from Draft 17's Project-stage input; both follow the same accepted name.

**Start gate:** The loaded Project menu mounts the private `Library`, and the current accepted document can be edited and persisted through Runtime/Session. A prior private implementation at `81b43cef38249cd9ddbabd3c4da74157af66cf81` proves the project-name edit seam is callable, but its helper-only tests and unreviewed state do not satisfy this issue's production regressions. Reuse its private code only after verifying the requirements here. No dependency on completion of the whole F2.1 saved-library workflow is required.

**Blocked by:** None for the menu field and accepted document edit. Do not change the canonical F2.2 F2.1 start edge or INT.2 acceptance join.

**Status:** awaiting independent Spec and Standards review

- [ ] In the loaded Project menu, show `Current project` and a visible textbox named `Project name`, titled `Rename project`, populated with the accepted document name. Do not show this editor without an accepted project.
- [ ] Typing updates only a local draft. Blur or Enter trims and commits a nonblank changed name; blank/whitespace or unchanged text restores the accepted name without an edit; Escape cancels and restores the accepted name. A successful trimmed rename displays the trimmed accepted value.
- [ ] Follow accepted name changes from Undo/Redo, repeated rename, and saved reopen. Preserve a draft through unrelated revision changes, matching React's `useEffect([document.name])` dependency. Keep the guide-stage input and Project trigger in sync with the accepted name without resetting guide stage/open preference.
- [ ] Capture accepted project/document ID, token/session identity, and revision for the draft. Reject stale submission after project/snapshot change and never overwrite a newer accepted edit. Preserve all document fields except `name` and use the existing private Runtime/Session `ReplaceDocument` path. No direct document mutation, new store/history authority, or public API/member visibility change.
- [ ] Verify accepted rename updates the Project trigger, guide field, saved-card title, persisted document, revision, and normal Undo/Redo/reopen state. Persistence failure retains the accepted name and reports via existing Runtime feedback.
- [ ] Add production-mounted regressions for menu event behavior, draft preservation/synchronization, currentness rejection, unrelated-field preservation, accepted edit history, and persistence/reopen. Retain the current missing-field browser observation as the expected-red baseline; helper-only projection tests are insufficient.
- [ ] Pair the public React/Dioxus Project-menu journey on the same project archive in desktop/compact and light/dark. Cover repeat rename, blur, Enter, Escape, blank/unchanged, Undo/Redo, reload, active identity, trigger/guide/card synchronization, and browser errors.
- [ ] Record applicable RF-006/RF-009 takeaways or “No new refactoring takeaway observed.”

**Source and historical scope:** Pinned React `5a472a9426e6e38993361da402cd4ec730feb369`: `Workbench.tsx:478,588–595,1247–1248`. The prior F2.2 statement excluding project rename was based on an earlier incomplete source inspection. This reviewed follow-up supersedes it only for React's `Current project` menu field, retaining the historical decision record. Saved-card rename and project duplication remain out of scope. Evidence of React's field and its absence from the root candidate is retained in `evidence/project-menu-name-19/source-audit/`.

**Completion joins:** Preserve F2.2's F2.1 start edge and INT.2 acceptance join. This bounded child does not close F2.2, F2.1, F2.4, or F9, and it adds no canonical graph edge.
