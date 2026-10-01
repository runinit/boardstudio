# Current architecture and validation

This describes the repository after the September 2026 cleanup. Superseded
migration reports are removed; retained research and deletion decisions are
listed in [the cleanup inventory](repository-cleanup.md). Packages live directly
at the repository root.

## App ownership

The app entrypoint composes controllers and the workbench. Controllers receive
explicit document references and actions; there is no additional global store.

| Owner | Responsibility |
| --- | --- |
| `useProjectSession` | Core worker lifecycle, serialized operation queue, committed versus preview scenes, persistence, project-open invalidation |
| `createProjectActions` | Undo/redo, new/import/duplicate projects, imported parts/models, undoable edits |
| `useCaseGeneration` | Physical-instance resolution, generation state, cancellation, cache reuse, revision/session checks |
| `useElectricalPlanning` | Wiring resolution, pin assignments, locks, and protected handoff review data |
| `createProjectExporter` | Queued export snapshot capture and browser delivery |
| `exports/` | Project/footprint/outline, PCB, firmware, keycap, and case workflows; source model packaging; handoff protection |
| `AssemblyPreview` | Preview worker lifetimes, independent PCB/keycap requests, cancellation, model conversion cache, stale reply rejection |

The project session and case controller share the existing preview cache only so
opening a project invalidates it. Async callbacks compare captured document, scene, board, physical instance, and
project session. Export workflows use one committed `ExportContext`; only their
own accepted wiring edits can advance it. A project with the same ID and revision
but a different open session cannot receive a previous export. PCB protection is
persisted only after packaging succeeds and before the download. Export CAD uses
a separate client from preview CAD.

The workbench composes feature interfaces and keeps shared selection, placement,
canvas projection, pointer transactions, and panel navigation. Feature drafts and
editing actions belong to these owners:

| Feature owner | Responsibility |
| --- | --- |
| `useWorkbenchNavigation` | Workflow return state and per-board workflow cameras; project/session reset |
| `useWorkbenchTheme` | Browser preference, system appearance subscription, document theme |
| `useOutlineEditor` | Outline drawing/point selection, snap grid, keyboard actions, inspector and canvas overlays |
| `ConstraintEditor` | Placement relationship form drafts and add/update/remove actions |
| `useScriptEditor` | Geometry script selection, source/result drafts, saves, and findings |
| `usePcbWorkspace` | Net creation, terminal assignments, generator overrides, and PCB inspector |
| `useCaseWorkspace` | Authored bodies/mounts, layer selection/appearance, readiness, diagnostics, and case inspector |
| `createKeymapWorkspace` | Shared key membership/read model, selection, typed edit targets, keymap inspector and canvas |

`useWorkbenchSelection` handles shared selection actions; `useWorkbenchTree`
derives the object tree. `createCanvasInteractions` owns drag, stagger, splay,
and pan updates, including the final pointer sample, captured transaction identity,
and rollback. Escape and board/mode switches cancel active edits; opening another
project discards captured work without replaying it into the new document.
`createCanvasCamera` handles camera actions, and `CanvasObjects` renders geometry.
`MatrixInspectorPanel` and `PartsInspectorPanel` compose selection-specific controls.
`createWorkbenchPlacementActions` prepares and commits part, matrix, and mirrored
pair placement using the normal edit path. `createWorkbenchEditActions` holds pure
terminal-assignment, definition-replacement, and constraint builders.

Keymap and firmware-position controls submit `set-key-binding`, `set-keycap-board`,
`set-matrix-keycaps`, and `set-keycap-key` operations. Rust merges those field edits
into the current document, validates targets and dimensions, and preserves other
bindings, pin locks, and protected handoffs. Null inherits a legend; an empty string
selects a blank keycap. Undo and redo use the existing core history. Presentation
uses generated Rust defaults and does not manufacture replacement configurations.

`planKeycapResize` plans keycap dimensions and neighbour reflow from the current
document and resolved placements. It maps linked-half selections to canonical
cells and returns one document edit with affected IDs, without mutating its inputs.
The workbench submits that edit through the existing undo path; core layout
resolution remains responsible for propagating mirrored positions.

