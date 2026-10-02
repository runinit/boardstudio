# F2 — Projects, panels and shared controls

**Triage:** ready-for-agent. Implementation/acceptance dispatch follows the [slice graph](../tasks.json) and [agent execution plan](../EXECUTION.md).

## Problem Statement

People cannot yet use the Rust/Dioxus candidate as the complete entry and navigation layer for BoardStudio. The current screen can restore an active project, open a saved project or bundled demo, import an archive, choose a theme, and expose basic compact Objects/Inspect visibility. It does not provide the React library’s loading/retry/search/delete-confirmation behavior, complete new-project and portable-copy flows, persisted/resizable panel modes, or the shared command/menu and guide interactions. Project and panel actions therefore have inconsistent error recovery, focus behavior, and responsive behavior compared with the pinned React reference.

The requested F2 scope is a frontend port. Existing Rust session, browser storage, archive worker, and demo fixture boundaries remain authoritative where available. Missing operations must be identified as private frontend adapters or explicit service gaps; this plan does not authorize backend rewrites, public API/schema/visibility changes, or a production entrypoint switch.

## Solution

Complete project discovery and management, guided setup, panel behavior, and shared menu/focus behavior in the Dioxus UI against the pinned React reference. Keep project documents and history in the existing session/runtime and durable storage; keep panel, guide, menu, and focus state in presentation state with browser-scoped preferences where the reference persists it. Route file import and portable-copy output through the existing archive worker boundary. Add only the smallest private adapters needed to invoke existing storage/session contracts; document operations for which the contracts do not exist.

Use the current public UI as the highest acceptance seam: exercise visible controls in a real browser and compare the same fixture, actions, viewport, theme, and outcomes against React. F1’s shell/runtime composition is coordinator-owned. Workflow slices integrate through private presentation components and callback/read-model props; they must not edit the shared shell/runtime concurrently. No additional seam is needed.

## User Stories

1. As a first-time user, I want a project start screen with clear New project, Open project, saved keyboard, and demo choices, so that I can begin without understanding the editor first.
2. As a returning user, I want the last active saved project restored when it is available, so that I can continue where I left off.
3. As a returning user, I want an unavailable or incompatible active project to produce a useful recovery state with a route back to the library, so that a bad record does not block healthy projects.
4. As a user with saved keyboards, I want the library to list the current document and saved documents without duplicates, sorted consistently, so that I can identify the copy I am using.
5. As a user with many projects, I want case-insensitive name search and a clear-search action, so that I can find a project quickly.
6. As a user, I want the library to report loading, empty, and storage-error states and offer retry after a list failure, so that I can distinguish no projects from unavailable storage.
7. As a new user, I want demo cards with useful previews and key/board counts, so that I can choose an example before opening it.
8. As a user opening a damaged or legacy project, I want an unavailable-preview fallback that leaves the project accessible, so that one malformed preview does not hide other projects.
9. As a user, I want to create a new empty project through the existing document/session flow, so that I can start from a clean document without resetting unrelated saved work.
10. As a user, I want guided setup to open for a newly created project and remain associated with that project, so that the guide does not leak state across project switches.
11. As a user, I want to open a demo as an editable copy, so that experimenting does not overwrite the bundled example.
12. As a user, I want to open a saved project and receive a clear unavailable/error outcome if loading fails, so that I can choose another project without losing the current state.
13. As a user, I want to import a BoardStudio archive and see read/validation errors before it replaces my current project, so that an invalid archive cannot silently discard the open work.
14. As a user importing a project, I want late import results from an earlier selection ignored after I choose a newer file or project, so that stale asynchronous completion cannot switch my workspace unexpectedly.
15. As a user, I want to save a portable project copy containing the whole project and referenced assets, so that I can back it up or move it to another browser.
16. As a user, I want to delete a saved project only after a named confirmation, so that accidental clicks do not remove my work.
17. As a user, I want delete progress, a recoverable failure message, and focus returned to the library search after success or cancel, so that keyboard users can continue managing projects.
18. As a user, I want the library to distinguish the currently open saved project from other copies, so that I understand which project subsequent edits affect.
19. As a user, I want the Objects and Inspect panels to support pinned, auto-hide, and collapsed desktop modes, so that I can choose how much canvas space they use.
20. As a user, I want to resize each desktop panel with a pointer or keyboard and have widths clamped to usable bounds, so that panel adjustment remains possible without a mouse and does not consume the canvas.
21. As a user, I want panel mode and width preferences retained across reloads, so that the workbench returns to my chosen layout.
22. As a compact-screen user, I want Objects and Inspect panels to open as dismissible drawers with a scrim and close control, so that the canvas remains usable on a narrow viewport.
23. As a compact-screen user, I want Escape and the close control to dismiss a drawer and return focus to its opener, so that I can continue keyboard navigation predictably.
24. As a user, I want panel option menus to close on outside interaction or Escape and restore focus to the menu trigger, so that transient controls do not strand focus.
25. As a user, I want shared command menus to position within the viewport, focus their first useful control, and dismiss on outside interaction, Escape, or an explicit close action, so that commands remain reachable even near screen edges.
26. As a keyboard user, I want supported workbench shortcuts to avoid firing while typing in an input or editing text, so that shortcut handling does not corrupt names, search terms, or numeric drafts.
27. As a keyboard user, I want menu, drawer, panel, and guide transitions to preserve a logical focus order and return focus to the initiating control when dismissed, so that all F2 workflows remain operable without a pointer.
28. As a user, I want the setup guide to show the five reference stages, current stage, readiness indication, stage detail, Previous, and Back to objects actions, so that I can move through setup without losing work.
29. As a user, I want to move freely between setup stages and preserve the current stage/open preference per project, so that I can pause and resume guidance.
30. As a user, I want an explicit return from the guide to the Objects view and a reliable way to reopen it, so that guidance never traps me in a mode.
31. As a user, I want Light, Dark, and System theme preferences to apply consistently to the entry screen, menus, panels, and compact drawers, so that presentation remains legible throughout F2.
32. As a user, I want unavailable browser preference storage to leave the application usable with a clear in-memory fallback, so that optional preferences do not block project work.
33. As a user, I want long project names and empty, loading, selected, confirmation, and error states to remain readable and operable at desktop and compact sizes, so that the same workflows work across supported viewport sizes.
34. As a user, I want project switching to preserve accepted saved work and respect the existing session-opening history policy, while panel and workspace changes preserve the current Undo history and save state, so that presentation changes never create document mutations.
35. As a maintainer, I want each React responsibility in the source inventory mapped to a Dioxus owner or an explicit retained test/demo disposition, so that a partial component port is not mistaken for F2 completion.

