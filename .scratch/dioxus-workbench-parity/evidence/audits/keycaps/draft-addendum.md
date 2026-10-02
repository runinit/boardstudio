# Draft: F6 Keycaps parity addendum and bounded vertical tickets

**Status:** planning draft only, for reviewer/root composition. Not published; no graph or ticket status changed. Scope is the existing F6 Keycaps lane (F6C.1–F6C.5) and F7.3 issue 05 consumer. Keymap remains its own F6 lane. Preserve all 62 parent definitions and every existing start/acceptance edge.

## Decision from parity audit

Start with the already-published F6C.1 physical 2D slice. It must put a real Keycaps canvas, canonical key selection and a concise effective-state Inspector summary on the public route. The summary may show actual projected key identity, physical dimensions, effective color and whether legend is inherited or explicit/blank. It must be sourced from the accepted document and existing resolution/default rules; it must not be a disconnected mock editor. F6C.2 owns saved settings controls and command/history integration.

This order keeps the independent Keycaps 2D authoring surface moving after INT.1. Neither 2D view nor settings forms are blocked by the missing CAD host adapter or F7 viewer. F6C.5 alone joins preview/STEP adapters and later F7/F8/BND gates.

## Existing graph map (unchanged)

| Existing child | Proposed visible vertical slice / ownership | Start prerequisite | Acceptance join |
| --- | --- | --- | --- |
| F6C.1 | Physical 2D caps + matrix list + searchable/shared selection + read-only selected-key Inspector. Real current projection; empty/standalone/mixed matrix, inherited/blank legend; no persisted defaults. | INT.1 | F3.1 |
| F6C.2 | Saveable board/matrix/key profile, socket, row, color, dimensions, legend and clearance settings through existing commands; drafts/errors/Undo/Redo/reload. | F6C.1 | none |
| F6C.3 | Key-size controls and linked matrix resize/reflow with one history transaction and Layout callback. | F6C.1 | F3.2, F3.5 |
| F6C.4 | Existing fit resolver/findings, stale-case state and affected-key navigation. | F6C.2 | INT.2 |
| F6C.5 | Keymap/Keycaps view toggle consuming the F7 viewer; actual keycap model preview, retry and keycap STEP export through the reviewed private CAD/worker adapter. | F6C.4 + F7.1 | F7.3, F8.2, BND.1 |
| F7.3 issue 05 | Keycaps consumer of the shared viewer: canonical board/pose, identity-safe picks, visibility/suppression and current-document overlays. It is not a duplicate F6 editor. | F7.1; common model bridge is implementation coordination only | INT.2, BND.1; F7.8 later cross-workflow join |

Keep F6C.1’s F3.1 join, F6C.3’s F3.2/F3.5 joins, F6C.4’s INT.2 join, F6C.5’s F7.3/F8.2/BND.1 joins, and F7.3’s INT.2/BND.1 joins exactly as they stand. F3.7/F9.2 remain downstream acceptance; a 2D ticket does not close them.

## Ticket draft 01 — F6C.1 physical 2D and current-key Inspector

**Outcome.** From the public Keycaps tab, show the current board’s supported physical keycaps in a 2D canvas; select the same stable key through canvas and searchable key list; show a real, read-only selected-key summary for projected size, color, legend value and inheritance/explicit-blank state. This is the visible first slice, not a mockup or a disconnected set of controls.

**Use current seams.** Accepted document/session, active board, current canonical selection callback, existing keycap projection/default formula, and private Dioxus page modules. Do not introduce another selected-key store or persisted defaults. Keymap’s active-layer binding labels remain a separate view.

**States/oracle.** No board, empty board, standalone switch, mixed matrices, missing and per-key override values, no search match, keyboard Enter/Space and pointer selection; selection stays scoped and history/revision/document are unchanged by inspect/search. Compare same archive and actions with React. No editor writes, fit adapter, CAD, shared 3D, or STEP in this ticket.

**Gates.** Start INT.1; acceptance F3.1. Root owns route/module mount/shared canvas selection/CSS. Keep parent acceptance open; capture fixture/actions, public result and RF-005 record.

## Ticket draft 02 — F6C.2 settings and durable edits

**Outcome.** Board keycap/legend colors and clearance; matrix profile, socket, first row and wall thickness; selected-key legend, explicit blank, color/profile/socket/row/width/depth overrides. Inheritance stays visible and distinct from authored empty values.

