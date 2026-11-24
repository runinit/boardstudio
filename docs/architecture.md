# Architecture

BoardStudio is an offline-capable keyboard design workbench with a Dioxus
frontend. Durable design semantics and session state live in Rust: a
synchronous in-process `CoreEngine` applies typed edits, while
`application::Session` owns that engine plus preview, gesture, save and export
bookkeeping on the main thread. Python scripts handle build, content
preparation and checks; the CAD provider is Rust/WASM behind a dedicated
worker. Core runs synchronously in-process with no queued edit tickets:
presentation calls typed methods and reads accepted state.

| Location | Responsibility |
| --- | --- |
| `core/` | Document validation, typed edits, resolved geometry, artifacts and portable archives |
| `core/crates/engine/` | In-process `CoreEngine` (`dispatch.rs`: `open`, `apply_edit`, `commit_edit`, electrical handoff review) |
| `core/crates/mirrored-half/` | Pure derived projector (`project_document`) plus `MirroredPair` mechanical validation and transitions |
| `core/crates/model/` | Canonical format-8 `ProjectDoc`, document-format admission, operations and wire types |
| `application/` | `Session`: owns the `CoreEngine`, applies `Event`s to accepted snapshots, emits host-owned `Effect`s |
| `web/src/` (`boardstudio-web` bin) | Page shell (`presentation.rs`, downward `presentation/editor/` canvas/camera/selection layer, `presentation/` workspaces), worker entry points (`lib.rs`: `cad-worker`, `service-worker`) |
| `web/crates/runtime/` | `Runtime`: drives a real `Session`, runs browser effects (`runtime.rs`, `runtime/effects.rs`), model delivery, presentation-independent operations; no Dioxus |
| `web/crates/host/` | Browser storage, CAD worker clients, renderer host; IndexedDB and persistence contracts |
| `web/crates/worker-host/` | Packaged worker surfaces: CAD worker client/jobs, offline manifest policy, renderer host page |
| `web/crates/layout-core/` | Unified `CanvasInteraction` pointer/keyboard state machine and canvas projection |
| `web/crates/ui-model/` | UI vocabulary shared by presentation code: tree contexts, selection, workspace and view state |
| `web/crates/ui-shared/` | Shared UI below all workspaces: declarative `inspector_fields`, RAII `DomEventListener`, unified `<Viewport3D />` (`viewport_3d/`), panels, canvas layers, layout camera, footprint graphics, model import |
| `web/crates/catalogue/` | Bundled component catalogue loading, generator normalization, physical setup proposals; no Dioxus |
| `web/crates/workspaces/`, `case/`, `parts/`, `pcb/`, `layout/`, `layout-objects/` | Workspace features (no sideways `-> case` deps): Library, Keymap editor, and keycap fit/settings/3D `<Viewport3D />` assembly viewer; Case viewport overlays and mechanical settings; Parts browser and generators; PCB wiring and physical setup; Layout objects tree, outlines, placement, setup guide |
| `web/crates/test-runtime/` | Shared native fixtures and detached-task support for web tests (`new_native_runtime`, layout fixtures) |
| `renderer/` | GPU scene rendering, picking and camera behavior |
| `cad/` | CAD provider bindings and the pure-Rust CAD WASM kernel |
| `contracts/rust/` | Shared boundary types (`Pose2`, `Vec2`, mechanical wire types) |
| `footprints/` | Built-in generators, parameters, render context, geometry and normalization |
| `ergogen/library/`, `kicad/` | Vendored model assets and attribution; PCB integration checks |
| `catalogue/` | Component definitions, source assets and bundled examples |

## Document core and session

`ProjectDoc` is the canonical format-8 document
(`core/crates/model/src/model/document.rs`,
[`document_format.rs`](../core/crates/model/src/document_format.rs)):
`format` is `"boardstudio/v2"` and `formatVersion` is `8`; anything else is
refused at the storage/archive boundary. `MirroredPair` records the mechanical
link between halves; `mirrored-half` validates every pair
(`validate_pairs`: distinct boards or same-board layouts, finite mapping,
independent ownership) and derives the editable geometry with the pure
`project_document` projector — sources stay canonical, mirrors are computed.
Transitions are typed too: `admit_edit` refuses linked-target edits with
`MirrorEditRefused::LinkedTarget`, and `create_pair`, `set_exception`,
`unlink_pair`, `create_half` move pairs between linked, local, omitted and
removed states.

