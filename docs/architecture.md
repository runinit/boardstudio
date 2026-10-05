# Architecture

BoardStudio is a static browser application with a Dioxus frontend. Durable design
semantics and session state live in Rust. TypeScript and JavaScript remain where
they implement generator/CAD providers, data preparation and development tooling.

| Location | Responsibility |
| --- | --- |
| `core/` | Document validation, typed edits, resolved geometry, artifacts and portable archives |
| `application/` | Accepted snapshots, ordered commands, interactions, save identities and scoped jobs |
| `web/src/presentation/` | Dioxus components, contextual panels and local form drafts |
| `web/src/runtime.rs` | Composition of session state and browser effects |
| `web/` host and worker modules | IndexedDB, worker transport, asset/file I/O, rendering adapters and offline policy |
| `renderer/` | GPU scene rendering, picking and camera behavior |
| `cad/` | CAD provider bindings and the Cadrum/OCCT WASM kernel |
| `contracts/` | Shared generated Rust/TypeScript boundary types |
| `ergogen/`, `kicad/` | Generator catalogue, footprint production and PCB output providers |
| `catalogue/`, `content/` | Component definitions, source assets and bundled examples |
| `tooling/demo-projects/` | Build-time preparation of bundled project archives |

The document engine owns durable state. Presentation submits edits through existing
application/runtime boundaries and displays accepted state; it does not create a
second writable document store. Worker replies, saves and exports retain their
captured project, board and revision identities so stale operations cannot replace
the current scope. Manufacturing output comes from accepted inputs, not rendered meshes.

Heavy CAD and artifact work runs outside the page. Generated JavaScript initializes
WASM modules; browser worker and service-worker entrypoints are packaged with the
page. Root and subpath deployments must preserve asset URLs and COOP/COEP isolation
headers. Ordinary offline caches remain part of the application.

The React frontend and its rollback path are retired. Existing saved-file backward
compatibility is not a current requirement; fresh projects must still round-trip
through persistence and portable archives. Historical cutover work remains in Git
at `323967ff`; there is no active migration contract or acceptance ledger.

See [development commands](../README.md), [domain terminology](../CONTEXT.md),
[component onboarding](hardware/component-onboarding.md) and [backlog](backlog.md).
