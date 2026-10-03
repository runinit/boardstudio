## Problem Statement

**Triage:** ready-for-agent. Implementation/acceptance dispatch follows the [slice graph](../tasks.json) and [agent execution plan](../EXECUTION.md).

A designer can open the Dioxus workbench and reach the Parts tab, but it is still an explicit unavailable-workspace placeholder. The pinned React workflow lets the designer search real footprint definitions, choose key assemblies and VIK modules, inspect 2D/3D previews, adjust supported generator parameters, create or import component definitions, edit assembly snapshots, and use existing save, placement, undo and export behavior. Without those workflows, designers cannot continue a board from Layout into part selection and assembly work in Dioxus.

The port must preserve actual library identities, source assets and limits. Generator recipes, KiCad parsing, document validation, history, module resolution, CAD conversion and rendering remain existing service responsibilities. A visible catalog that cannot produce a preview or a normal undoable edit does not complete F4.

## Solution

Port the Parts library as a sequence of small, user-visible Dioxus slices. Start with browsing, search and selection; add editable and imported footprints; restore supported generator forms and their previews; restore the standalone footprint/model preview; then finish saved key assembly authoring and source-backed module profile review. Each action goes through the current document edit path or an existing Rust artifact, module, CAD or renderer service.

Reuse the F3a SVG projection and the unchanged retained Ergogen generator service. Keep previews view-only; keep generator edits as drafts until Apply. Save component and assembly changes through the accepted edit/history path. Part placement into a board and matrix uses the already agreed F3 placement seam. Keep module-library definition/profile work in F4, while mounted-module placement, PCB artwork/layers, wiring, connectors and electrical readiness remain F5 work; case-facing physical fit remains F7 work.

The implementation has one real integration prerequisite: feature UI cannot currently call the host's private worker for artifact operations, import model bytes through the accepted project asset path, or request an isolated library sample preview through the current Dioxus runtime. The coordinator owns those shared Runtime/shell changes and should expose only the typed feature operations needed by F4. Use existing request/reply types and current asset ownership; do not add or widen a public API, document field, format, generator implementation or backend algorithm. If an existing service cannot provide a required public behavior, stop that item at the adapter boundary and record the smallest required decision rather than simulating or silently removing the behavior.

## User Stories

