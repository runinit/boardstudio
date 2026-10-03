## Problem Statement

**Triage:** ready-for-agent. Implementation/acceptance dispatch follows the [slice graph](../tasks.json) and [agent execution plan](../EXECUTION.md).

The current Dioxus Export view only offers a whole-project archive and one generic STEP export. Designers cannot reach the pinned React export package for a selected board, including KiCad PCB and draft handoffs, firmware, footprints, outlines, or the saved/generated case packages. The React application also exposes whole-project copy and keycap STEP from their local workflows. Leaving these paths as placeholders blocks the final design-to-manufacturing and design-to-portable-project journeys even though Rust already owns most of the archive, geometry, firmware and artifact contracts.

The replacement must preserve the exact reference output choices and accepted snapshot behavior. It must not invent a new output format, duplicate a provider owned by F2/F6/F7, or deliver an artifact for a project/board/instance that changed while it was being prepared.

## Solution

Build the complete Export workspace around private Dioxus presentation and export orchestration, and connect it to the existing archive, core artifact, firmware, electrical, and CAD providers through the coordinator-owned Runtime façade. Keep project-copy behavior in F2, the local firmware/keycap STEP actions in F6, case generation and its local geometry export in F7, and the board/electrical readiness work in F5. F8 presents and coordinates only the reference Export route and reuses those owners; it does not create duplicate provider logic.

The first UI slices can use fixed documents and provider fixtures before F5/F6/F7 finish. Final acceptance waits for their real selected-board, hardware, keymap/keycap, and case data. The React reference has no visible export progress indicator, export-cancel button, or dedicated Retry button: failures appear through the global app error message and retry means invoking the same export action again. Preserve that behavior. Internal cancellation/stale-work rejection and resource cleanup remain required where the existing service lifecycle supports them.

## User Stories

