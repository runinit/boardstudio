# Architecture

BoardStudio is a static browser application with a Dioxus frontend. Durable design
semantics and session state live in Rust. Python scripts handle build, content
preparation and checks; the CAD provider is Rust/WASM.

| Location | Responsibility |
| --- | --- |
| `core/` | Document validation, typed edits, resolved geometry, artifacts and portable archives |
| `application/` | Accepted snapshots, ordered commands, interactions, save identities and scoped jobs |
| `web/src/` (`boardstudio-web` bin) | Dioxus components (`presentation/`), contextual panels and local form drafts |
| `web/crates/runtime/` | Composition of session state and browser effects (`runtime.rs`), model delivery and presentation-independent operations; no Dioxus |
| `web/crates/host/` | IndexedDB, Core and CAD worker clients, renderer host and offline policy |
| `web/crates/ui-model/` | UI vocabulary shared by presentation code: tree contexts, selection, workspace and view state, canvas interaction ownership |
| `web/crates/ui-shared/` | UI used by several workspaces: panels, canvas layers, layout camera, footprint graphics, geometry scripts, model import |
| `web/crates/keycaps/`, `library/`, `keymap/` | Workspace features: keycap fit, settings and scene; the Library page; the Keymap editor. Workspace containers stay in the bin |
| `web/src/lib.rs` | Worker entry points packaged by wasm-pack (Core, CAD and offline service worker) |
| `renderer/` | GPU scene rendering, picking and camera behavior |
| `cad/` | CAD provider bindings and the Cadrum/OCCT WASM kernel |
| `contracts/` | Shared Rust boundary types |
| `footprints/` | Built-in generators, parameters, render context, geometry, normalization and model references; Core and the page call Rust directly |
| `ergogen/library/`, `kicad/` | Vendored model assets and attribution; PCB integration checks |
| `catalogue/`, `content/` | Component definitions, source assets and bundled examples |
| `core/examples/demo_projects/`, `content/layouts/` | Native bundled project preparation through public Core edits, wiring and archive APIs |

The document engine owns durable state. Presentation submits edits through existing
application/runtime boundaries and displays accepted state; it does not create a
second writable document store. Worker replies, saves and exports retain their
captured project, board and revision identities so stale operations cannot replace
the current scope. Manufacturing output comes from accepted inputs, not rendered meshes.

The page is split into crates so an edit in presentation code recompiles only the
`boardstudio-web` bin, not the runtime, host or Core. The bin re-exports the runtime's
modules at its root, so presentation code addresses them as `crate::runtime` and so on.
Shared presentation types live in `ui-model`, which the bin re-exports under
`presentation` so modules keep their `super::` paths; presentation crates depend on it
rather than on each other or on the page shell, and an edit there recompiles them all.
Runtime and ui-model helpers that the page's own tests drive are behind each crate's
`test-support` feature.

Heavy CAD and artifact work runs outside the page. Generated JavaScript initializes
WASM modules; browser worker and service-worker entrypoints are packaged with the
page. Root and subpath deployments must preserve asset URLs and COOP/COEP isolation
headers. Ordinary offline caches remain part of the application.

The React frontend and its rollback path are retired. Version 1 saved documents migrate to format version 2 at storage/archive load
boundaries before opening. Current projects round-trip through persistence and
portable archives. Historical cutover work remains in Git
at `323967ff`; there is no active migration contract or acceptance ledger.

See [development commands](../README.md), [domain terminology](../CONTEXT.md),
[component onboarding](hardware/component-onboarding.md) and [backlog](backlog.md).
The [documentation index](README.md) separates current references from design,
research and archived workflows. The [Layout part-editing investigation](investigations/layout-part-editing.md)
records an open edit-lifecycle concern without changing the ownership above.