## Implementation Decisions

- Dioxus owns presentation state: search text, selected library presentation state, panel reveal/menu/drawer state, guide stage/open state, and focus transitions. Durable project data and editing history remain owned by the existing Rust session and browser store.
- Preserve browser-local storage semantics and keys for project-scoped setup-guide state, panel preferences, theme, and existing tree grouping where those preferences are in scope. Validate malformed preference values and continue with defaults when storage is unavailable.
- Keep the current saved-project and active-project storage formats. Use existing document listing/loading/deletion/persistence and archive packing/unpacking contracts. Archive import must stage and validate its project/assets before opening it; stale open identities remain ineligible to publish.
- New-project and demo flows must use existing Rust document/session and fixture boundaries. Do not create a second durable project authority or reimplement document construction in a presentation component.
- Desktop panel dimensions use the React reference limits (Objects 200–420 CSS px; Inspect 280–480 CSS px) and account for the other pinned panel and minimum canvas width. Resize keyboard controls move by the reference increment; compact drawers do not expose desktop resize handles.
- Preserve semantic labels, dialog roles/descriptions, current/pressed state, keyboard Escape behavior, and focus restoration visible in the reference. Menus and dialogs must not leave hidden content focusable.
- Keep the pinned React source at 5a472a9426e6e38993361da402cd4ec730feb369 as the visual and behavioral oracle. Do not claim a whole shared React component ported when only one responsibility has moved.
- Existing public contracts are sufficient for list/load/open/recovery/archive import/export, active project persistence, and deletion storage primitives. Current Runtime lacks complete create/delete/portable-copy presentation actions; wrap existing public store/session/archive operations privately where possible. A true project rename or project-level duplicate flow has no observed affordance in the inspected React library. The coordinator verified this reference mismatch: project rename/duplicate are removed from the active parity requirements. Keep portable-copy behavior; do not invent new project actions or widen public API visibility.
- Portable archive output already exists in the current Rust Runtime and Export archive control. The remaining F2.2 work is complete Project-menu integration, project-name output and optional used bundled-model embedding, using the existing archive/store boundary and scoped private action. Do not mistake those gaps for a missing archive engine or dispatch duplicate archive infrastructure.
- The current Rust presentation has a project menu and simple compact panel controls, but not the React WorkspacePanel state machine, complete project library, setup guide, or shared CommandMenu behavior. F1 shell/runtime composition and shared route wiring remain coordinator-owned. Slice work is isolated behind private presentation components/callbacks and read-model inputs; integration changes to shell/runtime happen at a coordinator-owned merge point.
- No public Rust API, transport schema, document format, engine behavior, storage migration, production cutover, React removal, new performance budget, or replacement of existing CI/acceptance gates is implied.

## Testing Decisions

