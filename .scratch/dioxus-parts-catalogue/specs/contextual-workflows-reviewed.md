# F4 contextual Parts authoring and source workflows

**Status:** draft for independent Spec/Standards review; child tickets are not yet dispatch-ready.

**Existing parents:** F4.2 custom/imported footprint definitions; F4.3 retained-generator settings; F4.5 mechanical-fit profiles; F4.6 reusable key assemblies; F4.7 VIK module profiles. This refinement leaves the canonical 62-parent graph, each parent criterion, start edge and acceptance join unchanged.

## Problem Statement

The Dioxus Parts workspace now has a browsable catalogue, a selected-definition summary and a read-only center preview, but the React Parts experience uses contextual authoring surfaces whose placement depends on the selected workflow. The right Inspector exposes placement eligibility, hardware readiness, supported generator options, input-profile controls, keycap/outline controls, a custom-definition editor and model alignment. The part mechanical-fit editor replaces the center workspace preview. Selecting an assembly or module exposes different variant, source, member and profile actions. Dioxus cannot yet continue most of those workflows from the catalogue.

The reference implementation is distributed across a large `Workbench.tsx`, `PartsInspectorPanel.tsx`, `LibraryWorkspace.tsx`, generator/editor components and action controllers. Porting only the visible fields without identifying their owner, accepted-document input, edit transaction, async lifetime and browser evidence risks reproducing a generic Inspector that loses contextual behavior.

## Solution

Continue using the existing Objects/catalogue, center preview and Inspector composition. Add bounded contextual slices for native definition editing, KiCad import, model attachment, retained-generator editing, center-workspace mechanical-fit profiles, assembly authoring, and VIK module profiles. Each slice consumes a reviewed private Runtime capability and the existing domain/Core services. Draft state remains local until the reference commit boundary; durable changes use the normal accepted edit path and history. Asynchronous imports/extractions are scoped to the accepted project/selection and cannot apply to a changed or unmounted selection.

The Project menu follows the same ownership rule: implement its independent menu composition, project rename interaction and demo choices separately from the portable-copy action. The latter remains gated on F2.2 capability child 01 review and the shared `embedUsedModels` preference join. Its UI action must not be mounted against an unreviewed pack provider.

## User Stories

1. As a designer, I want selected native definitions to show the same contextual actions and editable fields as React, so that I can continue from browsing into component authoring.
2. As a designer, I want to create a valid custom component with a useful name, kind, courtyard and pads, so that a local part can be described without generator support.
3. As a designer, I want every accepted field commit to preserve unrelated definitions, assets, terminals, nets and placements, so that editing one property does not rewrite other project state.
4. As a designer, I want imported KiCad source geometry to remain source-owned while supported envelope, net and model-binding fields stay editable, so that a project does not claim to edit geometry that remains linked to KiCad source.
5. As a designer, I want import errors and stale async completions to leave the accepted project unchanged, so that a failed import cannot partially attach a component.
6. As a designer, I want supported STEP/STL/WRL assets and transforms to be attached through the existing asset flow, so that I can align physical models without losing exact source bytes.
7. As a designer, I want generator values to retain their typed semantics and remain drafts until Apply, so that zero, false, empty strings and structured options are not silently replaced.
8. As a designer, I want a current successful generator preview before Apply, so that a failed or stale preview cannot commit the wrong footprint.
9. As a designer, I want the selected part's center workspace to open its fit profile editor and show source evidence, so that Layout and Case consume the same measured facts without relocating the React control.
10. As a designer, I want an assembly recipe editor to change members and transforms without moving already placed assembly snapshots, so that reusable definitions remain distinct from existing placements.
11. As a designer, I want VIK module variant, provenance and qualification fields to be visible and editable where supported, so that library evidence is not confused with mounted PCB behavior or fabrication readiness.
12. As a designer, I want project rename and demo actions in the Project menu to match the reference, so that the Dioxus shell remains familiar while each action keeps its own lifecycle owner.
13. As a designer, I want Save project copy to share the same archive options and used-model closure as Export, so that both entry points produce equally portable files. This story remains blocked until F2.2 issue 01 and the shared preference contract are independently reviewed.

