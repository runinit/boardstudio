# First Dioxus tranche: INT.1, project library, panels/drawers

Source audit only. No repository files, parent issues, task rows, or tickets were changed. Baseline is continuation `530e60dd` and React reference `5a472a9426e6e38993361da402cd4ec730feb369`.

## Proposed ticket breakdown

### 1. INT.1 — Extract the private project and panel composition seam

**Blocked by:** None.

**What it delivers:** The project library and panel work can mount through small private page-crate components using the one existing Runtime/Session and its accepted-document/read-model and open actions. Keep shell mounting/integration at the coordinator-owned seam, and preserve the current F1/F3a shell, theme, workspace navigation, Footprints and layer behavior.

**Acceptance:** A real-browser smoke of the existing shell and restored Layout/Keycaps/Footprints flows remains intact; the library and panel slots can consume the shared current-project/scope inputs and existing saved/demo open actions; opening still uses the current Session lifecycle and stale-open handling; no public API or second Session/store is introduced.

### 2. F2.1 — Browse and safely open saved keyboards

**Blocked by:** 1.

**What it delivers:** The entry/library presents saved keyboards as useful cards, and users can safely open the selected saved project.

**Acceptance:** Public browser coverage shows loading, empty, populated, and list-error-with-retry states; saved cards have stable names, key/board counts, preview-unavailable fallback and a current-project marker without duplicates; opening a saved keyboard uses the existing lifecycle, preserves the current project on failure, reports unavailable/error results, and ignores stale superseded opens. Listing and card browsing do not mutate revision, history, or active selection; an explicit open follows the existing Session opening policy. Search and demo cards are separate tickets.

### 3. F2.1 — Open the Sofle demo family as fresh editable copies

**Blocked by:** 2.

**What it delivers:** Users can choose any of the three Sofle demo variants with a useful preview and counts, then open it as a fresh editable project copy.

**Acceptance:** Public browser coverage verifies Sofle v2, RGB and Choc names/previews/counts and unavailable-preview fallback; each open uses the existing authoritative document/model and electrical-resolution path without duplicating electrical rules, reports recoverable preparation/open errors, and ignores stale superseded results. Each open creates a fresh project identity; repeated opens preserve the previously opened or edited Sofle copy. Opening never overwrites the bundled source fixture. Merely browsing a demo card does not change project revision, history, or active selection.

### 4. F2.1 — Open measured-layout demos as fresh editable copies

**Blocked by:** 2.

**What it delivers:** Users can choose any of the 15 measured keyboard layouts from the reference library, see its preview/counts, and open it as an editable project copy.

**Acceptance:** The public demo catalog maps every measured-layout reference ID and name, including both single-board and split layouts. Previews/counts match the measured source data; preview failure leaves the demo openable. Opening builds or resolves the project through the existing authoritative generator/model/electrical path, with recoverable errors and stale-result rejection, without duplicating electrical algorithms. Every open gets a fresh project identity; repeated opens preserve earlier edited copies and do not overwrite the bundled/source fixture. Browsing alone does not mutate revision, history, or active selection.

### 5. F2.1 — Open the VIK module-review demo as a fresh editable copy

**Blocked by:** 2.

**What it delivers:** Users can choose the reference VIK module-review demo and inspect the same editable fixture through the demo library.

**Acceptance:** The reference card name, preview and counts are present. Opening resolves the pinned module/host-connector data and prepares the review project through existing authoritative providers and model operations; failures remain recoverable, stale work is ignored, and no electrical/module behavior is reimplemented in the presentation. Each open receives a fresh project identity so a prior edited copy survives a later open. Record the private provider/packaging adapter used; if a needed service path is actually absent, identify that bounded gap instead of widening public APIs or substituting a permanent placeholder.

### 6. F2.1 — Search saved keyboards

**Blocked by:** 2.

**What it delivers:** Users can find a saved keyboard by a case-insensitive name search and return to the full list with one clear action.