**Use current seams.** `SetKeycapBoard`, `SetMatrixKeycaps`, `SetKeycapKey`, existing Core validation/resolution and normal edit/history. Draft/commit/cancel must match React blur/selection/workspace behavior. No new schema, validation semantics, or public APIs.

**States/oracle.** All supported profiles/mounts/ranges; invalid values and service failures retain accepted values with actionable feedback; Undo/Redo/save/reload and selection switch; inherited legend updates after Keymap edit. Paired accepted document/settings and visible state from same pinned archive.

**Gates.** Start after F6C.1; no new canonical acceptance edge.

## Ticket draft 03 — F6C.3 shared size draft and reflow

**Outcome.** Width/depth sliders, Wide/Tall actions, mixed selection, keyboard/pointer editing and single commit; preserve canonical linked-half resize/reflow and neighboring geometry behavior.

**Use current seams.** Port the existing TS resize/reflow policy privately (`planKeycapResize.ts`, `keycapReflow.ts`) and hand off F3 Layout callback ownership. Existing document/history path only. No Core algorithm or API rewrite.

**States/oracle.** Quarter-unit increments, per-axis commits, duplicates, draft reset on selection/project change, stable affected IDs, one undo entry, Undo/Redo/save/reopen, positions/outline compared with React on paired fixtures.

**Gates.** Start after F6C.1; acceptance F3.2 and F3.5.

## Ticket draft 04 — F6C.4 fit and navigation

**Outcome.** Current `ResolveKeycaps` output, its validation/clearance findings and navigation to the exact selected keys or geometry; current-case revision status is legible.

**Use current seams.** Private query adapter with request identity/revision/scope and stale reply rejection. Existing conservative fit wording and Core resolver only; no new fit algorithm/public read endpoint.

**States/oracle.** Invalid profile/socket/row/wall/dimensions/color/legend/roof, unsupported key, neighbor swept envelope, case body/feature warnings, stale Case geometry, keymap-edited inherited legend, failed/late resolution. Old accepted result remains on failure or supersession.

**Gates.** Start after F6C.2; acceptance INT.2.

## Ticket draft 05 — F6C.5 shared 3D and Keycap STEP consumer

**Outcome.** Keycaps can toggle between existing 2D and F7-owned 3D shared assembly viewer, see generated keycaps and separate legend inlays, retry delivery failures, and invoke the actual keycap STEP action.

**Use current seams.** Existing resolver plus BND.1-proven private CAD worker/service and download provider; F7 shared viewer; current snapshot/revision checks and cancellation. No second viewer, duplicate CAD authority, fabricated meshes, or public API/schema change.

**States/oracle.** Loading/unavailable/error/retry, revision/board/project change during work, teardown, valid downloaded STEP bytes, readiness/error/empty results and matching React. Keep non-CAD 2D editor independent of unavailable CAD preview.

**Gates.** Start after F6C.4 and F7.1; acceptance F7.3, F8.2, BND.1. F7.8 remains later cross-workflow acceptance.

## Ticket draft 06 — F7.3 issue 05 Keycaps shared-viewer consumer

Continue the already-published issue 05, not a new F7.3 child or parent. Adopt the reviewed shared model bridge; provide current Keycaps board/model/contour, current-document-only Case overlay, renderer board-reference pose/elevation, identity-correct producer picks, and generated-keycap model suppression where generated keycaps exist. Preserve separate Keycaps projection and F6 edit ownership. Common model delivery stays F7.3; this ticket does not implement the F6 editor or CAD generator.

Start F7.1; shared bridge arrival is implementation coordination only. Keep INT.2 and BND.1 acceptance joins; F7.8 remains the later adoption join through F6C.5. Verify through the integrated public route with fixture, current/stale selection, missing/error/retry, and cleanup.

## Evidence and open checks

Audit screenshots/action trail are in `/tmp/frontend-parity-reset-20261002/keycaps/`. Use identical fixture hashes for acceptance; this audit used separate REVIUNG entries and is qualitative only. Existing issue 01 for F6C.1 and issue 05 for F7.3 should be continued/clarified, not duplicated. No ticket has been published or marked complete. RF-005 remains the handoff register; the only new note is the exact cross-layer ownership split between Core edit/resolve, TS reflow policy, private CAD worker and shared F7 viewer.