1. As a keyboard designer, I want to open Export from the workbench and see an Export package workspace, so that the available handoffs are clear in one place.
2. As a keyboard designer, I want Export to identify the selected board and explain that downloads use the committed design, so that I know which board the board-scoped handoffs use.
3. As a keyboard designer, I want to return to the workspace I came from with the Export return link, Export trigger, or Escape, so that export review does not strand me in a different workspace.
4. As a keyboard designer, I want the return action to restore the prior Design, PCB, Keymap, Keycaps, Case, or Parts workspace and its supported scope, so that I can resume the same workflow.
5. As a keyboard designer, I want a KiCad board handoff for the selected board only when its PCB readiness and resolved wiring are ready, so that a fabrication-oriented export does not imply that unresolved electrical work is complete.
6. As a keyboard designer, I want a separate Draft KiCad board handoff when the board is ready even if wiring is incomplete, so that I can share an explicitly incomplete board and its findings report.
7. As a keyboard designer, I want disabled KiCad outputs to say what work remains and provide the reference action to review wiring, so that I can resolve the blocker without guessing.
8. As a keyboard designer, I want the full KiCad handoff to contain the selected board output, applicable model assets and electrical handoff records, so that the downloaded package is usable with the existing downstream workflow.
9. As a keyboard designer, I want a completed PCB handoff to protect its electrical assignment state only after packaging succeeds, so that a failed or stale download cannot record a completed handoff.
10. As a keyboard designer, I want a ZMK v0.3.0 package when the selected board has ready resolved wiring, a controller, and at least one assignment, so that the output reflects the configured board rather than inferred pins.
11. As a keyboard designer, I want the firmware package to include its editable starter keymap and electrical plan, so that I can inspect and continue the generated handoff.
12. As a keyboard designer, I want unavailable firmware to explain that PCB wiring/controller work remains, so that a disabled action communicates its actual prerequisite.
13. As a keyboard designer, I want a KiCad footprints library for all project definitions when at least one definition exists, so that I can reuse the component footprints outside the project.
14. As a keyboard designer, I want SVG and DXF board-outline downloads for the selected board only when its resolved outline is ready, so that the vector output matches the active board contour.
15. As a keyboard designer, I want outline blockers to direct me back to review the board outline and layout findings, so that I can repair the source rather than export a misleading contour.
16. As a keyboard designer, I want the saved authored case-body STEP action with the same label and readiness as the React route, so that saved authored geometry remains available from Export.
17. As a keyboard designer, I want the Generated mechanical package action only when the current generated assembly is export-ready, so that the bundled STEP/STL parts, outlines, specifications and FR4 plate project correspond to a valid current mechanical result.
18. As a keyboard designer, I want case readiness messages to distinguish authored case geometry from generated mechanical assembly readiness, so that one does not stand in for the other.
19. As a keyboard designer, I want Export to retain the portable whole-project copy action and its used-model embedding option, so that I can save an editable archive containing all boards and the chosen model assets.
20. As a keyboard designer, I want the portable project copy to use F2's existing archive provider and control, so that the same packaging, asset identity and round-trip behavior is shared with project management.
21. As a keyboard designer, I want the Keycaps workspace's local Export keycap STEP action to remain there, so that F8 does not duplicate the F6 provider or move a reference-local control.
22. As a keyboard designer, I want the Export-route firmware row and Keymap workspace firmware action to call the same F6-owned generation provider, so that both reference entrypoints export the same resolved configuration.
23. As a keyboard designer, I want every export to use a committed accepted document/scene snapshot and the applicable selected board or physical instance, so that a preview draft or unrelated board cannot leak into the artifact.
24. As a keyboard designer, I want each long-running export to reject a result if the project, accepted revision, session, board, or physical-instance scope changes before delivery, so that stale output cannot be downloaded as current.
25. As a keyboard designer, I want a PCB export's own accepted wiring/protection edits to advance its captured snapshot in the same order as the reference, so that intentional export preparation does not invalidate itself.
26. As a keyboard designer, I want export failures surfaced through the existing global app error message with the source document untouched unless the workflow explicitly committed an accepted preparation step, so that recovery is clear and safe.
27. As a keyboard designer, I want to retry a failed export by using the same available export action again, so that retry matches the reference without adding a separate retry workflow.
28. As a keyboard designer, I want successful files downloaded with the expected filename and media type, so that the handoff is recognizable in my file system.
29. As a keyboard designer, I want temporary browser download URLs released after delivery even when download setup fails, so that repeated exports do not leak resources.
30. As a keyboard designer, I want export to show the same ready/Needs work state and reference blocker explanation instead of a new progress indicator or export-cancel button, so that the port does not invent controls absent from the pinned React behavior.
31. As a keyboard designer, I want a new Export operation to remain isolated from CAD preview meshes and caches, so that manufacturing outputs are built from the provider's captured inputs.
32. As a keyboard designer, I want large archive, PCB and CAD preparation to stay off the UI thread and to use existing worker disposal rules, so that export does not freeze the workbench or retain obsolete workers.
33. As a keyboard designer, I want the same board to remain selected after returning from Export, so that I can immediately review the same outline, wiring, or case issue.
34. As a keyboard designer, I want the Export workspace to remain usable in light, dark, compact, short-viewport, keyboard-only, and zoomed layouts, so that all outputs and return controls stay reachable.
35. As a keyboard designer, I want screen-reader labels to name each format, readiness, and disabled reason, so that export choices are understandable without relying on colored status dots.
36. As a keyboard designer, I want project changes made after a queued export starts to keep their normal save/history behavior, so that exports never act as a second editable document session.
37. As a keyboard designer, I want every available provider-backed action to retain its current output bytes, filenames, archive entries, model asset references, and revision checks, so that F8 is a frontend port rather than a format migration.
38. As a maintainer, I want F8-owned state limited to export presentation and private operation coordination, so that F2/F5/F6/F7 retain ownership of project, electrical, keymap/keycap, and mechanical behavior.
39. As a maintainer, I want no public contract, Rust API visibility, project schema, archive schema, or output format widened for UI convenience, so that the migration stays within accepted contracts.
40. As a maintainer, I want end-to-end journeys to reuse one project across Layout, Parts, PCB, Keymap, Keycaps, Case, and Export, so that F8 acceptance proves the joins rather than isolated provider demos.