## Implementation Decisions

- Preserve TypeScript control placement, labels, contextual pane ownership, focus/commit behavior and exact source semantics, except for a confirmed defect or documented platform constraint.
- Keep catalogue selection/query state with the Parts owner; keep local editor drafts with the relevant feature editor; keep durable ProjectDoc, history, browser assets and async identities under Runtime/Session.
- For authored definition edits, introduce the smallest private Parts-owned intent/admission path at the feature boundary: separate stable draft target identity (selection scope, selected definition identity and accepted field value) from the accepted snapshot identity used to admit a commit. A change to target or accepted field value resets the local draft; an unrelated accepted snapshot change refreshes the captured session epoch, document identity, token and revision without discarding the dirty draft. At the synchronous commit boundary, verify the refreshed capture and current selection, then resolve the target from the current accepted document and apply exactly one requested field transformation. Reject a stale/no-longer-selected mismatch before submitting any edit and leave document/history unchanged; preserve each React control's error/reset behavior rather than inventing generic stale-edit UI. Do not trust a cached catalogue definition or stale Inspector draft as the replacement document.
- Preserve the React Name control's exact contextual composition: the Parts right Inspector's `Edit footprint` disclosure, with `Custom geometry` detail and the `Definition name` label. It defaults open when the selected authored definition has no pads, closed otherwise, and a user-chosen disclosure state survives the accepted Name update. Do not place this control in the center preview/workspace. React source `PartsInspectorPanel.tsx:71–73` nests Name inside `InspectorSection`; `InspectorSection.tsx:4–15` owns the title/detail/default-open and user-chosen disclosure behavior, and `Workbench.tsx:1038–1045` mounts `PartsInspectorPanel` as Inspector content for the Parts workbench. The first child narrows eligibility to an existing accepted project-owned, non-generator definition; later native-authoring work retains the broader catalogue edit/upsert path.
- A valid field commit submits one ordinary `Event::Edit`/`EditCommand` commit with a fresh operation/transaction identity, the captured accepted revision, the target definition ID and a `ReplaceDocument` derived from the exact current accepted document. Session/Core remain authoritative for edit acceptance, history, undo/redo, token advancement and persistence. The field adapter stays private to Parts; do not add a public `SetPartDefinition` operation, widen Runtime visibility, or make a new shared Runtime facade. The first tracer slice exercises the Name control for an already project-owned, non-generator definition outside assembly authoring. It does not claim parity for unmaterialized catalogue entries; later native-authoring work preserves React's add-or-replace upsert when edits materialize catalogue definitions into the accepted document.
- Reuse existing `PartDefinition`, `AssemblyDefinition`, module catalogue, footprint import/compile, retained Ergogen generator, model asset, mechanical extraction, module profile and Core edit operations. No new schema, file format, public API/member visibility, generator implementation, renderer or CAD algorithm is authorized.
- Keep KiCad-owned pad geometry read-only. Preserve repeated KiCad pad numbers where the source allows them. Do not expose pad rotation or net remapping where the TypeScript editor does not provide those controls. Preserve the reference per-field commit behavior; do not group distinct fields into an invented transaction.
- Keep the F2.2 archive provider and Project-menu callback owned by their approved source capability. Project menu can prepare presentation and filename/rename affordances, but Save project copy is not wired until issue 01 review clears and the shared `embedUsedModels` option is callable.
- New Project/blank-project creation and setup guide are owned by Project-menu issue 17. This spec does not duplicate that action.
- Feature authors own separate Parts-private component modules and source-backed tests. The Parts-owned Inspector composition may mount those components using its existing accepted-snapshot and selection inputs; shared shell composition, Runtime internals, global styling, build/offline manifests and final parent joins remain coordinator-owned.
- Start children from explicit capability-level proofs where possible. Keep parent full criteria and all acceptance joins (INT.2, F3.2, F4.4, F7.3, F8.6 as applicable) open until their own criteria are verified; never redefine an acceptance join as an implementation blocker without source evidence.

