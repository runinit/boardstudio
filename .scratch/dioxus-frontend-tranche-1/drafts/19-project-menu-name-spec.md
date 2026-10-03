# F2.2 child 19 — Rename the current project from the Project menu

## Problem

The pinned React editor exposes a current-project name editor in its loaded Project menu. The current Dioxus Library/menu mounts project creation, import, saved cards, demos, and guide entry, but has no project-name input there. Draft 17 ports the distinct Project-stage guide input and explicitly leaves the menu input for this follow-up. This means a user can rename only by opening the guide, and saved project identity cannot be corrected from the ordinary Project menu as in React.

## Solution

Add the `Current project` name input to the loaded Project menu and commit changes through the existing accepted document edit path. Keep a local edit draft; trim and submit on blur or Enter; empty/unchanged input restores the accepted name without an edit; Escape cancels the draft. Gate submission against the accepted project/snapshot captured for the edit. Let the ordinary accepted document flow update the menu trigger, guide draft, saved-card title, and persistence; do not add a rename-specific store, API, history mechanism, or second document authority.

The pinned React source confirms a real menu affordance at `app/src/ui/Workbench.tsx:1247–1248`, distinct from the Project-stage input at line 1172. Both use the same `projectName` draft and `commitProjectName` handler. The handler trims, restores empty or unchanged input, and emits `replace-document` with only the name changed (`Workbench.tsx:588–595`); Enter blurs, and Escape resets the menu input to the current accepted name. An effect follows accepted `document.name` changes without resetting the draft for unrelated revision changes (`Workbench.tsx:478`).

The implementation boundary is callable in Dioxus: `Library` is mounted under the loaded Project menu, and the existing `Runtime::submit(Event::Edit { operation: ReplaceDocument, .. })` persists accepted document changes. A prior implementation commit `81b43cef38249cd9ddbabd3c4da74157af66cf81` contains reusable private `ProjectNameSource` admission and `ReplaceDocument` code, but has no dedicated child packet or independent review and is not in the current integration HEAD. Reuse it only after checking it against this contract; its helper-only tests do not replace the mounted/runtime regressions below. Its adjacent demo-control edits are outside this child.

Direct browser audit: pinned React source `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5173/` exposed a visible textbox named `Project name`, title `Rename project`, value `Sofle v2`, inside the open Project menu. Current root candidate source `d7ff5e3dcae67a719caf6660cfcff57c3811df34` at `http://127.0.0.1:34736/` exposed the loaded Project menu but no `Project name` input. Both fresh sessions imported the same archive, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. DOM snapshots, menu captures, and their hashes are retained in `evidence/project-menu-name-19/source-audit/`.

## User stories

1. As a designer, I can rename the active project from the Project menu so that its name is available without reopening setup.
2. As a designer, I can edit repeatedly and use Enter, blur, or Escape so that the menu behaves like the React editor.
3. As a designer, a rename changes only the accepted project name and follows normal Undo/Redo and save/reopen behavior, so it cannot overwrite unrelated work.

## Requirements

- Render the menu's `Current project` label and input only when a project is accepted. Match React's `aria-label="Project name"`, `title="Rename project"`, and live accepted value.
- Maintain the text draft while the user types. On blur or Enter, trim and commit a nonblank changed name. Empty or unchanged input restores the current accepted name and does not submit an edit. Escape restores the current accepted name and does not submit an edit.
- Follow accepted name transitions so Undo/Redo, another accepted rename, and reopen refresh the field and Project trigger. Do not reset an active draft merely because an unrelated document revision changed. A successful trimmed rename must display the trimmed accepted value.
- Before submission, verify the captured accepted project/document identity and its snapshot/session token and revision against current Runtime state. A stale completion or draft cannot rename the newly active project or replace an intervening edit. Submit through the existing `ReplaceDocument`/Session path; preserve project ID and every field other than `name`.
- The normal accepted flow updates the Project trigger, guide name input, saved-card title, accepted revision, persistence, and history. Undo and Redo restore the accepted name through that same flow.
- Keep the editor usable when persistence rejects/fails; surface the existing Runtime error/report path and do not show a successful new name until it is accepted.
- Provide mounted production-owner regressions for draft state and event semantics, identity/currentness rejection, field preservation, accepted-name synchronization versus unrelated revisions, and exact accepted history/reopen behavior. Include expected-red evidence for the current missing menu input.
- Pair a root-package React/Dioxus browser journey using the same project fixture. Rename twice using trim and Enter/blur, check blank/unchanged/Escape, Undo/Redo, reload, visible trigger/cards/guide synchronization, active project identity, revision/history, and browser errors. Cover the Project menu at desktop and compact viewport in light/dark theme.
- Record RF-006/RF-009 or state that this slice found no distinct refactoring takeaway. Keep public Core/application APIs and member visibility unchanged.

## Boundaries and graph

- Parent remains F2.2. Preserve its F2.1 start edge and INT.2 acceptance join; this child does not alter the canonical 62-task graph or close F2.1, F2.2, F2.4, or F9.
- This source-backed follow-up supersedes the historical F2.2/early planning exclusion of project rename for this concrete Current project menu input only. Retain the history of the earlier source-audit conclusion; do not broaden scope to saved-card rename, duplicate-project actions, or a new rename subsystem.
- Draft 17 continues to own the Project-stage guide name input and guide lifecycle. This child owns the ordinary loaded Project-menu entry only; both surfaces must follow the same accepted project name and normal edit/history flow.
- The Project menu and accepted Runtime edit boundary already exist. No prerequisite on full F2.1 library acceptance is needed for implementation. Parent-level F2.1/INT.2 acceptance remains open.