## Implementation Decisions

- F8 owns the Export workspace composition, format rows, blocker explanations, return navigation integration and private coordination needed to route the existing providers. Shared app shell, `main` composition, Runtime, global CSS and build entrypoints remain coordinator-owned.
- Preserve exactly the reference outputs: `.boardstudio` portable project copy (all boards; optional used-model embedding); KiCad full board handoff; draft KiCad handoff with findings; ZMK firmware ZIP; KiCad footprints library; SVG and DXF selected-board outline; authored Case STEP; generated mechanical package ZIP containing STEP/STL parts, outlines, specifications and FR4 plate project. Keycap STEP remains the local Keycaps action, not a new Export-row duplicate.
- Scope remains per current selected board for board, firmware, outline, authored case and generated mechanical outputs, with any physical instance taken from the current Case workflow context. Footprints use all project definitions. Project copy includes every board. The used-model option and keycap local export stay with F2/F6 owners.
- Preserve each reference readiness rule: full KiCad requires selected-board PCB readiness and ready wiring; draft KiCad requires board readiness and may report incomplete wiring; firmware requires ready wiring, a controller and at least one assignment; footprints require at least one definition; SVG/DXF require selected-board outline readiness; authored Case STEP follows authored-case readiness; the generated package follows current generated-mechanical readiness. Portable archive and Keycaps local action use their existing owner-specific readiness.
- The pinned React Export route has no visible export progress, cancellation, or dedicated retry UI. Its queued worker activity reports failures through the global app error message; the same action is invoked again to retry. Keep operation cancellation and supersession as internal lifecycle behavior, and keep Case generation cancel/retry controls with F7.
- Coordinate around one immutable accepted snapshot and current document/session/board/instance/revision identity. Preserve the existing PCB prepare/commit/protect ordering, worker boundaries, error results, filenames, file contents and final delivery semantics.
- Use existing Rust archive request/reply, CoreRequest/CoreReply, artifact plan/finish, firmware generation, electrical resolution/protection, and CAD STEP providers. Reuse the retained Ergogen source generation only in the existing artifact handoff where it is required; do not add a second provider or generator.
- No new backend algorithm, public service endpoint, public Rust member, request/reply shape, archive/project schema, firmware format or output type is in scope. If source inspection reveals a provider cannot express an exact React action, record that concrete gap; do not substitute a mock artifact or silently remove the action.
- F2 owns the portable whole-project archive service/control; F6 owns firmware and keycap STEP local buttons and generation path; F7 owns the mechanical generation and local geometry action; F5 owns board/hardware readiness and handoff state. F8 integrates those entrypoints and states without duplicating them.

## Testing Decisions