## Testing Decisions

- The acceptance seam is paired public browser behavior using the same React and Dioxus fixture, selected entry, operation, result and changed-document/history observations. A Rust compiler/unit test alone cannot close visible behavior.
- For native definitions and KiCad import, compare selected values, source ownership, exact IDs, repeated pad numbers, assets, per-field committed document state, Undo/Redo and save/reopen. Reject invalid input and changed-selection completions without mutating the prior accepted document.
- For model and generator work, exercise file bytes/type/size, transform validation, false/zero/empty/structured values, current versus stale previews, compile/import failure and recovery. Verify no project mutation before the commit action.
- For mechanical and VIK profiles, compare supported fields, evidence and qualification states with the corresponding React editor; reject stale extraction/load/save completions and verify normal history/reopen.
- For assembly recipes, verify member definitions, sides, poses, parameter overrides, default/custom models, draft preview isolation, save/Undo/Redo and unchanged existing snapshots/matrix identities.
- Compare light/dark desktop and compact layouts, menu/tab placement, keyboard traversal, focus restoration, accessible names/status/error announcements and browser console/network errors. Reuse existing relevant Vitest and Playwright workflows as source oracle, not as Dioxus acceptance.
- Record only evidence-backed refactor takeaways using existing RF identifiers; current planning evidence points to RF-001 (shared presentation/Runtime integration hotspot), RF-006 (active versus isolated sample scope), RF-008 (archive option/asset-resolution split) and RF-009 (parity/evidence accounting). No new RF finding is asserted by this plan.

## Out of Scope

- Completing or closing F4 parent rows, INT.2, F3 placement, F7 shared-viewer, F8 export, or frontend-v1 qualification.
- Adding catalogue definitions or unsupported generator behavior.
- Mounted-module instances, PCB layers, connector routing, wiring or electrical readiness; those remain F5.
- Case-level fit/fabrication claims; those remain F7.
- Replacing the retained Ergogen service, Rust geometry, Core history, asset format, module source authority, renderer host or shared viewer.
- Project-menu portable-copy behavior before its explicit issue 01 review/shared preference gates.

## Further Notes

Reference source inventory: `app/src/ui/Workbench.tsx`, `PartsInspectorPanel.tsx`, `PartsLibrary.tsx`, `LibraryWorkspace.tsx`, `GeneratorFields.tsx`, `createLibraryActions.ts`, `assemblyEditorController.ts`, `AssemblyEditor.tsx`, `PartMechanicalProfileEditor.tsx`, `ModuleInspector.tsx`, `InputProfileEditor.tsx`, `InspectorControls.tsx`, `InspectorSection.tsx`, `assemblyCatalog.ts`, `assemblyPresets.ts`, and `storage.ts`. React renders the Name field only in the right Parts Inspector inside `Edit footprint` / `Custom geometry`, outside assembly authoring when `editDefinition` exists and has no generator (`PartsInspectorPanel.tsx:71–73`). `InspectorSection.tsx:4–15` provides pad-count-dependent default disclosure state and preserves user-chosen state. `Workbench.tsx:1038–1045` mounts that component as Inspector content. `saveDefinition` replaces a matching accepted definition or appends a catalogue definition not yet present (`createLibraryActions.ts:28–32`); preserving that materialize/upsert path remains broader issue 04 work. Current Dioxus anchors include `web/src/presentation/parts.rs`, `parts/catalogue.rs`, `parts/details.rs`, `parts/preview.rs`, `web/src/runtime.rs`, `web/src/presentation/footprint_graphics.rs`, and the existing feature-private modules. The current accepted snapshot, scoped selection, Event::Edit and ReplaceDocument path are the start capability for the first authored-field slice; the field intent/admission adapter itself is owned implementation work in that slice, not a prerequisite that must already exist. Recheck exact source line evidence before implementation dispatch.

No RF ledger update is made in this planning draft. Preserve the existing ledger and add evidence only after an implementation/review handoff.