1. As a keyboard designer, I want to open Parts and see the parts catalog, so that the workspace is useful instead of an unavailable placeholder.
2. As a keyboard designer, I want searchable categories for switches, controllers, connectors, encoders, passives and LEDs, utilities, custom parts, key assemblies and VIK modules, so that I can find the actual item I need.
3. As a keyboard designer, I want search to match part names, supported generator terms, category, module family, variant and assembly name, so that I can locate a part without knowing its exact catalog label.
4. As a keyboard designer, I want the catalog to show the same preferred entries and legacy/custom saved assignments as the React reference, so that new choices stay clear while existing projects remain intact.
5. As a keyboard designer, I want to select a library row with pointer or keyboard and see its selected state and details, so that browsing does not depend on mouse-only interaction.
6. As a keyboard designer, I want loading, empty, no-match and source-load error messages in the catalog, so that a missing module catalogue is never mistaken for an empty catalogue.
7. As a keyboard designer, I want the selected footprint's pads, drills, graphics, text, courtyard or keycap envelope and dimensions in a 2D preview, so that I can check what the library entry contains before using it.
8. As a keyboard designer, I want named footprint layers and individual preview-part visibility controls, so that dense previews remain inspectable without changing the source definition.
9. As a keyboard designer, I want the preview to use existing F3a geometry projection and the retained generator for generated graphics, so that Layout and Parts do not show different generator output.
10. As a keyboard designer, I want a part's existing keycap dimensions and component companions reflected in the sample, so that the preview represents the selected definition and assembly recipe.
11. As a keyboard designer, I want to switch between 2D footprint and 3D model views, so that I can inspect both PCB geometry and attached model assets.
12. As a keyboard designer, I want loading, missing-asset, failed-model and retry states to be explicit, so that a 3D preview failure does not look like a valid empty assembly.
13. As a keyboard designer, I want preview construction and view toggles to leave the active project, revision, nets and history unchanged, so that inspection cannot accidentally edit my keyboard.
14. As a keyboard designer, I want to define or edit a part mechanical fit profile with named purposes, measured dimensions, source evidence, clearances and qualification state, so that layouts and cases can share a grounded part-fit description.
15. As a keyboard designer, I want profile extraction to map source geometry to supported mechanical purposes and report failures locally, so that an imported profile is not mistaken for measured fit evidence.
16. As a keyboard designer, I want profile saves to preserve unresolved gates and only clear a blocker when all required geometry is explicitly reviewed, so that a preview or partial profile cannot claim mechanical readiness.
17. As a keyboard designer, I want to edit supported generator parameters in named groups for footprint options, keycap dimensions, connections, model placement and advanced options, so that routine controls are easy to find and specialized inputs stay available.
18. As a keyboard designer, I want booleans, finite numbers, strings, side selection, structured values and model-file controls to retain their supported semantics, so that values are not silently coerced or replaced by defaults.
19. As a keyboard designer, I want generator changes to remain drafts until Apply, with stale previews discarded when the definition or parameters change, so that a result for an older form state cannot overwrite the current one.
20. As a keyboard designer, I want malformed parameters, unsupported files, generator failures and compile failures to appear beside the relevant form or preview with a recovery path, so that I can correct the cause without losing the workbench.
21. As a keyboard designer, I want Apply to save a valid generator change as one normal undoable edit, including the required terminal/net remap behavior for placed instances, so that I can reverse or reopen the change safely.
22. As a keyboard designer, I want to create a custom component and edit its kind, name, courtyard, pads, pad shape, dimensions, drill and numbers, so that I can describe a local part without inventing generator support.
23. As a keyboard designer, I want duplicate IDs, duplicate electrical pad numbers, invalid dimensions and non-finite coordinates to be reported before commit, so that malformed definitions do not enter the project.
24. As a keyboard designer, I want edits to a placed definition to affect its instances through the existing shared definition and document semantics, so that all instances remain consistent.
25. As a keyboard designer, I want to import a supported KiCad footprint and keep the parsed source, pads, drills, graphics, references, layers, units and attribution intact, so that the imported definition remains portable and faithful to its source.
26. As a keyboard designer, I want imported source-owned pad geometry to remain read-only in the definition editor while its supported envelope, net bindings and model attachments remain editable, so that edits do not desynchronize parsed geometry from its KiCad source.
27. As a keyboard designer, I want to attach a supported STEP, STL or WRL model to a definition or generator parameter and set its offset, rotation and positive scale, so that the model can be aligned to its footprint.
28. As a keyboard designer, I want unsupported, unreadable or stale model imports to fail with a local error and preserve the current definition and project assets, so that an interrupted import cannot partially attach a model.
29. As a keyboard designer, I want to create a saved assembly with named members, selected component definitions, front/back side, pose, parameter overrides and default/custom model bindings, so that a multi-part key assembly can be reviewed and reused.
30. As a keyboard designer, I want to import, add, remove and edit assembly models with finite transforms and positive scale, so that the preview matches the intended member configuration.
31. As a keyboard designer, I want to see an isolated assembly preview while editing, so that changing member composition or pose provides immediate feedback without placing parts into the current board.
32. As a keyboard designer, I want Save assembly to validate required name and members, retain definition snapshots and preserve unrelated project data, so that saved assemblies are stable and undoable.
33. As a keyboard designer, I want to apply an assembly to a selected matrix or place it on the selected board only when the relevant F3 placement target exists, so that F4 does not bypass placement scope or edit transactions.
34. As a keyboard designer, I want later changes to an assembly recipe to leave already placed snapshots unchanged, so that existing hardware does not move when I edit a reusable recipe.
35. As a keyboard designer, I want module catalogue entries grouped by their reviewed row with selectable variants, source links, licence and readiness information, so that I can compare source-backed options without confusing a preview with fabrication approval.
36. As a keyboard designer, I want to inspect the source module board and its included component outlines and source mounting holes, so that I can understand what the module snapshot contains before using it.
37. As a keyboard designer, I want to edit module definition profiles for measured volumes, openings, model bindings and supported rotary facts, with explicit evidence and qualification gates, so that unknown geometry remains visibly unqualified.
38. As a keyboard designer, I want module profile changes and asynchronous mechanical extraction to reject stale responses after a selection change or unmount, so that another module cannot receive an old result.
39. As a keyboard designer, I want module source/profile editing to leave mounted instance pose, board wiring and PCB layers to their owning workspaces, so that one editor does not obscure ownership or readiness boundaries.
40. As a keyboard designer, I want keyboard focus, semantic list selections, field labels, error announcements, Escape handling and compact layout to match the established workbench, so that the Parts workflow remains accessible across supported sizes.
41. As a maintainer, I want each F4 slice to identify its React source responsibility, Dioxus owner and visible acceptance trace, so that a partial component port is not counted as a complete workspace.
42. As a maintainer, I want generator execution, footprint import/compile, project asset storage and 3D preview calls to stay behind explicit existing service boundaries, so that Dioxus does not acquire duplicate business logic or a second writable document.
43. As a maintainer, I want no public Rust member visibility, service schema, project format or generator algorithm changed as part of this plan, so that F4 remains a behavior-preserving frontend port.