- Test public Dioxus UI actions from reference-style projects and compare artifact bytes/entries, filenames, media types, accepted revision and committed edits with the existing provider outputs. Component-internal state assertions alone do not satisfy acceptance.
- Transfer the React export coordinator tests for queued snapshot capture, stale project/session/board rejection, protected handoff ordering, pack failure, keycap CAD revision, firmware packaging, and delivery. Retain core tests for archive verification, export planning/finalization, outline output and firmware generation, and CAD tests for STEP geometry.
- Exercise every exact readiness boundary above in paired ready/not-ready browser states, including why an output is blocked and its return-to-workspace action. Cover invalid geometry, incomplete wiring draft, export worker failure, archive limit/missing asset, CAD error, stale completion, repeat retry and download cleanup.
- Verify output equivalence for full KiCad and draft KiCad; generated outline SVG/DXF; all definitions in footprints output; ZMK config/keymap/electrical-plan ZIP; case STEP and generated mechanical package; portable archive round-trip with both model embedding settings; and local keycap STEP. Keep keycap UI ownership in F6 tests and portable-copy controls in F2 tests while adding cross-workspace F8 journey coverage.
- Verify edits during queued export, project switch/reopen, selected board change, selected physical instance change, export's own accepted electrical preparation and failed packaging. Downloads must never deliver a stale scope and must not create handoff protection before successful packaging.
- Compare React and Dioxus Export workspace at desktop and compact sizes in light/dark themes; retain selected-board, blocked, ready, error and return states. Include keyboard/focus/axe and relevant manual assistive-technology checks, avoiding claims beyond the available host AT verification.
- Prior art includes `createProjectExporter.test.ts`, `storage.test.ts`, `ExportClient.test.ts`, `export.worker.test.ts`, `exportMechanicalAssembly.test.ts`, Rust archive tests, Rust KiCad output tests, Rust firmware tests, `MechanicalAssemblyPanel.test.tsx`, `KeycapPanel` coverage, and browser cases for board/case readiness and project/export flows.

## Out of Scope

New output formats; format or filename migrations; CAD, firmware, electrical, archive, storage or project-schema rewrites; duplicate provider implementations; Rust public visibility changes; UI export progress/cancel/retry controls absent from the reference; project copy ownership already assigned to F2; local firmware/keycap STEP controls assigned to F6; case generation and local Export geometry control assigned to F7; board/hardware readiness implementation assigned to F5; layout, parts, keymap/keycap, or case feature implementation beyond their export integration; shared shell, Runtime, global CSS, generated build assets or production-entrypoint cutover not assigned to F8.

## Further Notes

### Task slices and ownership

The executable task graph is `/tmp/boardstudio-workflow-plans/F8.json`.

| Slice | Visible outcome | Actual prerequisite | F8-private ownership | Existing provider or gap |
| --- | --- | --- | --- | --- |
| F8.1 | Complete Export workspace with exact rows, readiness reasons and return path | F2 shared frame; can start against fixtures before F5/F6/F7 | Export route composition, output row state/presentation and navigation behavior | Route rows, portable-copy controls, selected-board context, review links and return/Escape/header-toggle behavior were observed on 34763; see the public receipt below. Responsive/theme/accessibility coverage remains. |
| F8.2 | Reusable private export operation coordinator and delivery lifecycle | F2 accepted-snapshot/portable-copy seam; can characterize with fixture providers | Export intent dispatch, snapshot guards, error mapping, completion/download cleanup | Archive, STEP, firmware, outline and standalone-footprint handlers reuse existing owners. They are not yet one generic intent coordinator; public stale/failure/protection ordering remains open. No public contract gap is currently proven. |
| F8.3 | Board, footprint and outline outputs from selected board/library scope | F3/F4 source state and F5 board/outline readiness; build against fixed snapshots earlier | F8 export row to the existing artifact provider; no board/readiness implementation | SVG/DXF downloads were qualified on 34763; the all-definition KiCad footprints ZIP was qualified on 34767. Full/draft KiCad rows remain explicitly unavailable in source, and their combined F5 ordering/protection journey is absent. |
| F8.4 | Firmware and keycap STEP outputs at their reference entrypoints | F5 hardware handoff and F6 keymap/keycap controls; provider tests can start earlier | F8 route integration and shared operation coordinator only | Dioxus Keymap-local ZMK download and Keycaps-local byte-identical STEP are qualified in their owners' evidence; the Export ZMK row is mounted, but its Dioxus click/output is not qualified. F6 retains local ownership. |
| F8.5 | Authored Case STEP and generated mechanical package | F5 instance context and F7 case readiness/generation | F8 output-row integration only | Ordinary authored Case STEP is source-backed only when no generated mechanical configuration is active. F8 route download is not qualified; configured authored-only STEP and generated mechanical ZIP remain concrete provider gaps. |
| F8.6 | Complete saved-project-to-output journeys | F8.1–F8.5 plus F2, F5, F6, F7 completion | Cross-workspace joins, scenario evidence and F8 inventory disposition | No single saved-project journey exercises the remaining providers, snapshot races and packaging/protection ordering together. Existing owner receipts below qualify bounded leaves only. |

