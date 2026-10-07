# Architecture

BoardStudio is a static browser application with a Dioxus frontend. Durable design
semantics and session state live in Rust. Python scripts handle build, content
preparation and checks; the CAD provider is Rust/WASM.

| Location | Responsibility |
| --- | --- |
| `core/` | Document validation, typed edits, resolved geometry, artifacts and portable archives |
| `application/` | Accepted snapshots, ordered commands, interactions, save identities and scoped jobs |
| `web/src/` (`boardstudio-web` bin) | The page shell (`presentation.rs`): app state, workspace containers and composition, the component inspector, Export and the Case workspace container |
| `web/crates/runtime/` | Composition of session state and browser effects (`runtime.rs`), model delivery and presentation-independent operations; no Dioxus |
| `web/crates/host/` | IndexedDB, Core and CAD worker clients, renderer host and offline policy |
| `web/crates/ui-model/` | UI vocabulary shared by presentation code: tree contexts, selection, workspace and view state, canvas interaction ownership |
| `web/crates/ui-shared/` | UI used by several workspaces: panels, canvas layers, layout camera, footprint graphics, geometry scripts, model import |
| `web/crates/catalogue/` | Bundled component catalogue loading, generator normalization and physical setup proposals; no Dioxus |
| `web/crates/keycaps/`, `library/`, `keymap/`, `case/`, `parts/`, `pcb/`, `layout/` | Workspace features: keycap fit, settings and scene; the Library page; the Keymap editor; the case viewer, mechanical settings, closure clearance and the shared 3D viewer; the Parts browser, definitions, generators and assemblies; PCB wiring, routed-board references, modules and physical setup; the Layout objects tree, outlines, part placement and setup guide. Workspace containers stay in the bin |
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

Product edits submit accepted-state intents through `EditTicket`; previews and
strict captured operations keep their existing routes (see the
[edit-settlement contract and resolver inventory](plans/edit-settlement/map.md)).

The page is split into crates by workspace so each stays a manageable size. Lower
crates never depend on higher ones:

1. `runtime` and `host` (no Dioxus), then `catalogue` (no Dioxus);
2. `ui-model`: shared types only, so an edit there recompiles every presentation crate;
3. `ui-shared`: UI that several workspaces mount;
4. workspace crates, each on the crates below it: `keycaps`, `library` and `keymap`;
   `case`; `parts` (on `case`); `pcb` (on `parts`); `layout` (on `keycaps`, `case` and
   `parts`);
5. the `boardstudio-web` bin, the page shell.

An edit inside a workspace crate recompiles that crate, the crates above it and the bin.
Every presentation crate's `lib.rs` aliases itself as `presentation` and re-exports the
modules its code addressed in the bin (`crate::runtime`, `super::SelectionAdapter`,
`super::objects::TreeContext` and so on), and the bin re-exports each moved module where
it was declared, so code keeps the paths it had before the split. Helpers that other
crates' tests drive are behind each crate's `test-support` feature, which only
dev-dependencies enable. Each crate runs its own browser tests with
`wasm-pack test --headless --chrome web/crates/<crate> --lib` (`scripts/check.py browser`
runs them all).

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