## Implementation Decisions

- Dioxus owns the catalog query, selection, form drafts, local open/closed state and transient preview-view state. These are presentation state and are not saved as document fields.
- Existing Rust document/session edit operations remain authoritative for committed definition and assembly changes, validation, history and project revision. F4 must not mutate a second writable project copy or write model files outside the existing asset path.
- Existing artifact request/reply services handle KiCad footprint import and authored/imported footprint compilation. The retained Ergogen service remains the generator authority for supported generator parameter metadata and generated artwork; F3a's expression projection remains the shared presentation geometry path.
- Footprint and model file imports keep current accepted extensions, asset identity/bytes, source references and error behavior. A file input clearing after selection is part of the interaction; incomplete or stale async completion must not update a changed project or selected definition.
- Generator forms preserve JSON types, false, zero, empty and absent values as the React workflow does. Apply is a single committed transaction; preview is view-only and never creates a revision or Undo item.
- Assembly drafts use current document assembly definitions and existing edit operations. Save preserves other project data. Board/matrix placement uses the established F3 public placement actions and never changes an existing placement when only a saved assembly recipe changes.
- Part and module preview are isolated, read-only views. They reuse existing geometry, model conversion, CAD, renderer and asset services. They do not imply mechanical fit or fabrication readiness beyond their existing evidence.
- F4 work is limited to library-level module source/profile interaction. PCB-mounted module placement, host connector assignment, copied module circuits, layer ownership, electrical readiness and finding navigation stay with F5; case-contact and physical-instance review stay with F7.
- Shared shell composition, project/file lifecycle, the Dioxus Runtime, worker clients, asset bridge and common preview lifecycle remain coordinator-owned. F4 feature owners use the agreed public UI/component seam and request only the smallest typed operations missing from that seam.
- Preserve the existing Dioxus 0.7.10 toolchain and the incumbent light/dark/system design tokens and responsive conventions.

## Testing Decisions