Parts preview construction and placement are separate: `sampleAssembly` produces
an isolated preview document; `assemblyPlacement` creates document snapshots.
`assemblyCatalog` is the preset registry used by recipes and selectors.
`AssemblyViewer` renders snapshots from `useAssemblyPreview`. Its `AssemblyPreview`
owner creates workers lazily, retains at most 80 model conversion promises, evicts
failed conversions, and closes all owned workers on unmount. PCB and keycap inputs
have separate request sequences: stale core resolution never starts CAD, obsolete
keycap CAD is aborted, and late replies cannot replace the current scene. In-flight
model conversions can be reused across geometry revisions within the same scope.
Project session, board, or instance changes clear the previous scene and cache.
Export model packaging has its own asset resolver because it packages source files
rather than meshes.
`assemblyEditorController` owns immutable member/model changes, assembly saves,
and asset-import document guards. `MechanicalProfileController` owns asynchronous
library/stabilizer profile assignment. It rejects responses after a scope change,
newer mechanical configuration, superseding request, or editor unmount. The
existing `MechanicalDraft` continues to reconcile queued edits and acknowledgments.

`usePartsEditing` owns generator drafts, asynchronous compilation, preview state,
and net-preserving saves. Its terminal remapper validates every placed instance
before changing any cloned nets. Authored and imported parts keep their separate
editing path. Placement uses one discriminated state for standalone parts, matrices,
mirrored pairs, and setup; cancellation invalidates pending matrix projections.

`useSetupGuide` persists only the guide's open state and selected step, scoped to
the project ID in browser preferences. `createProjectActions` requests automatic
opening only after a new document has been accepted and saved successfully.
Opening/importing existing projects does not restart the guide. Completion is
derived by `deriveSetupGuide` from the selected board, current layout, applied
wiring, and existing case readiness; it is never saved as project data. Guide
actions use the normal workbench controls and exporters retain their own checks.
Electrical presentation captures the document object and board as well as the
revision, so a response from another open project cannot become the active plan.

Case preview race tests exercise `generateCasePreview`, the preparation and CAD
preview sequence used by `useCaseGeneration`. CAD creation remains lazy until
preparation succeeds for the current context. Full export uses the separate CAD
request path.

## Rust ownership and privacy

`contracts/rust` owns the shared keycap profile/mount enums, keycap configuration,
`KeycapSpec`, and its 2D pose types. Core re-exports preserve existing Rust paths;
CAD consumes the same specification without a private duplicate. The crate contains
wire types and defaults, with no CAD kernel or core-engine dependency. Core owns
profile resolution, settings validation, and clearance; CAD owns solid construction.
Contract generation writes both TypeScript types and `keycapDefaults.ts`. The drift
check covers defaults as well as schemas; runtime barrels export only intended values.


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

- `pnpm check:repo`: authored module/export reachability, runtime dependencies,
  local documentation links, and feature ownership boundaries, with scanner fixtures.
  Workbench/preview/keymap presentation must delegate domain workers; preview/export
  domain modules cannot import UI; keymap controls cannot replace whole documents.
- `pnpm check`: repository hygiene, contract and generator-catalog drift, runtime imports, native and package tests,
  WASM and production builds, boundary checks, browser tests, and Pages checks.
- `pnpm precommit`: core/CAD/renderer preparation and app/CAD typechecks.
- `pnpm test:e2e:dev`: development-server loading regressions.
- `pnpm test:perf`: the separately budgeted performance suite.

Use `BOARDSTUDIO_CHROMIUM=/usr/bin/chromium` on hosts with system Chromium and no
Playwright-managed browser. Keep browser behavior assertions when updating stale
selectors: selection now uses the Select menu, and canonical assemblies number
both switches and their companion parts.
Timing tests opt into `?cadMetrics=1` and wait for the first accepted renderer scene
before reading cold-start measures. Legacy gasket fixtures specify their saved
tabbed configuration and dimensions; selecting Gasket in the UI enables the newer
internal generator. Functional tests open disclosures before editing their controls.
Set `BOARDSTUDIO_TEST_PORT` to a free port when verifying an isolated worktree;
production, Pages, and performance tests share that override without stopping an
existing preview server. The default remains 4328; development tests use 4329.

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