- Acceptance tests exercise only observable browser behavior: project cards and previews, search results, loading/error/retry, create/open/import/copy/delete outcomes, saved document identity/assets, panel widths/modes, guide navigation, themes, keyboard interactions, and focus return. Avoid assertions on component names, Rust private fields, or internal state representation.
- For each slice, retain a paired public-UI evidence record for the same project fixture, action sequence, browser version, viewport, device scale, and theme in React and Dioxus. Capture the initial failing Dioxus behavior as red evidence and the corrected public flow as green evidence; compare document/history/persistence side effects as well as pixels and accessible interactions.
- Project-library evidence covers first load, empty store, populated store, damaged preview, failed list with retry, search/no-match/clear, demo open as editable copy, saved-project open/unavailable, and deletion cancel/success/failure. Verify project bytes/assets and current-project identity where operations write or delete data.
- Lifecycle evidence covers new project without resetting unrelated projects, archive import success and malformed archive, two superseding open requests, portable-copy round trip, active-project restoration, recovery-required navigation, and save failure/retry where the workflow touches persistence. Existing application/storage contracts are test oracles; do not substitute a mocked document state for persistence acceptance.
- Panel evidence covers default, pinned, auto-hide, collapsed, resize min/max, keyboard resize, pointer capture/cancel, simultaneous panel canvas constraints, compact breakpoint transitions, scrim/close/Escape, focus restoration, persisted preferences, and unavailable local storage. Verify panel changes do not increment document revision or alter history/camera.
- Menu/guide evidence covers viewport-edge positioning, outside dismissal, Escape, explicit close, first-control focus, trigger restoration, guide stage switching/readiness, per-project saved stage, and shortcut suppression during text entry.
- Prior art includes ProjectLibrary.test.tsx, storage.test.ts, storageReset.test.ts, createProjectActions.test.ts, createProjectExporter.test.ts, useSetupGuide behavior, the public Playwright project-library/startup-recovery/workbench flows, and existing Dioxus public browser regression evidence from F1/F3a. Transfer useful React assertions to the public Dioxus suite before any later React retirement.
- Required affected evidence includes paired public browser flows and focused persistence/archive assertions, plus applicable formatting, Rust checks, WASM build, frontend build, and integration checks under the project’s accepted gates. This planning task itself runs no builds or browser checks.

## Out of Scope

- Engine, generator, CAD, renderer, archive-format, firmware, document-schema, and saved-project storage rewrites.
- Project rename/duplicate UI unless the pinned React reference or an explicit follow-up decision establishes that behavior; the current library source inspection did not find project-level rename or duplicate actions.
- New project-copy semantics, account/cloud sync, cross-browser sync, or remote storage.
- Production entrypoint switch, React island retirement, data-writer cutover, API/schema changes, widening Rust visibility, and native desktop host support.
- F3 Layout tree/inspector editing, F4 Parts editing, later workspace-specific command menus, and F8 export workspace behavior beyond F2’s portable whole-project copy control.
- Broad accessibility or visual qualification for all workspaces. F2 verifies its public flows and carries host-blocked assistive-technology coverage forward under the shared frontend acceptance plan.

## Further Notes

### Slice sequence and integration seam

| Slice | User-visible outcome | Dependencies | Existing boundary / missing adapter | Private ownership and proposed module map | Red → green public UI acceptance evidence | Effort |
|---|---|---|---|---|---|---|
| F2.1 Project start, library, search and demos | Entry/library shows loading, empty, populated, searchable saved projects and demos; preview fallback, current-project marker, retry and demo-as-copy all work. | F1 shell/runtime integration seam; existing store list/load and fixture open are usable. Does not wait for create/delete/copy actions. | BrowserStore list_documents, load_document, active-project restore and fixture open_fixture exist. Missing presentation projection/list status and demo card/preview behavior; no new public service. | Isolated private library/entry components with document summary/preview projection. Coordinator owns route placement in shared shell. | Red: Rust library currently lists only name/id and has no React-equivalent search/cards/retry/preview states. Green: paired browser cards/counts/search/no-match/retry/demo-copy and opening state; no mutation from browsing/search. | L |
| F2.2 Project lifecycle and portable archive | New/open/import/recover/delete and portable project-copy flows expose success, progress, cancellation or recoverable failure without losing unrelated saved documents. | F1 shell seam; F2.1 only for wiring library actions into cards. Lifecycle adapters can be developed independently against store/session/archive boundaries. | Existing session Open/RecoverWithDocument, store save/load/delete/list, core archive unpack, and worker-backed pack contract are available. Runtime lacks complete create/delete/portable-copy actions; add private adapters using existing contracts. React-level rename/duplicate project actions not found and are an explicit reference/spec mismatch. | Private project action adapter separated from presentation. Shared Runtime action wiring stays coordinator-owned; no public API expansion. | Red: no delete action in Rust library, no full project create action, import only reports status, no complete reference-named portable-copy/menu flow with optional bundled-model embedding. Green: public browser new/switch/recover/import/copy round-trip/delete cancel/failure/success with saved document/assets and active ID assertions. | L |
| F2.3 Workspace panels and compact drawers | Objects/Inspect have persisted pin/auto-hide/collapse/width controls on desktop and closeable drawers on compact layouts, with keyboard/pointer resize and focus return. | F1 shared shell dimensions and mount points; does not depend on project lifecycle. | React behavior is entirely presentational; Rust has compact visibility toggles only. No domain adapter needed. | Private panel state/controller and panel presentation components. Coordinator owns shell insertion/layout CSS and shared runtime files; define props/events seam before slice coding. | Red: current controls only toggle compact panel booleans; no desktop modes, resizing, persisted widths or scrim. Green: public desktop/compact interaction matrix; prefs survive reload; revision/history/camera unchanged. | L |
| F2.4 Setup guide, shared menus, shortcuts and focus | Project-scoped five-stage guide, reusable command menu dismissal/positioning/focus, and supported shortcuts behave consistently across entry/library/panels. | Guide integrates with F2.3 panel host. Command menu is independently implementable; global shortcut wiring integrates at coordinator-owned shell. No dependency on F2.2 except project-created request callback. | React guide/menu and preference behavior are presentation-owned. Current Dioxus has Escape for native project details and basic theme selector, but lacks these shared controls. | Private guide/menu components, small presentation state models, focus helper module. Shared shell keyboard handler and event subscription remain coordinator-owned. | Red: Rust candidate has no guide or general command menu/focus behavior. Green: public keyboard/mouse flows including every escape/outside/dismiss path, opener focus return, per-project guide preference, theme and typing-safe shortcuts. | L |