- Every user-visible acceptance is a public Dioxus browser flow against the same reference fixtures and service outcomes, not a test of Rust component internals. For each slice retain one failing pre-port UI trace or assertion and the passing replacement trace, including the empty, loading, error and recovery states that the slice owns.
- Run the existing core/artifact and application edit/history tests unchanged for imported/authored footprint validation, definition identity, mechanical profile limits, module definition edits and undo behavior. New backend tests are out of scope unless investigation proves an existing service contract fails; a missing Dioxus adapter is not a reason to rewrite or broaden that service.
- Transfer catalog regression cases for canonical and legacy choices, custom entries with duplicate names, retired definitions, module variants and search/empty states to public UI coverage.
- Transfer generator/preview cases for false/zero/empty values, invalid structured JSON, active generator results, Y-up coordinates, footprint layers/text/drills and per-definition preview isolation. Verify actual generated definitions against the same retained generator inputs; do not treat a static mock or authored-footprint preview as generator coverage.
- Transfer import/edit cases for valid and malformed KiCad source, source-owned geometry, identity/asset preservation, model extension checks, stale project/selection changes, saved-project round-trip, and Undo/Redo through the normal UI.
- Transfer assembly cases for valid/invalid save, duplicate IDs, definition/model selection, side/pose/parameter overrides, model transforms, import failure, isolated preview, matrix apply, board placement, and unchanged prior placement snapshots.
- Cover part and module fit profiles with licensed fixture data, readiness gates, async extraction error/stale-result behavior and project round-trip. Cover module source/variant preview with the bundled reviewed fixtures. Keep mounted module, wiring, PCB artwork and case acceptance for F5/F7 rather than asserting them in F4.
- Compare paired React and Dioxus desktop/compact captures for light and dark themes, including selected, open disclosure, generator pending/error, model loading/error and assembly editor states. Include keyboard operation, focus restoration, axe checks and relevant manual screen-reader checks; existing host limitations remain explicit.
- The current React prior art is `partsCatalog.test.tsx`, `libraryDisplay.test.tsx`, `Ergogen2DPreview.test.tsx`, `libraryPreview.test.ts`, `generatorSettings.test.ts`, `sampleAssembly.test.ts`, and `assemblyEditorController.test.ts`, plus `library-workflow.spec.ts`, `library-preview.spec.ts`, `library-display.spec.ts`, `parts-catalog.spec.ts`, and `ergogen-library.spec.ts`. Keep these as the behavioral oracle until their public Dioxus equivalents are accepted.

## Out of Scope

Backend generator implementation or Rust generator parity; KiCad parser changes; core or application API/type/schema changes; new project formats; widening private/public Rust visibility; edits to generator recipes or catalog contents; new part variants or unsupported parameter families; replacing imported source geometry; changing project asset storage or persistence semantics; automatic library-model alignment claims; mounted module placement and VIK wiring/controller assignment; PCB module layer controls; case and physical-instance readiness; keymap/keycap behavior; Export changes; shared shell/runtime work not assigned to the coordinator; production entrypoint switch; React removal; backend performance or CAD rewrites.

F3 matrix placement and F2 shared work are prerequisites rather than scope to duplicate. A missing adapter or service capability is recorded as a concrete gap and remains open; it does not become a permanent disabled control or placeholder.

## Further Notes

### Execution order and ownership

The task graph is in `/tmp/boardstudio-workflow-plans/F4.json`. Suggested slices:

| Slice | Visible result | Prerequisites | Feature-private ownership | Existing boundary / gap |
| --- | --- | --- | --- | --- |
| F4.1 | Searchable library and selected definition/module/assembly | INT.1 private Parts host; F2.3 at final qualification | PartsLibrary, library query/selection, catalog row/detail composition | Data catalogs and search rules exist; Dioxus library surface is a placeholder. |
| F4.2 | Custom footprint create/edit and KiCad import | F4.1; coordinator artifact/asset calls | Definition editor, validation presentation, import status and part-specific form actions | Rust ImportFootprint and compile/import types exist. Runtime has no typed feature facade; coordinator owns it. |
| F4.3 | Generator forms, retained generator preview and Apply | F4.1 and unchanged F3a generator; INT.2 at acceptance | Generator fields, parameter draft/status and selected-definition 2D preview composition | Ergogen remains JS service; Rust authored compiler explicitly rejects generator-backed definitions. Reuse F3a; do not add a Rust generator. |
| F4.4 | 2D/3D library sample preview with error/retry | F4.1/F7.1 start; F7.3 common viewer and INT.2 join; F3a reused | Preview selection, layer state and isolated sample scene request state; F7 owns common model/viewer lifecycle | Existing core/CAD/renderer pieces exist, but there is no Dioxus library-sample preview owner or Runtime call path. This is the largest adapter/lifecycle gap. |
| F4.5 | Edit a reusable part mechanical-fit profile | F4.1; coordinator profile/extraction calls | PartMechanicalProfileEditor draft, evidence, mapping and qualification presentation | Mechanical profile and source extraction services exist; no typed Runtime façade currently exposes them. |
| F4.6 | Saved key assembly editor and preview | F4.1, F4.4; F3 matrix/board placement seam before Place/Apply controls | Assembly editor draft/forms, member/model controls, save/apply/place UI | Assembly document types and generic edit path exist; React sample/placement/controller helpers are UI logic that must be ported without changing the contract. |
| F4.7 | VIK/module source profile review in Parts | F4.1, F4.4; F5 for mounted use; F7 for physical-fit acceptance | Module source preview, library-level profile editor, variant/readiness presentation | Module catalogue and import/profile operations exist. Keep mounted module controls, wiring and board overlays in F5. |

#### Bounded module-profile editor refinement

The library-level module inspector's manual profile editor edits measured volumes,
functional openings, candidate model bindings/transforms, and the supported rotary
profile through the ordinary accepted `SetModuleDefinition` edit. Keep the draft
scoped to the selected module/session/scope and rebase only those profile fields on
the latest accepted definition; unrelated accepted metadata must survive. Existing
project-owned definition deletion or a conflicting profile edit rejects. This
surface records manual evidence and qualification only: asynchronous source
extraction, mounted-module wiring/placement and Case assembly acceptance remain
separate joins. A mounted editor does not close F4.7 or the parent F4 workflow.

F4.2 custom definitions do not require full F3 for editing/import/round-trip. F4.3 generator and F4.4 preview are independent of matrix authoring after INT.1 provides their private host contract. F4.5 part-fit profile UI can be built as a library action while Case remains downstream. F4.6 save can be delivered before placement if Place/Apply is held for its actual F3 dependency; do not mark F4 complete on that partial state. F4.7 source/profile UI may be built before F5, but its mounted-instance behavior is explicitly downstream.

### Source and file map

Pinned React reference is commit `5a472a9426e6e38993361da402cd4ec730feb369`; current clean Rust/Dioxus planning source is `c827c4e69389a77b1f0e8d647ce86a4c41529611`, executable F3a source `f44a3d1b`.

The F4 production source inventory is eleven responsibilities. Core library UI: `app/src/ui/PartsLibrary.tsx`, `LibraryWorkspace.tsx`, `PartsInspectorPanel.tsx`, `GeneratorFields.tsx`, `InputProfileEditor.tsx`, `usePartsEditing.ts`, and `createLibraryActions.ts`. Preview and geometry: `Ergogen2DPreview.tsx`, `ModelPreviewBoundary.tsx`, `sampleAssembly.ts`, `libraryPreview.ts`, `libraryPreviewGeometry.ts`. Assembly editor: `AssemblyEditor.tsx`, `assemblyEditorController.ts`, `assemblyPlacement.ts`, `assemblyCatalog.ts`, and `assemblyPresets.ts`. Module library source/profile: `ModulePreview.tsx`, `ModuleInspector.tsx`, `ModuleProfileEditor.tsx`, `PartMechanicalProfileEditor.tsx`, `app/src/modules/catalogue.ts`, bundled module snapshots/manifests and source/asset attribution. Styles are in `library-workspace.css` and `module-workspace.css`, with shared field and inspector patterns owned by F2/common UI.