### Current F8 evidence and remaining criteria (2026-10-03)

This map distinguishes mounted/source-backed behavior from completed public
qualification. None of these receipts closes F8.2–F8.6 or the F8 parent.

| F8 criterion | Sufficient retained evidence | Concrete remaining behavior / qualification |
| --- | --- | --- |
| Route, row state, return and review links (F8.1) | [34763 Export receipt](../evidence/export-workspace-20261003/34763-RECEIPT.md): seven design rows plus conditional mechanical row, portable-copy option, selected `Left PCB`, blocker guidance, SVG/DXF readiness; Back, Escape and header toggle return to Keycaps. | Layout was qualified at 1280×577 only. Compact/theme/keyboard/AT and disabled-reason destinations still need their exact acceptance evidence. |
| SVG and DXF (F8.3) | 34763 receipt records actual selected-board downloads, names, bytes, hashes, signatures/media types. | No new output-format matrix is needed. Reuse this leaf evidence; full/draft KiCad are still source-disabled and have no Dioxus artifact journey. |
| All-definition footprints (F8.3) | [34767 receipt](../evidence/export-footprints-20261003/RECEIPT.md) records the actual 14-definition ZIP, six model hashes, library table and utilities note. React's repeated-name throw and Core's deterministic collision repair are preserved there. | Bounded footprints child is green. It does not establish full/draft KiCad assembly, shared stale-race handling, or whole-parent acceptance. |
| ZMK and local Keycap STEP (F8.4) | [Current paired Keymap qualification](../evidence/case-keymap-current/keymap-layered-public/current-qualification/RESULTS.md) records Dioxus Keymap-local ZMK delivery; [Keycaps STEP receipt](../evidence/sol-review-wave-20261003/candidate-dd7697ed/STEP-PUBLIC-RECEIPT.json) records byte-identical local STEP. | The Dioxus Export ZMK route click/output and not-ready blocker state are not qualified. Keycap STEP remains F6-owned and is not an Export row. |
| Portable project copy (F8.6/F2.2 join) | 34763 shows the mounted option/control. Other public `.boardstudio` archives prove archive delivery in their own workflows; the shared owner implementation is in F2.2. | No paired Export-route download has verified project-name filename, both model-option semantics, archive round-trip and unchanged live document. Do not infer this from unrelated archive downloads. |
| Full/draft KiCad and ordering (F8.3/F8.6) | Rows/readiness logic are source-visible; F5 owns readiness and accepted electrical state. | `export_workspace.rs` still marks both rows unavailable with no action. Wire existing Core prepare/finish, selected-board models/contours and the accepted wiring commit → package → protection sequence, then qualify the joined artifact path. |
| Authored Case STEP / generated mechanical ZIP (F8.5/F8.6) | Runtime has a source-backed ordinary authored Case STEP path when no generated mechanical configuration is active; F7 has its own generation/local geometry owners. | The Export Case row is currently unavailable when a generated configuration is active; generated mechanical package is explicitly unavailable. Implement/reuse the provider path and qualify selected instance, entries, names and guarded delivery. |
| Shared lifecycle and final joins (F8.2/F8.6) | Bounded footprint, outline, firmware-local and STEP-local receipts exercise their owner paths; session/export identity guards are source-visible. | No public stale-scope, failed-package, retry, URL cleanup, export-owned protection-ordering, or one-project Layout→Parts→PCB→Keymap→Keycaps→Case→Export join has been qualified. F2.2/F3.7/F4.5–4.7/F5.8/F6.6/F7.8 joins remain required. |