**Acceptance:** Public browser coverage verifies case-insensitive matching, trimmed query behavior, no-match feedback, and clear-search restoration. Search changes only the visible saved-card list; it does not change current-project identity, active selection, document revision, or Undo history. The list loading and retry behavior remains owned by ticket 2.

### 7. F2.3 — Set desktop panel modes and retain preferences

**Blocked by:** 1.

**What it delivers:** Users can pin, auto-hide, or collapse Objects and Inspect on desktop, and the chosen mode/width preference survives reload where browser storage is available.

**Acceptance:** Public browser coverage exercises both panels’ reference defaults and mode controls, menu dismissal/focus return, clamped saved widths, malformed/unavailable preference storage falling back in memory without blocking work, and verifies panel changes leave document revision, Undo history, and camera unchanged. The unavailable-storage path has no required warning or added UX.

### 8. F2.3 — Resize desktop panels with pointer or keyboard

**Blocked by:** 7.

**What it delivers:** Users can resize each pinned desktop panel with pointer or keyboard while keeping the canvas usable.

**Acceptance:** Public browser coverage verifies Objects bounds of 200–420 CSS px and Inspect bounds of 280–480 CSS px, reference keyboard increments, pointer capture and release on up/cancel/lost capture, and the minimum canvas constraint when both panels are pinned. The resulting width uses ticket 7’s existing persisted preference and does not change document revision, history, or camera.

### 9. F2.3 — Use compact Objects and Inspect drawers

**Blocked by:** 7.

**What it delivers:** On compact viewports, Objects and Inspect open as dismissible drawers so the canvas remains usable.

**Acceptance:** Paired desktop/compact public browser coverage verifies the reference breakpoints, scrim and close control, Escape dismissal, opener focus restoration, and that closed drawer contents are excluded from keyboard focus. Moving across the breakpoint does not strand focus or create a document/history/camera change.

## Parent acceptance mapping

- **INT.1:** Ticket 1 covers private read-model/callback composition, same Runtime/Session ownership, and F1/F3a preservation.
- **F2.1:** Tickets 2–6 cover saved-card content and safe saved open, list states/retry, all 19 reference demos (3 Sofle variants, 15 measured layouts, and the VIK module-review demo), fresh-copy identity, stale/error behavior, and saved search/no-match/clear. Demo acceptance explicitly covers the private packaging/provider path and does not assume today’s fixture button is sufficient.
- **F2.3:** Tickets 7–9 cover desktop modes/preferences/fallback, pointer and keyboard resizing/constraints, and compact drawer behavior/focus exclusion. Each includes the required zero document/history/camera side-effect check.

The broad F2 issue also lists delete, create/import/copy, guide, global command menus and shortcuts. Those belong to other F2 slices (especially F2.2/F2.4), so they are intentionally outside these selected parent rows.

## Source evidence supporting the split

Continuation source (`/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`):

- `web/src/main.rs:1-15,17-28` declares `presentation` and `runtime` in the page binary crate. `web/src/presentation.rs:58-90` constructs one `Runtime`, subscribes the view, and provides that same `Rc<Runtime>` to descendants. `web/src/runtime.rs:50-53,69-106` owns one `Session` and its BrowserStore. This supports a private same-crate extraction and confirms the single-session path.
- `web/src/presentation.rs:550-594` shows today’s library: document listing projects only `(id, name)`, list failures go to global status, and the only demo controls are REVIUNG41 and Sofle v2 fixtures, followed by import and saved-project open. It currently has no cards/search/local loading-retry UI or full demo catalog.
- `web/src/runtime.rs:668-687,689-696,698-763,764-795` shows existing fixture, archive-import and saved-project opening. Fixture/import and saved opens reserve an open sequence and ignore stale completions; saved opens report missing/error outcomes before publishing. However, `open_fixture` imports the packaged document identity unchanged, so it does not prove repeated demo opens are fresh copies. `web/src/runtime.rs:150-160` submits lifecycle events to the existing Session.
- `web/src/host/storage.rs:203-226` provides saved document listing, and `:176-201` provides loading. These are sufficient for the library read path.
- `web/src/presentation.rs:597-605,956-966` shows Objects/Inspect currently use two transient booleans and compact visibility buttons. `web/src/presentation.rs:1111-1123` shows Inspect remains normal workspace presentation; no desktop panel controller exists.
- The issue’s behavioral limits and exact desktop bounds are in `.scratch/dioxus-frontend-v1/issues/02-projects-shared-ui.md`, user stories 4–8 and 18–23, implementation decisions, and Testing Decisions. Task-row acceptance is in `.scratch/dioxus-frontend-v1/tasks.json` for INT.1, F2.1, and F2.3.