The exact ticket ledger names 11 TSX source owners: `AssemblyEditor.tsx`, `Ergogen2DPreview.tsx`, `GeneratorFields.tsx`, `InputProfileEditor.tsx`, `LibraryWorkspace.tsx`, `ModuleInspector.tsx`, `ModulePreview.tsx`, `ModuleProfileEditor.tsx`, `PartMechanicalProfileEditor.tsx`, `PartsInspectorPanel.tsx`, and `PartsLibrary.tsx`. `Ergogen2DPreview.tsx` is already only a partial F3a port: Dioxus owns the Layout SVG and projection, but Parts/PCB/3D preview composition is not accepted. Shared `Workbench.tsx`, `main.tsx`, `Runtime`, worker clients and the common renderer are not F4 feature-owned files; coordinator-only shared edits stay outside feature-private ownership.

### Rust/service facts and adapter gaps

- `CoreRequest`/`CoreReply` already expose document-scoped module resolution, keycap resolution, mechanical resolution, and `Open`/`Edit`; `EditOperation` includes module-definition and mounted-module operations. `ProjectDoc` already stores assemblies and modules. No schema extension is needed for the listed UI.
- `ArtifactRequest`/`ArtifactReply` already include footprint import, module-board import, footprint compilation and mechanical extraction. The Rust artifact worker handler implements these. `CoreWorker` already has a public artifact request call, but the shared Dioxus `Runtime` keeps the worker private and currently exposes no typed F4 artifact/model-file operation. The UI needs a coordinator-owned typed bridge, not a new core endpoint.
- Rust authored-footprint compilation supports authored or KiCad-source geometry and intentionally rejects generator-backed definitions. The retained Ergogen service remains necessary to expose generator parameters and evaluate generator recipes. F3a packaged `layout-generators.js` and private `footprint_forms`/`footprint_graphics` already establish the boundary and SVG projection; extend/reuse this path instead of forking geometry interpretation.
- `RendererHost`, current CAD worker request/result types and Core model/module resolution exist. React `useAssemblyPreview` additionally owns isolated sample document resolution, separate PCB/keycap request sequences, cancellation/stale guards, an 80-entry model-conversion cache, and disposal. No equivalent standalone Dioxus Parts-preview owner or Runtime facade is present. Reuse the current worker/renderer types with explicit scope, drop, stale-result and cleanup behavior. Keep any new Dioxus orchestration private to the web presentation; coordinator owns changes to shared Runtime composition.
- React's assembly save and placement helpers create saved definition snapshots and update an assembly draft; the `AssemblyDefinition` model and normal document edit path already exist. The task must preserve member IDs, model modes/defaults, source terminal nets, prior placed snapshots and matrix identities. Route placement through existing F3 public UI actions; do not change Rust contracts to reproduce a React component API.
- The VIK module catalogue and source/asset manifests are already bundled and the artifact service can import a module board. Library-level source/profile inspection is F4. `ModuleInspector` also contains mounted placement, host connector, bus/terminal assignment, embedded-circuit copy and PCB instance work; that behavior belongs to F5 even though the current source file spans both milestones. F7 remains responsible for mechanical assembly-view acceptance.

### Remaining explicit boundaries

- The shared Runtime/worker façade, file-to-asset path and isolated preview integration need coordinator assignment before feature slices can reach their service operations. The feature plan names no public API/schema change.
- Full F3 is still open. Until its placement actions are available, catalog, import, edit, generator and preview slices can be accepted independently, but F4's whole workflow cannot close its Place/Apply acceptance.
- Actual host screen-reader verification is separately carried from F1/F3a. F4 can still preserve semantics, keyboard and focus behavior and report the host AT limit accurately.
- No test suite, build, browser session or worktree change was run for this planning-only task.