The retained [34763 route and SVG/DXF receipt](../evidence/export-workspace-20261003/34763-RECEIPT.md)
and [34767 footprints ZIP receipt](../evidence/export-footprints-20261003/RECEIPT.md)
are bounded evidence, not a substitute for the listed provider or parent joins.

The source-backed private coordinator implementation refinement for F8.2 is in
[08-export-coordinator-dispatch.md](08-export-coordinator-dispatch.md). It
preserves the acceptance above and keeps F8.2/F8.3 open until the current
Runtime-to-Core/artifact/archive path is verified in a public browser journey.

F8.1 and fixture-backed coordinator tests may proceed before the workspace providers are ready. F8.3–F8.5 require their true F3/F4/F5/F6/F7 state for integrated acceptance. F8.6 is the final join and cannot close on fixture-only evidence.

### Source and file map

Pinned React reference is commit `5a472a9426e6e38993361da402cd4ec730feb369`; current clean Rust/Dioxus planning base is `c827c4e69389a77b1f0e8d647ce86a4c41529611`, with executable F3a source `f44a3d1b`.

React source responsibilities are nested: `app/src/ui/Workbench.tsx` owns the Export route, seven format rows, portable-copy option, global Export trigger, Escape, and return control; `app/src/main.tsx` wires the snapshot/services and callbacks; `app/src/createProjectExporter.ts` coordinates queued capture/dispatch/delivery; `app/src/exports/context.ts` guards document/scene/session/board/instance identity; `app/src/exports/documents.ts`, `pcb.ts`, `firmware.ts`, `keycaps.ts`, `cases.ts`, and `assets.ts` build output artifacts; `app/src/ExportClient.ts` and `app/src/export.worker.ts` transport provider operations and transferable buffers; `app/src/storage.ts` packages portable project archives; `app/src/ui/createKeymapWorkspace.tsx` and `KeycapPanel.tsx` retain local firmware/keycap STEP entrypoints; `app/src/ui/CaseGenerationControls.tsx` and `MechanicalAssemblyPanel.tsx` retain local case geometry export.

The Dioxus presentation now exposes project copy, ZMK, outline, standalone footprints and ordinary Case STEP rows/actions. `application::Session` has StartExport, CancelExport, accepted snapshot, export identity and stale completion handling. `web::Runtime` owns CoreWorker, BrowserStore, archive/STEP worker lifecycle, download Blob URL and cancellation-on-supersession/close. `CoreWorker` transports existing Core, Artifact and Archive operations; Rust Core has `prepare_export`/`finish_export`, outline and footprint artifacts, electrical handoff protection and firmware generation. The full/draft KiCad and generated mechanical Export rows remain disconnected. These contracts stay private behind Runtime; no public API gap is currently proven for the implemented leaves.

### Scope and truth notes

React `ExportKind` also includes the internal `keycaps-step` operation, but its only visible entrypoint is in Keycaps; it is listed here solely to preserve the scope/ownership boundary with F6. The Export route contains seven visible design output rows (full KiCad, draft KiCad, ZMK firmware, footprints, SVG, DXF, authored Case STEP), an optional generated mechanical package row, and the portable project copy control. The route has no visible export progress, cancel button or dedicated Retry action. Its global app error can be dismissed; the user repeats the same action to retry. Internal Rust session cancellation exists and must not be presented as a React UI control.

The current bounded export evidence is pinned to sources listed in each receipt; see the criteria reconciliation above. No source/build work is implied by this evidence-map update.

### Independent review reconciliation