### Concrete gaps and risks

- The roadmap’s F2 language includes rename and duplicate, but the pinned ProjectLibrary.tsx and inspected Workbench project menu expose delete, new, open, demo and portable copy; project-level rename/duplicate affordances were not found. The coordinator resolved this roadmap/reference mismatch in favor of the pinned source: new/open/demo/import/delete/portable copy are required, project rename/duplicate are excluded.
- Runtime currently exposes saved-project open and archive import, but not a full project action facade. BrowserStore has document list/load/delete/active-project persistence primitives; packaging exists at the core archive worker boundary. Private adapter reachability needs a source-level check during implementation before proposing a public contract.
- Project library list failures currently report through the global runtime status and do not create a library-local retry state. F2 needs a library-scoped status result without allowing a failed read to look like an empty library.
- Import is asynchronous and scoped by an open sequence; preserve this supersession rule and do not publish assets/document state before archive validation finishes.
- Current panel settings in React are local preferences while panel visibility and content are session presentation state. Persist width/mode, not transient reveal, drawer open, menu open, or focused element.
- Resize drag must release capture on pointer-up/cancel/lost capture. Compact breakpoint changes and scrim dismissal must not leave hidden panel content in keyboard order.
- Actual assistive-technology testing is unavailable in the carried host evidence; record this limitation rather than treating automated accessible names as full F2 accessibility acceptance.
- F2 is planned against a clean integration baseline c827c4e6 (executable F3a evidence f44a3d1b). No builds, tests, browser sessions, or repository edits were performed while producing this planning artifact.


### Independent review reconciliation

Archive ZIP packing already exists in Runtime, but current packing includes only document-local assets and uses `keyboard.boardstudio`. F2.2 owns the shared archive action and Project-menu control, preserving `${document.name}.boardstudio` and always including local assets. It adds the optional used bundled-model discovery/bytes/hash projection required by the reference. F8 owns the Export-route copy button and Embed used models checkbox and calls that same action. Asset resolution occurs through the existing retained provider/host boundary; no stored format changes are implied.


### Authoritative execution dependencies

The milestone-level prerequisites above describe integration context. The refined rows below replace whole-milestone or symbolic dependencies. Preparation/fixture work may start after `Start after`; completion also requires `Acceptance joins`. Existing F1/F3a source and evidence are baseline prerequisites, not tasks to repeat. Full workflow qualification also joins F2 shared panels/controls under F9.

| Slice | Start after | Acceptance joins |
| --- | --- | --- |
| F2.1 | INT.1 | Own slice acceptance |
| F2.2 | F2.1 | INT.2 |
| F2.3 | INT.1 | Own slice acceptance |
| F2.4 | F2.3 | Own slice acceptance |

### Refactoring observation handoff

Update the [living RF register](../refactor-findings.json) and [post-port takeaways](../../../docs/migration/POST-PORT-REFACTOR.md) for architectural, design, theoretical or quality issues discovered in this slice, or record “No new refactoring takeaway observed” with reviewed scope. Distinguish confirmed findings from hypotheses; include evidence, impact, current mitigation, later proposal and validation. This does not authorize unrelated refactoring or defer required parity fixes.
