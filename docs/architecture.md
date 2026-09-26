# Current architecture and validation

This describes the repository after the September 2026 cleanup. Migration reports
and test audits retain their original evidence; their counts are not current
coverage targets. Packages now live directly at the repository root.

## App ownership

The app entrypoint composes controllers and the workbench. Controllers receive
explicit document references and actions; there is no additional global store.

| Owner | Responsibility |
| --- | --- |
| `useProjectSession` | Core worker lifecycle, serialized operation queue, committed versus preview scenes, persistence, project-open invalidation |
| `createProjectActions` | Undo/redo, new/import/duplicate projects, imported parts/models, undoable edits |
| `useCaseGeneration` | Physical-instance resolution, generation state, cancellation, cache reuse, revision/session checks |
| `useElectricalPlanning` | Wiring resolution, pin assignments, locks, protected handoff review data, export wiring preparation |
| `createProjectExporter` | Project, PCB, firmware, footprint, outline, and mechanical packaging; model assets; downloads |

The project session and case controller share the existing preview cache only so
opening a project invalidates it. Async callbacks keep using current document
references and captured revision/context guards. Export protection is persisted
only after the artifact is produced, before its download.

The workbench keeps shared selection state and pointer transaction references.
`useWorkbenchSelection` handles selection actions; `useWorkbenchTree` derives the
object tree. `createCanvasInteractions` owns drag, stagger, splay, and pan updates,
including the final pointer sample, captured transaction identity, and rollback.
Escape and board/mode switches cancel active edits; opening another project
discards captured work without replaying it into the new document.
`createCanvasCamera` handles camera actions, and `CanvasObjects` renders geometry.
`MatrixInspectorPanel`, `CaseInspectorPanel`, and `PartsInspectorPanel` compose
selection-specific controls through explicit action callbacks. Document mutations
remain in the workbench and `createLibraryActions`.

`planKeycapResize` plans keycap dimensions and neighbour reflow from the current
document and resolved placements. It maps linked-half selections to canonical
cells and returns one document edit with affected IDs, without mutating its inputs.
The workbench submits that edit through the existing undo path; core layout
resolution remains responsible for propagating mirrored positions.

Parts preview construction and placement are separate: `sampleAssembly` produces
an isolated preview document; `assemblyPlacement` creates document snapshots.
`assemblyCatalog` is the preset registry used by recipes and selectors.
`AssemblyViewer` is the single preview model-loading owner. Export model packaging
has its own asset resolver because it packages source files rather than meshes.

`usePartsEditing` owns generator drafts, asynchronous compilation, preview state,
and net-preserving saves. Its terminal remapper validates every placed instance
before changing any cloned nets. Authored and imported parts keep their separate
editing path. Placement uses one discriminated state for standalone parts, matrices,
mirrored pairs, and setup; cancellation invalidates pending matrix projections.

Case preview race tests exercise `generateCasePreview`, the preparation and CAD
preview sequence used by `useCaseGeneration`. CAD creation remains lazy until
preparation succeeds for the current context. Full export uses the separate CAD
request path.

## Rust ownership and privacy

`core/src/artifact/kicad.rs` retains shared formatting, footprint serialization,
and snapshot/electrical validation helpers. Private child modules handle export
planning and completed board/footprint assembly. The existing `prepare_export`
and `finish_export` entrypoints are re-exported without changing their signatures.
Exporter tests live beside output assembly and can inspect private ancestor helpers.

`cad/wasm/src/lib.rs` is the WASM facade and initialization boundary. Its private
model module owns input/result types, mesh conversion, and STEP import/export.
The construction child owns solid building; its cache child owns preview-region
and body cache lifetimes. Existing WASM entrypoints are re-exported unchanged.
Nested ownership lets children use private ancestor types and helpers without
widening field or helper visibility.

Mechanical resolution has private profile, part-geometry, battery-space, stack,
foam-contour, body-construction, and finalization stages. Part geometry returns
PCB reference holes, component volumes, profile openings, and foam exclusions;
construction returns bodies and additional stack layers. Ordered diagnostics flow
through the resolver's shared callback until final validation. Electrical resolution
separates key discovery, required signals, GPIO allocation, net construction,
net finalization, and reversible-jumper checks. Net construction returns peripheral,
RGB, and key nets plus assignments and ordered diagnostics before jumper aliases
are applied. Their diagnostic ordering and public entrypoints remain unchanged.

The renderer's private stack ranking and explosion helpers share one source file
between the WASM module and native regression tests; there is no parallel
TypeScript implementation. Mechanical footprint and profile points share one
private mirror/rotation transform.

## Current project format

Only the current `boardstudio/v2` shape is supported. There are no released older
projects to migrate. Opening a project does not repair old dimensions or rewrite
library definitions. Missing required mechanical dimensions and removed fields
on part definitions, matrices, or cells are rejected.

- Part models use `models`; the old singular `model` field is removed.
- Matrix companions come from explicit assembly members. The old matrix `diodes`
  and cell `diode` flags, implicit RGB chains, and their net cleanup are removed.
  Diode direction remains an input to the current electrical planner.
- Matrix membership uses canonical cell IDs. Ordered-ID recovery and residual
  interpolation for early projects are removed. Empty cells use parametric poses.
- The six generic built-in footprint generators and their compiled catalog are
  removed. The starter uses the canonical Ceoloide MX switch.
- Infused-Kim Choc and diode sources are retained as attributed reference files,
  but excluded from the executable catalog. The runtime has 37 active generators.
- Custom/imported parts, authored case bodies, and current assembly snapshots
  remain supported. KiCad syntax adapters remain necessary for bundled generators
  and supported external footprint imports.

The Rust model is the source for generated TypeScript contracts. No renderer
protocol, mechanical cutting profile, generator source geometry, or CAD kernel
changes are part of this removal. TypeScript checks unused locals and parameters.

## Validation entrypoints

- `pnpm check`: contract and generator-catalog drift, runtime imports, native and package tests,
  WASM and production builds, boundary checks, browser tests, and Pages checks.
- `pnpm precommit`: core/CAD/renderer preparation and app/CAD typechecks.
- `pnpm test:e2e:dev`: development-server loading regressions.
- `pnpm test:perf`: the separately budgeted performance suite.

Use `BOARDSTUDIO_CHROMIUM=/usr/bin/chromium` on hosts with system Chromium and no
Playwright-managed browser. Keep browser behavior assertions when updating stale
selectors: selection now uses the Select menu, and canonical assemblies number
both switches and their companion parts.

Geometry-only browser fixtures have no controller and use the explicit draft PCB
handoff. Fixtures with existing manual nets must review their replacement through
the wiring UI first. The real-MCU handoff test covers production PCB and firmware
exports. Authored geometry/model tests use custom definitions rather than relying
on retired parts appearing in the placement catalog. Reload tests wait for the
committed revision and completed local save.

The CAD build removes its generated `wasm/pkg/package.json` before invoking
wasm-pack 0.15. That version otherwise reads its previous full manifest as a
dependency-only map and rejects array fields on repeated builds. Source manifests
and compiled caches are retained.

For export equivalence, the core test
`native_board_output_can_be_written_for_kicad_cli_oracle` accepts
`BOARDSTUDIO_KICAD_ORACLE_PATH` and writes front/back boards. Compare those files
across a structural change. CAD tests verify unchanged-region cache reuse and STEP
round-trip geometry; STEP timestamps are not a byte-equivalence contract.
