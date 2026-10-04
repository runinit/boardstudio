# Parts fit-profile capability receipt — 2026-10-02

This source inventory supports draft children 14–16. It records available service contracts and current Dioxus limits; it does not claim readiness before independent review or a callable adapter.

## React placement and workflows

- `LibraryWorkspace.tsx:93` replaces the center library preview with `PartMechanicalProfileEditor` keyed by definition ID; `LibraryWorkspace.tsx:98` owns the `Define profile` / `Edit profile` action. The existing editor is not an Inspector section.
- `PartMechanicalProfileEditor.tsx:14–25` owns the local profile draft; adding a manual contour copies the fixed square with four vertices at ±2.5 mm (`emptySquare`). Existing draft optional values are spread/preserved by edits. `LibraryWorkspace.tsx:93` keys the editor by definition ID, so changing definitions remounts and discards the prior local draft.
- `PartMechanicalProfileEditor.tsx:26–59` defines standard-family and KiCad extraction actions. `loadFamily` merges the returned profile into current draft, then writes selected family and gap (`:31–37`); it must preserve response-omitted local optional fields. `:60–76` contains manual gap/contour fields, geometry mapping, error, Cancel and Save controls.
- `Workbench.tsx:1384` commits the profile to the selected definition with a ProjectDoc replacement; if the ID is not yet in project definitions, it appends that exact selected catalogue definition as a project override. This is currently a whole-document closure; a Dioxus feature must validate the captured selected entry and submit against the latest accepted document so unrelated intervening edits are preserved.
- `PartMechanicalProfileEditor.tsx:74` invokes `onSave(draft)` and immediately calls `onClose()`. Its `run()` error state covers only the Core/Artifact profile loaders (`:26–29`); React does not retain the editor or present an accepted-document save failure/retry UI. A port must not claim otherwise.
- `mechanicalProfileController.ts` contains request/version invalidation for the separate Case profile workflow. Do not assume that controller already owns or covers the Parts editor's different draft and selection lifecycle.

## Existing Rust services and Dioxus boundary

- `MechanicalPartProfile` is stored on `PartDefinition`; existing Core edit/Undo/Redo and persistence support ordinary accepted profile changes.
- `CoreRequest::MechanicalProfile` takes request ID, definition ID, built-in source and plate-to-PCB measurement. Core dispatches it through `mechanical::builtin_profile` and returns `CoreReply::MechanicalProfile`. React chooses gaps through `plateToPcbGap(family, defaultPlateThickness(family))` and preserves local optional profile fields when the reply arrives.
- `ArtifactRequest::ExtractMechanical` takes request ID, source, purpose mappings and maximum deviation. The existing React adapter passes `maxDeviationMm: 0.005` for both read and apply (`app/src/main.tsx:68–72`). The artifact provider returns the supported extraction/source-provenance record or a typed error.
- The Dioxus Parts view already receives accepted snapshot, scope, query and selected definition in the center preview. It has no selected-part fit editor or Parts-owned typed Runtime/Core/Artifact facade. Root owns shared Runtime adapter integration; children 15 and 16 must not bypass that owner or widen public interfaces.
- Manual profile editing (issue 14) can use the current accepted-document/Event::Edit path without either async adapter. It should expose only the manual fields from the React editor. Issue 15 consumes the Core profile service; issue 16 consumes the Artifact extraction service. Both own only local draft updates and request-identity checks after the coordinator adapter supplies a callable contract.

## Parent and refactoring boundaries

F4.5's current task-graph start edge remains F4.1 and its full acceptance join remains INT.2. These children refine work without changing the 62-parent graph or declaring F4.5 complete. Case fit/fabrication acceptance remains F7. Preserve RF-001/RF-009 existing takeaways for the shared presentation/Runtime hotspot and parity accounting; this inventory asserts no new RF finding.

## F4.5-C02 — KiCad source mapping and saved profile follow-up — 2026-10-04

Paired public UI journey used candidate `http://127.0.0.1:34800/`, build `frontend-functional-batch-20261004`, source `14434d50` (coordinator-confirmed `/boardstudio` HTML/JS hashes), and read-only TypeScript reference `http://127.0.0.1:5175/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`. Owned sessions were `f45-c02-dioxus` and `f45-c02-reference`. Both used a fresh Sofle v2 project and the existing imported fixture `THQWGD001C [4pin] [Reversible].kicad_mod`.

In both Parts editors, `Read KiCad layers` exposed the source primitives and purpose controls. Mapping `geometry-58` (Dwgs.User rectangle) to Plate cutout and `geometry-59` (Dwgs.User rectangle) to Clearance envelope, then using `Apply selected geometry`, populated the same draft with one four-vertex cutout and one four-vertex clearance. The observed coordinates matched across apps: cutout corners ±9.5 mm; clearance corners X −8.9/−3.65 mm and Y ±7 mm. Save closed the editor and exposed `Edit profile`; one Undo returned to `Define profile`, Redo restored `Edit profile`, and reopening the editor retained both contours and their coordinates in each app. This verifies the paired extraction-to-profile-draft and profile edit/Undo/Redo/reopen path on this fixture.

On Dioxus, an exploratory attempt to map the fixture's oval `pad-0-drill` as a PCB mounting hole returned the local message `Mapped PCB mounting drill pad-0-drill must be circular`; clearing that unsupported mapping and retrying the supported rectangle mappings succeeded. No stale-selection race was observable: extraction completed within the immediate UI action sequence, and no public latency injection was available. This receipt makes no stale-selection runtime claim and does not close F4.5 or its INT.2 join.
