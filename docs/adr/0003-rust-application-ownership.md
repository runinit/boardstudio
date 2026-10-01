---
status: accepted
---

# Rust application ownership and browser trial boundaries

Proposed and approved by the user on 2026-10-01. Decision index:
[capability map and blocking resolutions](../../CAPABILITY-MAP.md).
This records accepted architecture scope; it is not a detailed module spec,
permission to widen APIs, or evidence of a completed trial.

## Context

The [context assessment](../dioxus-context-baseline.md#current-architecture)
establishes substantial existing Rust capability. The remaining language and
lifecycle boundaries include TS session/interaction coordination, transport,
CAD facade policy, generators, browser storage and authored offline policy.
Replacing markup alone would leave the accepted full-Rust destination unmet.

Reuse the [Wayfinder destination](../../.scratch/dioxus-browser-trial/map.md),
[constraints](../../CONSTRAINTS.md), [current architecture](../architecture.md),
[fixed outline](0001-fixed-edited-outlines.md) and
[linked refinement](0002-linked-outline-refinements.md) decisions. Preserve
document formats, geometry/tolerances, history, gestures, export readiness and
asset identities; the current internals may change. Concurrent encoder/VIK work
is protected and must be reconciled before implementation, not silently absorbed
from the dirty main checkout into this committed-baseline proposal.

The closing inventory observed concurrent changes to main-checkout Workbench,
PLAN/TODO/CHANGELOG and the encoder/VIK implementation index. This task wrote
none of those main-checkout files. Their progress is left intact; the isolated
assessment's production sources and preserved decision copies remain unchanged.
[Scope-review evidence](../../.scratch/dioxus-scope-review/review-checks.json)
records the changing overlay separately from the committed reference.

## Decisions and alternatives

### 1. Full Rust and required platforms

**Observed problem.** “Full Rust” could mean a Rust UI, all application runtime
logic, or a dependency tree without JS/C++. Only the middle meaning matches the
accepted destination while reusing the selected CAD kernel. “Cross-platform”
framework support also does not establish BoardStudio support.

**Improvement.** All maintained application runtime decisions move to Rust,
including session, gestures, offline policy and parameter-dependent generators.
Allow generated bindings/bootstrap and pinned upstream framework/browser
runtime code, Cadrum 0.8.20/OCCT 8_0_1_rev2, three-d 0.19.0, imported source/data
and development/test tools. Rust-authored policy emitted through a generated
loader remains Rust-owned; JS policy authored in `write-sw.mjs` remains JS
application logic. Exported firmware/build files are artifacts/tooling.

Require the existing web/static-host model, local-first operation and desktop
editing with compact layouts. Propose Chromium for first-milestone acceptance,
matching existing Playwright configuration; no native/server milestone.
Firefox/WebKit coverage is unestablished and must precede a wider support claim.

**Alternatives/tradeoffs.** A UI-only port is cheaper but insufficient. Replacing
OCCT or eliminating every upstream JS runtime would expand kernel/framework
scope without evidence. A server simplifies heavy execution but changes offline,
privacy and deployment contracts. Dioxus desktop/native remains an optional
future host, requiring its own browser/canvas/storage replacement evidence.

**Preserved/change.** Preserve accepted web behavior and CAD authority. The
explicit upstream-runtime allowance and first-milestone browser matrix are
proposed refinements, not a waiver for maintained application JS.

**Validation.** Review runtime dependency/asset reachability and provenance;
verify static paths, offline behavior and actual browser capabilities. Framework
platform claims alone do not prove them. [P1/P3](../../CAPABILITY-MAP.md#isolated-integration-prototypes)
answer browser packaging/offline questions without reopening kernel selection.

### 2. Engine, editor session, interactions and UI state

**Observed problem.** `useProjectSession`, feature controllers and transport
caches coordinate different snapshots and async lifetimes. Framework state and
refs currently encode application sequencing. Putting all of it in Dioxus
signals would preserve that coupling; moving selection/drafts into ProjectDoc
would conflate editor state with the durable model.

**Improvement.** A headless session owner serializes commands and owns accepted
read models, scope, persistence acknowledgment, selection, 2D navigation/camera,
gesture transactions and generation/export intent. Engine owns authoritative
document edits/history/geometry. Interaction modules turn normalized events
into session intents; host retains browser event/pointer handles. The renderer
retains live 3D camera/GPU state, while the session selects view presets and
coordinates commands; these are different states, not mirrored camera stores.
Dioxus owns text drafts, panel display and focus presentation until a typed
intent is submitted. Reusable UI drafts disappear or reset with their scope.

**Alternatives/tradeoffs.** One Dioxus application store saves a boundary but
couples business lifetimes to component mounting. Putting the whole coordinator
inside core introduces browser/application workflows into a domain engine.
A separate crate for every workspace multiplies interfaces. One headless
application crate plus session/interaction modules is justified now by native
characterization and dependency isolation, rather than speculative platforms.
Web host/presentation modules share one package; existing engines stay separate.

**Preserved/change.** Keep one writable document/history authority, revision
checks, one-step gesture Undo, final pointer samples, selection/focus/snap
semantics and project/board-switch cancellation. Internal owner/packaging changes
are refactors. Component unmount must not cancel an already accepted save merely
because a panel vanished; explicit session/application teardown owns shutdown.

**Validation.** Characterize existing public requests and session/action tests;
compare the same interaction trace across frontends. Native tests with controlled
executor/storage ports must expose ordering and late replies. P2 proves real
browser capture/focus behavior; an in-memory state demo alone cannot.

### 3. Execution and transport

**Observed problem.** Existing core transport serializes JSON around an already
typed Rust engine. TS clients own settlement/restart and CAD scheduling.
Large CAD/module initialization and serialization costs remain regardless of
UI language. Dioxus async tasks do not provide a worker executor.

**Improvement.** Session resides on the page, issuing work through its own ports.
A persistent core worker calls public `CoreEngine::handle`; core mutations remain
serialized. CAD preview and export use distinct workers/caches, with bounded
queues; scene preparation remains off-page. Browser adapters own Worker handles,
initialization, crash/close events and binary transfer. Session owns eligibility
and restart policy. The existing canvas GPU renderer remains on-page.

Start with separately built CAD/renderer packages and Rust adapters to their
generated WASM exports. CAD public entrypoints accept JS values; renderer's Rust
`wasm` module and scene input are private. Direct typed/native calls across these
crates are not already available. Do not re-export private members to make a
trial compile. Keep published JSON/serde payload meanings; scoped transport
identity adds session, worker epoch and job identity outside those contracts.
Borrow data within a worker; transfer owned binary buffers between workers and
page. A transferable JS buffer is not a zero-copy Rust vector conversion.

**Alternatives/tradeoffs.** Linking CAD into a worker may remove an inter-module
crossing but couples Dioxus's tooling to OCCT/WASI/ctor initialization and can
duplicate heavy binaries. Keep it an isolated comparison if P1 warrants it.
Putting engines on the page blocks interaction. Shared-memory threads require
new build/hosting constraints; they are unnecessary for the initial message
transport. A remote executor changes the accepted browser/offline contract.

**Preserved/change.** Keep exported APIs, error behavior, revision guards,
caller settlement and crash/reopen limitations. Internal envelope/transport
changes are deliberate adapter changes with compatibility tests, not document
schema changes. Engine crash recovery currently resets Undo history; do not
claim otherwise or blindly replay a commit whose outcome is unknown.

**Validation.** P1: worker readiness, failure/close, bounded queue, repeated
initialization, buffers/cache ownership and latest-job completion. Measure raw
and compressed initial/lazy/coexistence size, initialization time, crossings,
bytes and memory. P0 only resolves dependency versions, not worker execution.

### 4. Preview, cancellation and stale-result handling

**Observed problem.** Correctness currently relies on request IDs, revisions,
session/board/instance guards and cache protocols spread across TS owners.
A canceled future can leave external work running; dropping a late CAD delta
can desynchronize worker and consumer caches.

**Improvement.** Session owns preview/draft generation separately from committed
document revision. A gesture captures its scope/base revision and final sample;
cancel restores committed display without saving/history. Supersession cancels
queued jobs, signals active work and makes publication ineligible. Every waiter
settles on completion/cancel/failure/close. Apply valid cache deltas in protocol
order even when cancellation races, then reject stale presentation. A missing
delta base requires a full resynchronization rather than a guessed patch.
On project/session/board/instance changes, invalidate scoped state before
starting replacement work. Request and worker epochs distinguish restarted jobs.

**Alternatives/tradeoffs.** Revision-only rejection misses reopen of the same
ID/revision. UI-resource cancellation misses work already sent externally.
Terminating a CAD worker can stop stuck work but drops caches and affects all
its callers. Cooperative cancellation preserves warm caches, yet can occur
only between yielded operations, not mid synchronous OCCT construction.

**Preserved/change.** Preserve exact preview/commit and export-readiness semantics,
cache/delta correctness and committed-history grouping. Keep the last valid mesh
only with truthful pending/stale status; it cannot establish current export
readiness. No new approximate geometry/fabrication claims are proposed.

**Validation.** Replay supersession, delayed reply, cancel-after-completion,
scope switch, crash and disposal cases against existing tests and real workers.
P1/P2 must establish settlement and final-sample behavior under actual scheduling.

### 5. Renderer-independent scene contracts

**Observed problem.** Core emits semantic SceneDelta, CAD emits prepared meshes,
and the renderer's private payload contains view/theme/camera and mesh fields.
Making that private renderer input the shared domain contract would couple the
application to its current backend; making Dioxus nodes the scene would do the
same to the UI.

**Improvement.** Providers own a neutral scene/read-model boundary, with shared
Rust schemas placed in `contracts/rust` when needed. Describe stable IDs,
millimetres and existing orientation/transform conventions, contour/mesh data,
material intent, scoped revision/generation, fidelity/readiness and full/delta
base identity. Session owns selection/view overlays separately. Rendering
adapters consume these values to build SVG or current three-d input; browser/GPU
handles, Rust ownership handles and OCCT shapes never cross as scene data.

**Alternatives/tradeoffs.** Reusing the current payload verbatim is fast for one
adapter but leaks renderer state into providers. A new universal retained scene
graph or renderer rewrite introduces unnecessary scope. Prefer a small semantic
contract and adapters around existing capabilities; add fields only when an
actual consumer needs them. Preserve full snapshots until delta equivalence is
demonstrated rather than inventing a new protocol for every scene category.

**Preserved/change.** Preserve public SceneDelta/PreparedCaseAssemblyIR meaning,
units, precision, object identity, hit testing and saved geometry. Neutral
presentation contracts are new internal boundaries; existing private fields are
not made public implicitly. Current HtmlCanvasElement/WebGL2 interface remains.
OffscreenCanvas is a separate proposal, unnecessary for this milestone.

**Validation.** Compare semantic scenes, transforms, IDs and current/delta results
on the same fixtures, followed by paired 2D/3D visual and picking checks. P2 must
prove mount/resize/DPR/disposal against the actual renderer, not a mock canvas.

### 6. Durability and export boundaries

**Observed problem.** The inspected `useProjectSession.accept` saves a committed
reply before publishing it, but a failed save leaves CoreEngine advanced and
the UI on the earlier document. `schedule` catches the failure and permits later
work. CoreClient's restart snapshot is advanced by the engine reply before the
save acknowledgment. Open-project actions have explicit recovery, while ordinary
edit-save failure lacks an equivalent synchronization guarantee.

**Improvement.** Keep engine commit and durable save distinct. Session retains
the pending committed reply and publishes it only after the host reports actual
IndexedDB transaction completion. On failure, show failed/recovery status and
block subsequent mutations/exports; retry saving that same snapshot without
replaying the edit, retaining engine Undo history while it remains alive.
If recovery requires abandoning work or reopening a prior snapshot, require
an explicit choice and disclose resulting history loss. No automatic discard.
Crash recovery uses the session's acknowledged snapshot, with pending/unknown
outcomes reported rather than inferred from a transport cache.

Host owns IndexedDB transactions, keys/assets, file reads/downloads and object
URL cleanup. Rust archive/domain services validate and serialize. Preserve
`boardstudio-v2` database version 1, stores/keys, SHA-256 identities and active
project preference for eventual production compatibility; trial writes use a
separate namespace/origin. Preserve stored JS value representation, absent/null
meaning and supported unknown fields; default serde-to-JS conversion is not
automatically a compatible IndexedDB encoding.

Session captures a committed export context after queued saves complete.
Services own readiness checks, generator/CAD inputs, snapshot-bound artifacts,
archive paths and deterministic source semantics. Host supplies assets and
delivers final bytes only after scope/revision checks. Preserve existing export
workflows' explicit accepted-edit adoption, and keep export CAD from evicting or
depending accidentally on provisional preview state.

**Alternatives/tradeoffs.** Persist-first engine transactions require a larger
engine/history redesign. Publish-before-save changes observable durability and
export behavior. Reopening the old snapshot immediately after every failed save
would erase Undo. Fail-stop/retry is smaller and explicit, though it temporarily
gates editing during recovery. Storage libraries are optional adapter choices;
they do not replace tests of transaction completion, abort or representation.

**Preserved/change.** Save-before-publication, successful-save ordering, formats,
assets and captured export consistency remain. The ordinary save-failure
recovery state is an **intentional behavior change approved at scope review**. It does
not promise history survival across worker crashes. Accepted outline/readiness
blocking and valid-save availability remain; failed durability is a separate
operational recovery condition.

**Validation.** Characterize failure/drift first; any correction needs a failing
regression for the expected cause before implementation. P3 exercises real
transaction completion/abort, retry without a second edit/history entry, archive
round trip, asset preservation and offline reopen. Existing archive/export
and snapshot tests remain required, including stale delivery and cached/uncached
STEP equivalence. Inventory actual supported saved documents before cutover.

### 7. Remaining JS/TS, generators and transition

**Observed problem.** Executable bundled generator bodies feed geometry,
terminals/models and net/pose-dependent exports. Rust final serialization does
not replace that execution. Authored worker/storage/offline policy and TS CAD
facades also remain application code. Indefinite bridges would leave two
architectures and fail the accepted destination.

**Improvement.** Choose Rust implementations of the supported generator semantics,
retaining source/version IDs and parameter/net/side/mirror/reference behavior.
Imported KiCad footprint/model source is data and stays faithful. Use Rust-owned
host/CAD orchestration and offline policy. M1 may use existing embedded part
definitions without running generator authoring/export; this is a stated trial
omission, not a claim of generator completion. React remains the separate
working reference, not a second session store inside the Dioxus application.

**Alternatives/tradeoffs.** A constrained build-time translation may be useful
only after its supported language and parameter corpus are demonstrated; it is
not selected as a general transpiler. Precomputed defaults miss dynamic nets,
poses and parameters. Embedded/remote JS execution can be a temporary bridge,
but cannot establish full Rust. Same-page React/Dioxus islands reduce packaging
work yet complicate state, pointer ownership and disposal; use whole-workflow
trial entrypoints on separate origins first.

**Preserved/change.** Preserve supported saved source/version definitions, safe
paths, archive assets, footprint/net IDs, licensing/provenance and existing
rejection of user-supplied executable JS. No file-version change or implicit
generator conversion. Trial projects move through copied archives, since
separate origins cannot share IndexedDB directly. Production handoff later needs
one active writer and verified recovery; do not run both coordinators against
the same live document.

**Validation.** Before retaining any temporary application adapter, assign a
named accountable person, removal item, evidence-based exit condition and named
follow-up slice/deadline, as constraints require. Capability ownership: host
adapters/offline policy → `host-platform`; generator/CAD/export bridges →
`generators-cad-export`; React replacement → `dioxus-presentation`. This review
introduces no runtime bridge, so it creates no whole-migration removal backlog.
Later specifications must supply actual assignees/removal items before use.
Compare all supported generator inputs/poses/nets and semantic output, preserved
imports/assets and React archive round trips before retirement. Cutover requires
affected parity/resource gates, transferred regression coverage and a verified
rollback; trial acceptance alone authorizes no React deletion.

### 8. Dioxus version and presentation lifecycle

**Observed problem.** Prior 0.7.10 advice is a research candidate. Unversioned
Context7 examples can refer to `main`, and React-style effects do not establish
Dioxus dependency/cleanup behavior.

**Improvement.** Recheck release status and propose exact Dioxus/CLI 0.7.10 for
the browser trial. Select web/minimal/mounted capabilities deliberately, retaining Rust
1.98.0 and matching existing browser bindings. The isolated P0 graph resolves;
do not copy its lockfile into production. The eventual application records its
own lockfile/features and build-tested CLI/tool versions.

Use component signals for presentation, subscribed read models and explicitly
scoped resources for replaceable async presentation queries. Copying a Signal
handle does not clone its document. Reactive reads may restart resources;
component spawn cancellation/unmount only cancels the Rust future, not an
already dispatched worker job. Session coordination and explicit cancellation
remain authoritative. Mounted web elements can be downcast to the existing
canvas type; host retains listeners/observers/RAF and tears them down explicitly.
Do not move a save transaction into a short-lived component resource.
The tagged manifest and event implementation require `mounted` explicitly;
minimal plus web alone would not support mounted-element conversion.

**Alternatives/tradeoffs.** 0.8 alpha may contain useful changes, but it introduces
prerelease risk and requires a separate change to the current 0.7.x constraint.
An older 0.7 patch has no evidenced advantage. A single large document Signal
is convenient but may cause cloning/broad updates; subscription granularity must
be measured instead of chosen from framework familiarity. Framework cancellation
is useful for UI futures but insufficient as an executor/job protocol.

**Preserved/change.** Preserve existing CSS/tokens, SVG interaction language,
3D rendering, keyboard/focus/accessibility and responsive behavior. Exact patch
and component ownership are proposals. No design-system migration, server/SSR
requirement or source/dependency change is made by this document.

**Validation.** P1/P2 verify release WASM assets, static paths and real lifecycle
behavior. Paired parity, visual/a11y/responsive and performance evidence remains
required under constraints. The CLI and runtime are not tested by P0; it also
does not validate CAD linking or establish deployed bundle sizes.

## Versioned official-source verification

Checked on **2026-10-01**. Context7 resolved `/dioxuslabs/dioxus/v0.7.10` and
returned tagged signal/resource/spawn references. Some canvas/HTML snippets
still cited `main`; those were not accepted as version proof. Exact tagged
source was fetched directly instead. Several web-rendered API pages were
unavailable; tagged source and installed exact-version browser bindings supplied
the fallback. [Source URLs/hashes](../../.scratch/dioxus-scope-review/framework-evidence.json)
record successful retrieval, not execution evidence.

| Proposal / fact checked | Primary source and limits |
| --- | --- |
| Version selection | [Official latest-release endpoint](https://api.github.com/repos/DioxusLabs/dioxus/releases/latest) reports v0.7.10, non-prerelease, published 2026-07-30; [release](https://github.com/DioxusLabs/dioxus/releases/tag/v0.7.10), [release list](https://github.com/DioxusLabs/dioxus/releases) distinguishes 0.8 prereleases. Recheck before eventual provisioning. |
| Web/minimal features and Rust floor | [Dioxus 0.7.10 manifest](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/dioxus/Cargo.toml): web feature, minimal hook/signal/HTML/launch capabilities, Rust floor 1.83.0. [CLI manifest](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/cli/Cargo.toml) declares 1.82.0. Floors are compatible with 1.98.0; they do not prove build success. |
| Bindgen/tool provisioning | [Tagged workspace](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/Cargo.toml) contains semver requirements, not this application's exact resolutions. [CLI web build](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/cli/src/build/web.rs) detects application wasm-bindgen, verifies its tool and obtains esbuild. P0 resolves wasm-bindgen 0.2.129, js-sys/web-sys 0.3.106 and serde-wasm-bindgen 0.6.5. No CLI install/build was tested. |
| Signals/resources | [Tagged signals guide](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/signals/README.md), [resource documentation](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/hooks/docs/use_resource.md), [resource implementation](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/hooks/src/use_resource.rs) establish handle copying, reactive subscriptions and prior-task cancellation. They do not cancel external worker computation or acknowledge durable saves. |
| Task/drop lifecycle | [Tagged core lifecycle implementation](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/core/src/global_context.rs) implements scoped spawn/drop; these are UI-runtime tasks, not a browser-worker CPU executor. Session-owned background job lifetimes need separate adapters. |
| Mounted canvas/SVG | [Tagged web events](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/web/src/events/mod.rs) maps mounted data to web_sys::Element through WebEventExt; the `mounted` feature is required explicitly with minimal/web. [Tagged elements](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/html/src/elements.rs) defines canvas and namespaced SVG. Canvas downcast, capture, measurement and teardown remain compile/runtime probes. |
| Worker transport/transfer | [web-sys 0.3.106 Worker source](https://docs.rs/crate/web-sys/0.3.106/source/src/features/gen_Worker.rs), inspected from installed exact-version source, provides creation, transfer posting and termination; [HTML worker standard](https://html.spec.whatwg.org/multipage/workers.html) defines separate globals and messaging. [Current Worker constructor reference](https://developer.mozilla.org/en-US/docs/Web/API/Worker/Worker) describes module/classic options. The [official WASM worker example](https://wasm-bindgen.github.io/wasm-bindgen/examples/wasm-in-web-worker.html) illustrates initialization, but its dated browser-compatibility paragraph is not a current support matrix. |
| Durable completion | [IndexedDB transaction lifecycle](https://w3c.github.io/IndexedDB/#transaction-lifecycle) and [commit algorithm](https://w3c.github.io/IndexedDB/#commit-transaction) distinguish a successful request from completed/aborted transaction. IndexedDB completion preserves today's browser-local save contract; it is not a promise of physical-media flush. |
| Existing renderer/CAD limits | [Renderer source](../../renderer/src/wasm.rs) uses HtmlCanvasElement/WebGL2 and private scene input; [CAD exports/init](../../cad/wasm/src/lib.rs) expose JS-value entrypoints and ctor/WASI initialization; [CAD manifest](../../cad/wasm/Cargo.toml) pins Cadrum. These verified existing contracts justify reuse, not an undocumented direct-link/OffscreenCanvas proposal. |

## Review and evidence boundary

No earlier accepted ADR is superseded. The user approved this architecture scope
on 2026-10-01; P1–P3 and the representative milestone remain unexecuted.
P0's mounted-feature graph resolves online; its offline attempt fails against
the incomplete dependency cache. Record provisioning as unresolved rather than
claiming offline builds or changing established binding pins to fit the cache.
The historical [baseline results](../dioxus-context-baseline.md#baseline-validation)
retain their failures, restrictions and build provenance. Documentation checks
and P0 do not prove application parity, performance or cutover readiness.
The next bounded specification is [editor-session](../../SPEC-editor-session.md),
proposed for its own Phase 1 review. It consumes existing engine/service
contracts and records evidence needs; it does not implement those providers.
Do not generate a whole-migration implementation plan or promote probe code on
the strength of this ADR.