`Session` (`application/src/session/mod.rs`) holds the `CoreEngine` by value
and applies one `Event` at a time: `Open`, `PreviewEdit`, `ResolveEdit`
(carrying an `EditResolver` that runs once against the accepted snapshot),
`Undo`/`Redo`, gesture events, generation/export events and `Close`. Each
`submit`/`complete` call returns the host-owned `Effect`s (`Persist`,
`Settled`, pointer/frame capture, `RunGeneration`, `RunExport`, …) for the
host to run. Durability (`NoDocument`/`Saved`/`Saving`/`Failed`) and lifecycle
(`Empty`/`Ready`/`Closing`/`Closed`) live in the read model next to the
accepted snapshot, scope and selection.

## Runtime composition

`Runtime` (`web/crates/runtime/src/runtime.rs`) wraps a real `Session` with
typed entry points the presentation calls directly: `submit(event)`,
`commit(label, resolver)`, `operation()`, `scope()` and `model()`. Effect
execution lives in `runtime/effects.rs`; persist completions share two pure
decision helpers with native tests
(`runtime_report::persist_report` and `persist_dispatch`, covered by
`runtime/persist_notification_tests.rs`). Status and notifications split two
ways: `subscribe` observes general refreshes while `subscribe_durability`
observes save-only movement, so the shell can show `"Saved locally."` without
re-rendering the workbench.

`RuntimeReport` (`runtime_report.rs`) is the only notification vocabulary:
`status`/`alert` reports, `transient` one-shot delivery, `publish` into the
shell queue and `take_notifications` to drain it. The native test double
(`runtime_test_stub.rs`, used when not on `wasm32`) drives the same real
`Session` and Core with in-memory saves, so panel and projection tests run
without a browser.

## Canvas interaction

All pointer and keyboard editing flows through the unified `CanvasInteraction`
state machine (`web/crates/layout-core/src/interaction.rs`). One struct owns
the mode plus `space_down`/`control_down` modifiers and consumes `CanvasEvent`s
(`PointerDown`/`PointerMove`/`PointerUp`, `Begin`/`End`, `KeyDown`/`KeyUp`,
`Blur`, `ScopeChanged`, `WorkspaceChanged`). `CanvasMode` names every gesture
the canvas supports — panning, marquee, part and rail drags (`DraggingParts`,
`DraggingMatrixRail`, `DraggingPartRail`), splay and anchor picking,
part/matrix/mirrored-pair placement and outline-handle drags — so workspaces share hit-testing, snap and preview behavior instead of
reimplementing gestures.

## Offline policy

One service worker serves every route from a validated manifest
(`web/src/service_worker.rs`, `OfflineManifest` in
`web/crates/worker-host/src/offline.rs`). Install caches the shell manifest
(`shell_assets`, always including `index.html`) with a single `addAll`; fetch
serves shell and cached assets first and only stores responses whose URL
matches `runtime_cache_prefixes`. That split keeps the app shell available
offline while heavy provider payloads stay on demand:
`scripts/build-web.py` stages `assets/ergogen-models/`, `assets/cad/`,
`assets/cad-worker/bundled-step/` and `assets/modules/` under those prefixes,
writes a small `modules-index.json`, and ships per-module assets lazily instead
of inside the shell bundle. Caches are version-scoped and the worker deletes
older same-scope caches on activation.

## Build and test topology

`python3 scripts/check.py` owns the gates. Native-first: `test` runs one
workspace `cargo test` (lib, bins and integration targets) plus the CAD and
KiCad provider suites, so pure controller, projection and layout tests run as
plain `#[test]`. `typecheck` pins the page binary for `wasm32`; `lint` runs
Clippy over the workspace and again for `wasm32` across the page, host,
worker-host, runtime, UI and workspace crates. `browser` stays narrow:
the drag smoke test, the CAD browser suite, provider checks and the
packaged-page suite (`scripts/run-wasm-tests.py --all --depth 1`), which owns
every `#[wasm_bindgen_test]` in `web/src/page_suite_tests/` (the host and
worker-host browser tests run there through the page binary's `test-support`
seams); everything else proves itself natively, with
`test-runtime` fixtures bridging the two worlds.

See [development commands](../README.md), [product purpose](../PRODUCT.md),
[design system](../DESIGN.md), [domain terminology](../GLOSSARY.md) and
[coding standards](../CODING_STANDARDS.md). The [documentation
index](README.md) separates current references from design, research and
archived workflows.