### Independent review reconciliation

The exact start/acceptance table supersedes the earlier coarse dependencies: catalogue, generator-form and isolated-preview work do not wait for all of F2, custom-footprint editing, or all of F3. Existing catalogue fixtures and the retained F3a generator are usable inputs. F4.4 owns isolated sample-project inputs and pending/error selection state; F7.3 owns the single shared 3D viewer, camera/picking/model/render lifecycle. F4 never implements a second renderer. Generator-edited-preview composition is checked when both F4.3/F4.4 land, not used as a blocker on their independent implementation. F4.6 save/editor work proceeds on fixtures; Apply/Place joins F3.2.


### Authoritative execution dependencies

The milestone-level prerequisites above describe integration context. The refined rows below replace whole-milestone or symbolic dependencies. Preparation/fixture work may start after `Start after`; completion also requires `Acceptance joins`. Existing F1/F3a source and evidence are baseline prerequisites, not tasks to repeat. Full workflow qualification also joins F2 shared panels/controls under F9.

| Slice | Start after | Acceptance joins |
| --- | --- | --- |
| F4.1 | INT.1 | Own slice acceptance |
| F4.2 | F4.1 | INT.2 |
| F4.3 | F4.1 | INT.2 |
| F4.4 | F4.1, F7.1 | F7.3, INT.2 |
| F4.5 | F4.1 | INT.2 |
| F4.6 | F4.1 | F4.4, F3.2 |
| F4.7 | F4.1 | INT.2, F4.4 |

#### F4.6 source refinement — saved assembly authoring

The pinned React `PartsInspectorPanel` exposes a saved-assembly list with New,
select and Duplicate actions, and `AssemblyEditor` owns a local draft with name,
member definition, front/back side and XY/rotation controls before Save. The
Dioxus Parts surface had only the eight preset selectors and no saved-assembly
list or editor. The first mounted authoring packet restores that path through
the existing accepted-document edit/history boundary: a draft can be created,
edited, duplicated and saved; only its assembly record and any referenced
catalogue definition snapshots are applied to the latest accepted document.
An intervening edit to the same saved assembly is reported instead of being
overwritten, while unrelated accepted document changes are retained.

The follow-up source packet adds the pinned editor's current generator
parameter affordance (hotswap/side), member model-default/custom selection,
model asset selection and finite offset/rotation/positive-scale edits, model
import through the existing verified asset path, visual-only model members,
and an isolated recipe preview in the retained Parts preview owner. Save merges
draft assets and referenced definitions through the same accepted document
edit/history boundary. The “Edit model defaults” action calls the retained
Ergogen `modelBindings` provider with the member's definition and overrides,
preserving generated unresolved-model identifiers instead of inferring model
bindings from UI state.

The mounted placement follow-up routes Apply-to-selected-matrix through the
existing `SetMatrix` accepted edit and Place-in-Layout through the F3.2
cursor-controlled matrix placement owner. Both carry operation-scoped
definition snapshots, preserve the accepted Session/Core edit boundary, and
reject stale scope/source/selection state. The Dioxus placement control does
not yet reproduce React's independent “Place on selected board” X/Y-origin
form, which commits a set of component parts; it currently offers the existing
Layout matrix placement workflow, so criterion 33 remains open for an exact
paired acceptance/evidence trace. The full F4.6 acceptance/evidence table above
is unchanged. Do not mark the parent criterion complete from these bounded
authoring packets.

### Refactoring observation handoff

Update the [living RF register](../refactor-findings.json) and [post-port takeaways](../../../docs/migration/POST-PORT-REFACTOR.md) for architectural, design, theoretical or quality issues discovered in this slice, or record “No new refactoring takeaway observed” with reviewed scope. Distinguish confirmed findings from hypotheses; include evidence, impact, current mitigation, later proposal and validation. This does not authorize unrelated refactoring or defer required parity fixes.