Interface tests cover preview supersession, session/board/instance changes, shared
in-flight model conversion, failed-cache retry, worker closure, navigation camera
isolation, stale export delivery, and packaging/protection/save ordering. Public
core tests cover field merging, invalid edits, encoder push positions, and undo/redo.
Native/WASM boundary parity includes all four keymap edit operations.

For export equivalence, the core test
`native_board_output_can_be_written_for_kicad_cli_oracle` accepts
`BOARDSTUDIO_KICAD_ORACLE_PATH` and writes front/back boards. Compare those files
across a structural change. CAD tests verify unchanged-region cache reuse and STEP
round-trip geometry; STEP timestamps are not a byte-equivalence contract.

## Experimental P1 CAD worker boundary

P1-r1 and its renewed P1-CAD-F1-r1 review follow-up test packaging with a copied
holed-plate input. This is an isolated prototype under
[`.scratch/prototypes/p1-cad/`](../.scratch/prototypes/p1-cad/README.md), governed by
[ADR 0003](adr/0003-rust-application-ownership.md) and the
[run ledger](migration/RUN.md). The coordinator owns it. It does not replace the
current application architecture described above or establish full M1 parity.

| Boundary | Purpose and owners |
| --- | --- |
| Dioxus report → Rust host client | Dioxus owns only the test report signal. The client owns each Worker handle, readiness, executor/operation ledger, pending callers and accepted mesh cache. The copied prepared-case input remains immutable for each request. |
| Rust host → Rust worker | Opaque serde_json text frames carry executor/operation identity and prepared JSON; separate owned ArrayBuffers carry meshes and STEP. Workers have a serialized job queue and executor-local completed cache base/body metadata. Valid older completions update the host cache before obsolete display results are rejected. Wrong-base and obsolete executor results cannot advance it. |
| Rust worker → separately generated CAD module | Pinned wasm-bindgen imports existing generated CAD exports. The CAD Rust provider and OCCT own geometry and private solid/region/body caches. Preview and committed export have distinct worker/module lifetimes; export captures its prepared input and requires no preview cache. |

Initialization JS is generated by the prototype build script and only imports
and initializes the Rust worker. CAD JS is generated binding/runtime glue, not
maintained application policy. The reference/control scripts are development
tooling calling the existing public TypeScript CAD facade. Core preparation runs
through the existing public native example; no private API or provider visibility
is widened. Accepted versions and contracts remain unchanged.

Cancellation controls bypass the job queue. Queued jobs and active previews at
cooperative yields settle without publishing or advancing completed cache metadata.
Synchronous OCCT is not interruptible; its private geometry cache may retain work
computed before cancellation. Job errors settle the caller without advancing the
completed cache and permit retry. Close, crash, initialization failure and deadline
failure settle pending callers without replay. The host terminates its workers on
Drop and clears browser callbacks; the worker registration ends with its worker.

CAD copies output from its WASM memory into generated JS typed arrays. The Rust
worker transfers the original owned buffers, which detach at the sender. The host
copies each received buffer once to Rust Vec storage shared with its cache through
Rc. STEP reopening copies Rust bytes to a JS array, transfers it, and CAD copies
into its own WASM memory. The measured fixture transfers 4,608 preview bytes and
31,318 export bytes; these counts exclude JSON framing and internal CAD copies.
Separate CAD runtimes consume additional initialization and memory. No latency,
memory budget or whole-workflow improvement is claimed.

Cold preview can use the existing planar mesher, while an export cache miss uses
the kernel mesher and warms the body cache. References must compare the same
executor lifetime and operation history; different valid triangulations do not
justify relaxing geometry, material, bounds or STEP oracles. The control matrix
covers fresh preview, fresh export and both shared-cache operation orders.

