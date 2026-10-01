# Establish Rust routes for engine integration and remaining application JavaScript

Labels: wayfinder:research
Type: research
Mode: AFK
Status: resolved
Assignee: /root/dioxus_integration_research
Parent: [Dioxus browser trial and Rust application migration](../map.md)
Blocked by: none

## Question

What must be ported, reused or replaced to connect a Rust/Dioxus browser UI to
the current engines and eventually remove maintained TypeScript/JavaScript
application logic, including CAD adapters and executable Ergogen generators?

Assess verified current code alongside primary sources for the versioned
browser/WASM and engine integration APIs. Inventory the ownership seams for
core edit/preview/commit requests and replies, Undo, worker lifetimes/recovery,
queue supersession, transferable buffers, Rust canvas renderer lifecycle,
CAD kernel/meshing/STEP loading, IndexedDB/assets/archives, committed exports
and the Ergogen-to-KiCad generator path. Distinguish Rust reusable APIs from
WASM/JavaScript-only wrappers; do not assume compiling a crate into the UI
keeps expensive work off the browser's main thread.

Identify feasible Rust browser-worker/transport and renderer/CAD adapter routes,
their compatibility constraints and the evidence the layout and case trials
must produce. For executable generators, describe honest eventual replacement
routes and semantic/output-parity gates; retaining JavaScript or embedding a JS
interpreter is not itself a Rust rewrite of the application logic. Do not
choose a production architecture, widen public APIs or implement a generator.

Capture one cited research artifact on an isolated `research/` branch. Cite
current local files/revisions, external primary sources and version/feature
constraints. Separate verified facts, plausible routes and unresolved decisions.
Account for active encoder/VIK work without claiming it is completed. Link the
artifact and research revision from the answer.

## Comments

- The read-only inventory found Rust domain/contracts, archives, CAD and 3D
  rendering; React/TypeScript owns sessions, browser workers/storage, SVG
  interactions and runtime CAD/generator adapters.
- User requires both layout and case trials, followed by staged migration.
  All maintained application logic must ultimately be Rust; generated browser
  bindings and test/development scripts may remain.
- Research context: branch `research/dioxus-integration-20260930`, based on
  `96dd51d3`, in `/tmp/boardstudio-research-dioxus-integration-20260930`. The
  researcher owns only `.scratch/dioxus-browser-trial/research/rust-integration.md`
  on that branch; the live main checkout may contain newer uncommitted source.

## Answer

Resolved 2026-09-30 from `/root/dioxus_integration_research` findings. This resolves
the integration inventory and possible routes, not a chosen implementation.

The [Rust engine and browser integration evidence](../research/rust-integration.md)
establishes reusable typed core handling, Rust archives/artifacts, a WebGL2
renderer that requires an HTML canvas and public CAD entrypoints whose
interfaces use JavaScript values. Typed CAD helpers remain private; reuse of
existing entrypoints does not require widening their visibility. Session/save
ordering, browser workers/transports, SVG gestures, IndexedDB/assets, CAD
scheduling and renderer lifetime still need Rust application implementations.
Async UI work alone does not move expensive synchronous CAD off the UI thread.

Bundled Ergogen generator execution and normalization depend on parameters,
nets, references and transforms; they are also used before export. A complete
Rust application needs a semantics-preserving rewrite or an agreed declarative/
translation route. Existing custom/imported footprints are data; the current
application does not execute user-supplied JavaScript. The research records
saved-definition compatibility and output-parity gates, not an invented
arbitrary-generator support requirement. Worker packaging, CAD initialization,
copy/transfer accounting and exact behavior still need trial evidence. Candidate
checks and route alternatives remain for the human architecture decision.

The parent captured the artifact on `research/dioxus-integration-20260930` at
`8e9ce913ae0fa4b15a7f9092565f757a2c95868a` in the standalone repository
`/tmp/boardstudio-research-dioxus-integration-capture-20260930`. The
[portable research bundle](../research/rust-integration.bundle) preserves that
commit and branch, requiring baseline `96dd51d3`; `git bundle verify` passed.
SHA-256 of the Markdown artifact is
`fedf09bd76706f24297bd394a7d785b2d1d2eef3b4812147a924ca493410f843`.

Original linked-worktree Git metadata was read-only, so its main-repository
research ref remains at baseline. The captured commit lives in the standalone
clone and bundle; the Markdown copy here matches it. Code observations include
the main checkout's active dirty overlay, explicitly documented in the artifact.
Recheck current contracts before implementation. No production implementation
or engine/browser checks were performed by this research.