Independent review identified three concrete facade gaps beyond ordinary UI wiring. (1) ResolveKeycaps returns specs/findings, but the Rust web CAD enum has no keycap construction/STEP request: BND.1 owns existing-service bridge feasibility, and F6 owns the local action. (2) Runtime pack_archive currently includes only local document.assets and uses a fixed filename; F2.2 adds reference project-name output and optional used bundled-model embedding, while F8 owns the Export-route checkbox/button. All local assets stay included regardless of the checkbox. (3) Session StartExport captures an immutable token and has no export-owned snapshot-adopt transition. BND.2/F8.2 must preserve resolve→own wiring commit→package→own protection commit→guarded delivery using a proven private orchestration; unrelated edits/scope changes still invalidate it. Do not claim this is already solved by Session. If existing contracts cannot safely express it, record the exact contract proposal before changing public APIs. Authored Case STEP is canonical selected-board geometry; generated mechanical output explicitly projects the selected physical instance. URL cleanup on errors is retained Dioxus lifecycle hardening, not an exact React behavior claim.

### Verified keycap adapter boundary

The follow-up source audit found the already exported Rust WASM `build_keycaps` function in `cad/wasm/src/model/keycaps.rs`, re-exported by the CAD package already copied into the Dioxus build. The gap is the Dioxus private host/worker request path, not keycap geometry or STEP engine support. BND.1 proves a crate-private adapter from existing `KeycapSpec` to this export, with stable cap/legend body IDs, revision/scope validation and preview chunks/yields matching the reference. Do not bundle the React UI/client runtime or add a new CAD engine API. Existing public `web::cad_jobs` enum/request types must not be widened implicitly; prefer private wire/host types. Preview cancellation can be observed between chunks; synchronous STEP kernel calls cannot be interrupted mid-call, so cancel/scope change suppresses late delivery. Keycap export resolves with `cases: null` as in the reference. The [boundary report](../evidence/planning/boundary-gaps.md) records the exact source trace and implementation limits.


### Authoritative execution dependencies

The milestone-level prerequisites above describe integration context. The refined rows below replace whole-milestone or symbolic dependencies. Preparation/fixture work may start after `Start after`; completion also requires `Acceptance joins`. Existing F1/F3a source and evidence are baseline prerequisites, not tasks to repeat. Full workflow qualification also joins F2 shared panels/controls under F9.

| Slice | Start after | Acceptance joins |
| --- | --- | --- |
| F8.1 | INT.1 | Own slice acceptance |
| F8.2 | F8.1 | INT.2, BND.2 |
| F8.3 | F8.1, F8.2 | F3.4, F4.3, F5.2, F5.3, F5.5 |
| F8.4 | F8.1, F8.2 | F5.2, F6K.4, F6C.5 |
| F8.5 | F8.1, F8.2 | F7.2, F7.6, F7.7 |
| F8.6 | F8.1, F8.2, F8.3, F8.4, F8.5 | F2.2, F3.7, F4.5, F4.6, F4.7, F5.8, F6.6, F7.8 |

### Refactoring observation handoff

Update the [living RF register](../refactor-findings.json) and [post-port takeaways](../../../docs/migration/POST-PORT-REFACTOR.md) for architectural, design, theoretical or quality issues discovered in this slice, or record “No new refactoring takeaway observed” with reviewed scope. Distinguish confirmed findings from hypotheses; include evidence, impact, current mitigation, later proposal and validation. This does not authorize unrelated refactoring or defer required parity fixes.

### Active reviewer routing and current public evidence

For the current F8.4/F8.6 public Keymap-to-firmware qualification, route the independent review to `sol-review` (Sol 6.1 High, `gpt-6.1-sol`, high). The active routing is also recorded on F8.4 and F8.6 in [tasks.json](../tasks.json). Historical Astra and earlier source reviews remain preserved as completed historical evidence; this route does not replace them. The exact paired current-build results, downloaded ZMK packages, archive identity, snapshots, and hashes are in [the current qualification bundle](../evidence/case-keymap-current/keymap-layered-public/current-qualification/RESULTS.md). The public result is bounded: the Dioxus Keymap-local source export succeeds, while the Dioxus Export workspace still lacks the reference ZMK row; an unavailable-firmware case has not yet been qualified.