The coordinator-owned retirement follow-up is **P1 executor retirement**, linked
to the approved [host-platform acceptance](../SPEC-host-platform.md#success-criteria)
and [P2/P3 integration gates](../SPEC-host-platform.md#commands-and-testing-strategy).
It is part of the future explicitly approved production executor/session slice. Retire this experimental runtime boundary
only when that slice provides equivalent ownership, cancellation, cache, export
and resource evidence and production adoption is explicitly authorized. Preserve
the prototype branch and reports as primary evidence; passing P1 alone authorizes
no production promotion, React removal or source-worktree deletion. Revision7 is
the tested fixture; full-range u64 support in the existing CAD JS Number ABI,
other geometry, complete platform coverage, Undo/redo and M1 performance remain
unproven by this experiment.

## Experimental P2 browser lifecycle boundary

The approved P2-r1-R1 run continues the accepted lifecycle investigation in
[the isolated prototype](../.scratch/prototypes/p2-lifecycle/PLAN.md), with its
actual authority and current gate status in
[the renewal record](../.scratch/prototypes/p2-lifecycle/evidence/renewal/PLAN.md).
It is an investigation of one copied Reviung 41 document, an SVG gesture surface
and the existing renderer canvas. The coordinator owns the experiment. It does
not replace the production session or establish full editor, persistence or
performance parity. An interim runnable build remains incomplete while any
original acceptance gate is blocked.

The prototype uses modules in one isolated web package rather than new
production crates. Gesture policy must be testable without Dioxus or DOM
handles; browser attachment has a different lifetime from the root-owned core
worker. Keeping those owners separate allows a panel to unmount without
destroying engine history. Dioxus presents immutable document/scene snapshots
and owns panel visibility and focus presentation. It does not run a second
CoreEngine. The native engine harness is a public-provider characterization
test, not the browser's document authority.

The web host reuses the accepted P1 core worker and opaque serde_json framing,
including float_roundtrip and executor/operation identity. The worker alone
owns the browser's CoreEngine. Requests and replies copy JSON text between
separate WASM memories; an immutable Rust read model is decoded at the host.
The fixture is 105,292 bytes before parsing, not a measurement of every request,
reply or retained heap. No zero-copy, latency or whole-application memory claim
follows from Rust components.

The renderer adapter consumes existing Core SceneDelta geometry through the
unchanged generated renderer entrypoints. Its bounded input is a bare board:
contours and thickness, with empty surfaces, holes and models. Paired renderer
evidence must use that same reduced public input. This attachment proof cannot
establish full-project 3D fidelity or production scene-preparation scheduling.
The renderer still owns its camera and GPU state. The adapter owns actual DOM
elements, ResizeObserver, browser listeners and pending animation-frame handles;
dirty work queues a single frame rather than a continuous idle redraw loop.
Submission counters are instrumentation, not proof that pixels were painted.

Mount conversion and initialization failures must be explicit. A cancelled
mount checks its panel generation after each await and before allocation or
publication. Cleanup must attempt all owned releases, report failures, and
prevent further frames or late scene publication. Context loss is an explicit
failure; recovery is not claimed. Real browser evidence, including visible
pixels, pointer capture, final samples, cancellation, DPR/resize and repeated
unmount/remount, decides whether these lifetime contracts hold.

The original React application remains the reference and fallback on an
isolated origin. No production provider behavior, schema, backend or framework
pin changes are introduced. The renewal permits only the recorded four-file
renderer formatter patch; baseline strict Clippy failures remain acceptance
blockers rather than authorizing provider fixes or lint suppression.

The named coordinator-owned retirement follow-up is **P2 lifecycle retirement**,
part of the next explicitly approved production session/web-host slice. Retire
this runtime and its copied P1 host adapter only after that slice provides
equivalent public-engine history, captured gesture, stale-result and browser
resource evidence at both deployment prefixes, and production adoption is
explicitly authorized. Preserve the branch and reports as evidence. P2 alone
does not authorize React removal, prototype promotion or worktree deletion.