React reference source (`/home/chris/01_Projects/ts-boardstudio2`):

- CodeGraph exploration of `app/src/ui/WorkspacePanel.tsx` found mode values `pinned | autohide | collapsed`, side-specific width limits, preference keys `boardstudio:v2:panel:${side}`, default pinned mode, storage exception fallback, and pointer/keyboard resize behavior (`WorkspacePanel.tsx:3-24,52-135`; remaining pointer termination code is after line 135).
- `app/src/ui/ProjectLibrary.tsx:22-72,74-143` provides project-name and damaged-preview fallback, card summary/current marker, asynchronous list status/retry, search/clear/no-match, demo previews and card actions. `:144-153` contains named delete confirmation, outside the selected F2.1 task row.
- Full demo inventory/evidence is in `app/src/demos/keyboards.ts:7-18` and `app/src/demos/sofle.ts:6-11`: 3 Sofle variants, 15 measured layouts and 1 VIK module-review demo. `app/src/demos/keyboardPreviews.ts:5-37` derives previews from measured sources without constructing full projects on browse. `app/src/demos/keyboards.ts:15-83` shows measured keyboard construction and existing core electrical-resolution/application flow; `app/src/demos/moduleReview.ts:28-89,91-174` shows the VIK-specific project/source preparation path. These reference paths explain the bounded family split; they are not permission to duplicate domain/electrical algorithms in Dioxus.
- `app/src/ui/ProjectStart.tsx:7-35` is the reference entry actions and library mount. Project-level create/import behavior is outside selected F2.1. `app/src/demos/keyboards.ts:67-83`, `app/src/demos/sofle.ts`, and `app/src/demos/moduleReview.ts:91-174` provide the family-specific reference open behavior.

## RF handoff

Existing observation: **RF-002** in `.scratch/dioxus-frontend-v1/refactor-findings.json` and `docs/migration/POST-PORT-REFACTOR.md` already records that the page binary and `boardstudio_web` library are separate crates, so `pub(crate)` members in the library cannot be called from the page binary. `web/src/main.rs` confirms Runtime/presentation are page-binary modules. The ticket therefore keeps any extraction private to the page crate and does not propose widening library visibility.

**No new refactoring takeaway observed** in the reviewed INT.1/F2.1/F2.3 seam. Existing one-Session ownership is clear in the source; project-library-local list state and panel preferences can remain presentation concerns.

## Context limitation

Correction: my earlier check searched only `.scratch/dioxus-frontend-v1`; it did not establish that the file was absent from the repository. `CONSTRAINTS.md` exists at the continuation worktree root. I have now read it. Relevant requirements reinforce these ticket boundaries: use independently reviewable vertical slices; preserve stale-result rejection and one authoritative state owner; compare React and Dioxus with matching fixtures/actions including errors and recovery; preserve visual, keyboard/focus, responsive and persisted-project behavior; and retain applicable checks rather than treating builds as parity evidence. Its task-characterization section also asks each task to identify its category, affected capabilities/contracts, reference/fixture, expected result, and checks. The nine proposed tickets remain drafts and should get those dispatch details when assigned.
