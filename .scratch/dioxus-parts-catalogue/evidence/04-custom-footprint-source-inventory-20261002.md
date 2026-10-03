# Draft04 custom-footprint authoring — capability and source inventory

This planning packet refines one local child under the existing F4.2 parent. It does not change any of the 62 canonical parent rows, their edges, F4.2's INT.2 acceptance join, shared shell/runtime ownership, or the parent status. No source or build target was changed for this inventory.

## Proven start capabilities in the integration source

The current integration source at `2db850b205971a9ad66996933896a4abf46be4a3` contains both feature entry points needed to start this child:

- `web/src/parts_new_component.rs` (SHA-256 `86e6c83b013c9e00757a04fe6a608f2a92c28ab8530e6c606b003aa5f4a6a182`) defines `NewCustomComponentAction` outside the catalogue loading/error branch. Its capture includes the accepted session epoch, document identity, snapshot token/revision, current Parts scope, view/scope generations and generated definition ID. `prepare_create_edit` appends one default empty custom definition to the current accepted project through `Event::Edit` / `EditOperation::ReplaceDocument`; the owner selects it after successful operation settlement only while that captured owner remains current. The module's production tests cover creation, accepted session history, stale capture and identity collision behavior.
- `web/src/parts_definition_name.rs` (SHA-256 `7a1b83d1e22a331cb79ac65354a4c50d828ea39e3464285797af3a6fdb2f3355`) supplies the contextual project-definition Name edit. It admits only a still-selected definition in the current scope and accepted snapshot, resolves it from the current document, and submits one normal accepted edit. Its current tests exercise accepted/revision-preserving edits, stale selection/scope/document rejection, dirty-draft refresh after an unrelated accepted edit, Undo/Redo, simulated durability completion and serialized save/reopen. The full live-browser acceptance for this field remains separate.
- `web/src/presentation/parts.rs` (SHA-256 `4bea702b5bd4d42c896fe15123643849981c035976f1b1e5a448321a5625b81a7f`) mounts the create action in Parts actions and uses the accepted project definition plus selected scope for Inspector details/editing.

These are sufficient capability-level start proof for the remaining native-definition editor. A full catalogue/module-source success gate, F4.1 acceptance, or new host save callback is not needed to start. Keep F4.1's canonical parent edge and F4.2's INT.2 join intact; this child does not close either parent.

## React source map and behavior

The compared React source is pinned at `5a472a9426e6e38993361da402cd4ec730feb369`; the relevant source files are identical at that reference and this packet's repository snapshot.

| Behavior | React source | Contract to preserve |
|---|---|---|
| Create action and selection | `app/src/ui/PartsLibrary.tsx:12-44`; `app/src/ui/Workbench.tsx:759-773,1283` | New custom component is a Parts catalogue action. It materializes the default definition immediately, clears search, selects the new project definition, and reveals the inspector. |
| Contextual editor placement | `app/src/ui/PartsInspectorPanel.tsx:62-110`; mounted from `Workbench.tsx:1038-1047` | Definition editor is in the right Parts Inspector, under Edit footprint / Custom geometry, for a selected non-generator project definition. KiCad pad fields are read-only; their import/source ownership stays separate. |
| Name, local-draft boundaries | `PartsInspectorPanel.tsx:73-81`; `InspectorControls.tsx:1-18` | Name is a raw text draft committed on blur; Enter blurs, Escape restores. Kind select commits immediately. Preserve the Name field's raw-string behavior; the definition issue text is not a new commit rejection. |
| Courtyard and pad actions | `PartsInspectorPanel.tsx:82-108`; `createLibraryActions.ts:32-134` | Width/height edit a centered rectangle; pad add/remove and shape commit immediately; text/numbers use DraftInput and its blur/Enter/Escape behavior. The visible shapes are circle, oval, rectangle and rounded rectangle. No pad rotation control exists. Add pad uses `pads.length + 1` as its default number. |
| Validation | `createLibraryActions.ts:46-127`; `workbenchGeometry.ts:15-37` | Positive finite courtyard size, finite pad position, positive finite pad size/drill, nonempty unique pad IDs and nonempty unique authored pad numbers. Rejected courtyard/pad commits preserve the old accepted field and show recoverable feedback. `definitionIssues` is advisory for a blank Name; Name remains raw-string committed. |
| Net identity/removal | `createLibraryActions.ts:88-131` | Pad ID rename remaps old IDs to new IDs on every placed instance of that definition. Pad removal drops only those pins. Pad numbers are not net identity. Preserve other nets/parts/document data. |
| Accepted edit, history, durability | `Workbench.tsx:575-585`; `createProjectActions.ts:40-85` | Each accepted action goes through one ordinary commit edit; Core/Session accept it, create history, and proceed through normal persistence. Undo/Redo and normal saved-project reopen remain required evidence. |

### Source discrepancy requiring independent disposition

The parent draft requires unique nonempty authored pad numbers. React's manual Number edit enforces that rule in `createLibraryActions.updatePad`, but Add pad directly commits a number of `pads.length + 1`; after pads have been renumbered or removed, that default can duplicate an existing number. `definitionIssues` reports duplicate IDs but not duplicate numbers. Keep this case visible in the Spec/Standards review packet. Resolve the desired behavior before coding; do not silently adopt a new-number allocator or preserve the duplicate as an accepted value without review.

Relevant React test oracles include `app/e2e/library-workflow.spec.ts`, `app/e2e/parts-catalog.spec.ts`, `app/e2e/savedProjectState.ts`, and existing edit/history coverage in `app/src/createProjectActions.test.ts` and `app/src/ui/workbenchEditActions.test.ts`. Current UI tests do not replace the required feature-level paired public journey.

## Remaining implementation and acceptance gap

The existing Dioxus feature creates an empty custom definition and edits Name, but does not yet expose kind, courtyard dimensions, pad add/remove or the pad fields. Implement those remaining controls as a private Parts feature on the accepted document/selection contract. Reuse the existing field-admission behavior; do not submit a stale whole-document copy when unrelated accepted data changed. Validate stale scope/selection/project at the synchronous command boundary and keep each reference commit boundary as one normal edit/history unit.

Before acceptance, compare the exact field results in pinned React and Dioxus, prove invalid-input recovery and per-action revision/history, and use a separate project fixture with multiple placed instances and net pins to prove pad-ID remap and pad removal while preserving unrelated data. The durable check must observe the accepted revision saved through the ordinary session path and verify the reopened value/artifact; the Name test's simulated persistence/JSON round-trip alone is not browser durability acceptance. Include desktop/compact, light/dark, keyboard/focus, page-error, Undo/Redo and actual save/reopen evidence.

## RF handoff

No new refactoring takeaway was observed from this source inventory. Keep existing RF-001 (shared UI/Runtime composition hotspot), RF-002 (private feature boundary), RF-006 (accepted project versus local draft/scope), and RF-009 (paired parity and evidence accounting) only where the implementation/review actually confirms those observations. Do not update the shared RF register or claim a resolution from this planning packet.

## Provenance

- Existing parent F4.2 issue SHA-256: `f686d3bb4b105596235d3174bb162d876b3f999edbe227b86670dcb5e86ff3e1`.
- Existing contextual-workflows specification SHA-256: `f739b30e6e4fb5aa743ccfd0be3b069a81abd701d96297857a0251bfa48f9884`.
- Draft04 body before this refinement: `evidence/04-custom-footprint-authoring-before-20261002.md`, SHA-256 `f9fc68e085d8a4fe026b25d36f5497702e79579d61924cba83e8578300031da3`.
