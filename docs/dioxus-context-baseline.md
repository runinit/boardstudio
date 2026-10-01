# Full-Rust/Dioxus context and baseline

Assessed on 2026-09-30, with check timestamps recorded in UTC. This is a context
reconciliation and baseline assessment. It authorizes no migration implementation
and records no new accepted architecture decision.

## Checkout and preservation

| Item | Verified state |
| --- | --- |
| Source repository | `/home/chris/01_Projects/ts-boardstudio2` |
| Source branch / latest local committed dev | `dev`, `96dd51d3e790c28f5554a8c9888a147c8814e2a7` |
| Assessment branch | `docs/dioxus-context-baseline`, based directly on that commit |
| Assessment worktree | `/home/chris/01_Projects/ts-boardstudio2/.scratch/worktrees/dioxus-context-baseline` |
| Assessment Git metadata | `.scratch/git/dioxus-context-baseline.git` in the source repository |
| Published-main comparison | Local `main` and cached `origin/main` are `fd991be3`; cached `origin/dev` is `30d2eb25`. These are ancestors of the assessed dev commit. No fetch or remote-freshness claim. |
| Original dirty state | 125 porcelain entries at the isolation inventory; concurrent test cleanup and encoder/VIK work remain in the original checkout. Counts describe a changing overlay, not a commit. |
| Assessment changes | Documentation and preserved decision evidence, uncommitted. Production files and dependency manifests remain at the base commit. |

The source `.git` directory is read-only in this session. A separate bare copy
of its Git metadata owns the new branch and linked worktree; the original
repository's worktree list therefore does not list this new worktree. The copy
uses the repository filesystem because `/tmp` lacked room for the Git objects.
No existing checkout was reset, cleaned, deleted, recreated or switched. No
commit, merge, history rewrite or push was performed.

The [preservation manifest](dioxus-context-baseline-evidence.json) records hashes
for 50 copied Wayfinder/encoder decision and research artifacts. The copies were
verified byte-for-byte. Both Dioxus research bundles verify against the base:
`b1eb69c1` for platform research and `8e9ce913` for Rust integration research.
The original research branch names still point at `96dd51d3`; the bundles retain
the actual research commits. Research completion is not prototype completion.

This session's assessment tracking was moved from the dirty source checkout to
this worktree. Only its own PLAN/TODO additions were removed there, after saving
the original documents and recreating the tracking here. The previously written
[migration constraints](../CONSTRAINTS.md) were copied unchanged.

The original metadata lists 26 worktrees: nine directories present and 17 marked
prunable at this inventory. Missing directories are recorded, not repaired.
Full paths, HEADs and dirty status are in the evidence file.

| Existing checkout / branch | HEAD | Dirty entries |
| --- | --- | ---: |
| Main checkout, `dev` | `96dd51d3` | 125 |
| `3e8a/ts-boardstudio2`, `codex/cadrum-cad-backend` | `0ca5e3f1` | 64 |
| `boardstudio-electrical-refactor` | `fd2b1fd6` | 0 |
| `boardstudio-mechanical-refactor` | `ac88ad21` | 14 |
| `boardstudio-outline-wip/ts-boardstudio2` | `6abf3eb9` | 82 |
| `boardstudio-performance/ts-boardstudio2` | `6abf3eb9` | 4 |
| `boardstudio-ui-integration` | `30d2eb25` | 0 |
| `f357/ts-boardstudio2`, detached | `49f88a36` | 43 |
| `gasket-case-redesign/ts-boardstudio2` | `6abf3eb9` | 0 |

Protected historical Rust work includes `codex/rust-matrix-core` and
`codex/rust-archive-core` at `7703d273`, `codex/rust-matrix-archives` at `b7b7c3df`,
and `codex/rearchitecture` at `f7b6291c`; these are ancestors of dev. The preserved
outline recovery commit `698706c2` is not an ancestor and remains recovery
evidence. An ancestry result is not permission to delete a branch or worktree.
See [development worktrees](development-worktrees.md),
[outline recovery](outline-recovery-integration.md) and the
[cleanup inventory](repository-cleanup.md). Removed historical migration reports
are indexed there; their absence does not mean Rust functionality must be rebuilt.

## Context and workflow

Read sources include the root agent guidance, [constraints](../CONSTRAINTS.md),
[product](../PRODUCT.md), [domain vocabulary](../CONTEXT.md),
[plan](../PLAN.md), [tasks](../TODO.md), changelog, architecture, ADRs, design
specifications, migration research and the local Wayfinder tickets. The original
checkout's project/user memory was consulted as a locator. Its pending-publication
and performance notes are stale in places; verified artifacts and current
instructions take precedence.

The installed package is **addyosmani/agent-skills 0.6.11**, with 25 skills.
`context-engineering` was read first; Git workflow, documentation/ADRs and the
existing task-management workflow support this assessment. All nine distinct
referenced local Markdown resources exist. The skill inventory and exact
installation paths are in the evidence file. Dioxus CLI is absent; this checkout
has no Dioxus application crate or runnable Dioxus trial.

Preserve the [Wayfinder map](../.scratch/dioxus-browser-trial/map.md), its six
tickets and research as provenance. Their historical routing to Matt Pocock's
skills is superseded by the user's instruction to use Addy's skills going
forward. The map's product decisions remain usable. No ticket was rewritten or
closed to manufacture agreement.

## Current architecture

BoardStudio already has substantial Rust domain ownership. The remaining work
includes application coordination and runtime policy, beyond replacing React
markup. [Current architecture](architecture.md) describes the existing contracts;
the following distinguishes authoritative engines from their current hosts.

| Area | Current owner and public boundary |
| --- | --- |
| Document, edits and history | [CoreEngine](../core/src/lib.rs) owns `ProjectDoc`, revision checks, preview/commit semantics, Undo/Redo stacks and resolved geometry. `handle(CoreRequest) -> CoreReply` is public Rust; `request(&str) -> String` is its JSON/WASM facade. Rust also owns electrical/mechanical resolution, keymap/firmware semantics and bounded [Rhai geometry scripts](../core/src/script.rs). |
| Session and durability | [useProjectSession](../app/src/useProjectSession.ts) owns the serialized operation queue, committed UI snapshots, project-session identity, persistence acknowledgments and worker lifecycle. A preview updates the displayed scene; a committed reply is saved before publishing the accepted React document/scene. Engine mutation and browser save are separate operations. |
| Editor and UI state | React feature owners hold selection, navigation/cameras, panel and form drafts, placement gestures and async presentation. [Canvas interactions](../app/src/ui/createCanvasInteractions.ts) retain captured transaction identity, the final pointer sample, cancellation and rollback semantics. Some TS application planning, including placement, keycap resize and terminal remapping, remains. These owners describe current behavior, not a mandated future crate structure. |
| Workers and transport | [CoreClient](../app/src/CoreClient.ts), [CaseClient](../app/src/CaseClient.ts) and [ExportClient](../app/src/ExportClient.ts) manage TS worker requests, failures, cancellation and caller settlement. [Core worker](../app/src/core.worker.ts) owns the Rust engine and serializes each request/reply through JSON. Case/export/scene workers use request IDs, revisions/session guards, caches and transferable buffers; preview and export CAD have separate lifetimes. |
| Rendering | 2D editing is React/SVG. The [Rust three-d renderer](../renderer/src/wasm.rs) owns GPU scene/rendering operations on an `HtmlCanvasElement`. [renderClient](../app/src/renderClient.ts) owns browser mount/resize/DPR, RAF and pointer lifecycle. [scene.worker](../app/src/scene.worker.ts) prepares Rust scene data off the page thread; the GPU renderer stays on the page thread. No current OffscreenCanvas renderer interface was found. |
| CAD and models | [cad/wasm](../cad/wasm/src/lib.rs) owns Cadrum/OCCT solid construction, mesh preparation and STEP operations, including C++ initialization. It exports the existing WASM functions; typed construction/model helpers remain private. [cad/src](../cad/src/index.ts) still owns TS loading, validation, scheduling/yielding, cancellation and bridge caches. |
| Generators and export | [Ergogen runtime](../ergogen/src/index.ts) executes trusted bundled JS generator bodies and parameter/net/pose semantics during editing and export. [export.worker](../app/src/export.worker.ts) allocates generator nets and adapts results. Rust [artifact APIs](../core/src/artifact/mod.rs) prepare/finalize and serialize KiCad output; Rust [archives](../core/src/archive.rs) package project/assets. The artifact typed dispatcher is private; its public entrypoint is JSON. |
| Persistence and offline | [storage](../app/src/storage.ts) owns IndexedDB `boardstudio-v2`, database version 1, project IDs, SHA-256 asset keys and the active-project preference. [write-sw](../app/scripts/write-sw.mjs) emits the authored JS service-worker cache/update policy. Its generation by a build script does not make that policy merely generated WASM binding glue. |

Fourteen previously inspected session/worker/rendering/CAD/storage source files
were hash-checked and match this committed baseline, allowing that inspection to
be reused. Changed core models and interfaces were read from this worktree.
The original checkout's new encoder/module fields and contracts are active WIP,
not part of the committed architecture or proof of shipped support.

Remaining runtime JS/TS includes React/React DOM 19.3.0, the workspace
CAD/Ergogen/KiCad facades, browser/session/editor coordination, workers, generator
execution, import/export adapters, storage and service-worker policy.
`libcascade` 3.0.2 is a CAD development/test dependency used as an independent
oracle, not the production CAD backend. Generated wasm-bindgen glue and Node,
Vite, pnpm and test scripts are distinct from maintained application logic.

## Reusable decision index

These classifications preserve existing sources and distinguish accepted
behavior from proposed implementation. Accepted ADRs are not reopened.

| Classification | Decision or evidence | Existing source / implication |
| --- | --- | --- |
| Accepted, applicable | Browser-first Dioxus 0.7.x, maintained application logic in Rust, generated bindings and development/test tooling allowed | [Wayfinder destination](../.scratch/dioxus-browser-trial/map.md), [constraints](../CONSTRAINTS.md). Current effort includes application rearchitecture and incremental vertical slices. |
| Accepted, applicable | Rust engine authority; behavior, saved documents, wire formats, gestures, visual/accessibility/responsive behavior and retirement gates | [Constraints](../CONSTRAINTS.md), [architecture](architecture.md). Reuse working Rust; preserve observable contracts while choosing new ownership. |
| Accepted, applicable | Edited outlines stay fixed; accepted refinements remain linked with explicit Freeze behavior | [ADR 0001](adr/0001-fixed-edited-outlines.md), [ADR 0002](adr/0002-linked-outline-refinements.md), both `accepted`. These record behavior, not completion of every planned stage. |
| Accepted, applicable | Confirmed outline choices, authored cutouts, version ownership and scoped export blocking | [17-decision outline plan](design/board-outline-plan.md), [recovery evidence](outline-recovery-integration.md). Stage 1 evidence does not establish all later stages as shipped. |
| Accepted, applicable | Linked mirrored layouts and current navigation/setup/focus semantics | [Mirrored layout](design/mirrored-layout-pair.md), [workflow plan](design/workflow-overhaul.md), [product](../PRODUCT.md). Preserve these in parity fixtures. |
| Accepted, applicable | Typed keymap edits, Rust defaults and firmware generation through current APIs | [Keycaps/keymap](keycaps-and-keymap.md), [architecture](architecture.md). Physical fit, firmware compilation and device acceptance remain separate evidence gaps. |
| Accepted, applicable | Production Cadrum/OCCT adapter and existing Rust renderer | [CAD decision record](cad-kernel-options.md), [renderer source](../renderer/src/wasm.rs). Full-Rust application scope is not evidence for reopening the third-party kernel choice. |
| Accepted, applicable | Frozen performance controls, budgets and ownership/cache protections | [Performance baseline](performance-baseline.md), [generation performance](generation-performance.md). The separate 200 ms exact gasket target remains unmet. |
| Accepted scope; implementation pending | Approved encoder/VIK behaviors and eight-ticket breakdown | [Specification](../.scratch/encoder-vik-modules/spec.md), [ticket index](../.scratch/encoder-vik-modules-implementation/README.md). Concurrent code exists; the index's historical pending status is not a current completion audit. |
| Provisional | Exact Dioxus patch/CLI pin, worker packaging and Rust interface integration choices | [Platform research](../.scratch/dioxus-browser-trial/research/browser-platform.md), [integration research](../.scratch/dioxus-browser-trial/research/rust-integration.md). Resolved research tickets do not approve an architecture. |
| Provisional | Unimplemented workbench design details and performance/kernel experiments | [Workbench design](design/workbench-redesign.md), [optimization status](generation-optimization-status.md), [kernel research](pure-rust-cad-assessment.md). Retain intent/evidence without assuming implementation or selection. |
| Provisional | Encoder/VIK synthesis defaults and per-part qualification | [Specification defaults and gates](../.scratch/encoder-vik-modules/spec.md). Ready-for-agent scope does not turn unanswered historical choices into human agreement or establish hardware fit. |
| Superseded by current scope | Earlier Leptos-CSR-first prototype recommendation; historical Wayfinder skill routing | [Framework comparison](frontend-framework-comparison.md), [map](../.scratch/dioxus-browser-trial/map.md). The user selected Dioxus and Addy's skills. Preserve useful browser/offline/testing cautions; do not restart framework selection. |
| Invalidated by source evidence | CAD research prose identifying Three.js as current mesh consumer and libcascade as current production integration | [Historical CAD prose](cad-kernel-options.md), [renderer](../renderer/src/wasm.rs), [CAD package](../cad/package.json). Current rendering is Rust three-d and libcascade is dev-only. |
| Insufficient for expanded scope | A UI substitution or temporary TS client bridge as the complete migration design | [Framework comparison](frontend-framework-comparison.md), [current session scope](../PLAN.md). Temporary coexistence remains useful; the final ownership of session, workers, generators and offline policy needs a Rust architecture. |
| Unresolved | Application ownership/acceptance contract, runnable layout and case trials, then staged migration route | [03 architecture](../.scratch/dioxus-browser-trial/issues/03-trial-architecture-acceptance.md), [04 layout](../.scratch/dioxus-browser-trial/issues/04-layout-trial.md), [05 case](../.scratch/dioxus-browser-trial/issues/05-case-trial.md), [06 route](../.scratch/dioxus-browser-trial/issues/06-staged-migration-route.md). None is completed by this assessment. |

The historical statement that there were no released older projects is not
permission to discard saved documents. The baseline accepts `boardstudio/v2`
and has no v1 importer; a compatibility corpus must identify actual existing
documents and currently loading shapes instead of assuming that corpus is empty.

Dioxus 0.7.10 remains a candidate, not an installed or accepted exact pin.
Its [official versioned reference](https://docs.rs/dioxus/0.7.10/dioxus/) and
[release](https://github.com/DioxusLabs/dioxus/releases/tag/v0.7.10) were checked
again. Framework APIs must be verified against the version selected later;
React ownership/effect patterns are not an implementation specification.

## Baseline validation

The tested production sources are commit
`96dd51d3e790c28f5554a8c9888a147c8814e2a7`, with documentation additions only.
Each completed check records command, working directory, UTC timestamps,
exit code, HEAD and before/after hashes of every tracked file in
[validation evidence](dioxus-context-baseline-evidence.json). No completed
command changed a tracked file. [Raw logs and manifests](../../../context-reconciliation-20260930/)
remain outside the assessed worktree.

Environment: Linux 7.2.8 CachyOS x86-64; Node 26.10.0; pnpm 12.6.0;
Rust/Cargo 1.98.0; rustfmt 1.9.0; Clippy 0.1.98; wasm-pack 0.15.0;
Chromium 153.0.8010.52; KiCad CLI 10.0.6. Native and
`wasm32-unknown-unknown` targets are installed. Checks used offline Cargo and
CI's `NODE_OPTIONS=--no-experimental-webstorage`. CI also covers Node 24.21.0;
that matrix entry was not exercised here.

Installed packages were reused through local links, with workspace package links
resolving inside this worktree. No dependency installation was completed.
`pnpm run` initially attempted automatic installation and failed on restricted
network access; those wrapper failures are retained in the evidence. Further
wrapper runs were stopped and the existing commands were invoked directly.
Dependency manifests and lockfiles were verified unchanged.

| Existing gate / command | Current result |
| --- | --- |
| `pnpm run check:repo` implementation: `node --test scripts/repo-check.test.mjs`, then `node scripts/repo-check.mjs` | Pass; scanner fixtures and 553 authored / 369 production-reachable modules. |
| `cargo fmt --manifest-path <crate>/Cargo.toml --all -- --check`, for core, renderer, contracts/rust, cad/wasm | Core and renderer fail existing formatting; contracts and CAD pass. No formatting applied. |
| `cargo clippy --manifest-path <crate>/Cargo.toml --locked --all-targets -- -D warnings` | Core, renderer and CAD fail existing lints, including manual containment, test-module ordering and chunk handling. Standalone contracts is blocked because it has no Cargo.lock and `--locked` forbids creating one. |
| `cargo test --manifest-path core/Cargo.toml --locked` | Pass: 334 passed, six existing ignored. |
| `cargo test --manifest-path renderer/Cargo.toml --locked` | Pass: 26 passed. |
| Native CAD tests with cached pinned OCCT and `--locked` | Pass: 23 passed, four existing ignored. The initial run lacked the `prepare_case` fixture driver; building that existing prerequisite, then rerunning, passed. No source repair. |
| Contract freshness/runtime scripts behind `check:contracts` and `test:contracts` | Pass via direct Node invocation. Generation used check mode and temporary output only. |
| App and CAD `tsc --noEmit` | Pass via installed TypeScript directly. |
| Ergogen package test stages | Integrity fixture test fails with `spawnSync /usr/bin/node EPERM`. Independent integrity verification, catalogue freshness, catalogue assertions and runtime tests pass for 36 modules/generators and front/back poses. Full package gate does not pass. |
| App `vitest run` | 460 passed, nine failed, 469 total. All nine failures are in project-archive tests, with native cargo subprocess `EPERM` preventing the expected operation/errors. Preserve failures; no assertions or fixtures changed. |
| Core and renderer release WASM compilation | Pass with `cargo build --locked --release --target wasm32-unknown-unknown`. Compilation alone does not pass packaging. |
| Core/renderer wasm-pack packaging | Blocked: no matching local wasm-bindgen tool. Used `--mode no-install` to prevent tool installation. Existing locks use wasm-bindgen 0.2.129 / web-sys 0.3.106. |
| Frontend build (`tsc`, Vite, `write-sw`) | All three stages pass directly. Inputs include separately hashed, pre-existing core/CAD/renderer generated packages; this is frontend build evidence, not a fresh complete WASM build. Existing large-chunk warning retained. |
| `check:boundaries` implementation | Interrupted at the recorded 180-second limit after native drivers built; no parity result emitted. Not a pass. Cached WASM provenance also prevents claiming a fresh full-build baseline. |
| Production browser suite | Fails before tests start. A separate-port diagnostic confirms `connect EPERM 127.0.0.1:4386`; the preview-server setup is unavailable in this sandbox. No browser assertion ran. |

Unavailable/not completed: CAD WASM/container build and combined CAD package
gate; KiCad Node package suite; development-server/Pages browser checks; Dioxus
build/runtime checks; visual/a11y/responsive parity; and comparable performance
sessions/soak. The earlier KiCad runner stalled after its native build and was
stopped; it is not treated as passing or rerun through the same blocked path.
Container tooling cannot use its required writable runtime state here. Dioxus
has no implementation to test. No dedicated Dioxus/React parity, screenshot
baseline or automated accessibility gate is configured by this documentation.

Root `build`/`check`/`test:perf` were not claimed as passing. Their composite
paths include blocked prerequisites and catalogue generation that writes source
output; this session used the available stages and read-only catalogue check.
Existing performance reports/budgets remain evidence from their recorded runs,
not fresh startup, latency, memory or size acceptance from this session.

Five inherited PLAN links to ignored historical measurement files are absent
from both this worktree and the original checkout. Their exact paths are recorded
in the evidence file. Maintained reports and tracked controls remain available;
the raw evidence behind those five links needs locating before reuse. The links
and baselines were not rewritten to make the documentation appear complete.

Earlier checks against the dirty source overlay are diagnostic history, not the
committed baseline. In particular, its mounting-datum failure and encoder
catalogue/asset failures are not reproduced by this clean core baseline. The
earlier temporary raw-log directory is no longer available; the new durable
evidence captures this worktree's checks. No unrelated failure was repaired and
no baseline was updated to obtain a pass.

## Highest-risk unresolved questions

1. **Application authority and durability.** Where does the Rust application
   coordinator live, and how does it own committed read models, drafts, session
   identity, save acknowledgments and failure recovery around the existing core?
   Preserve one-step Undo and the distinction between engine commit, durable
   save and UI acceptance. Do not replace these with assumed React-to-Signal
   equivalences.
2. **Worker and crate boundaries.** How are Rust worker entrypoints, initialization,
   cancellation, request settlement and transferable-buffer ownership packaged
   with Dioxus? Core has a public typed handler; artifact/CAD typed internals do
   not. Reuse public interfaces without casually widening visibility, moving
   heavy CPU work onto the page thread or multiplying WASM crossings.
3. **Generator semantics and saved compatibility.** Which Rust execution route
   preserves bundled generator parameters, net identity, mirroring, footprint
   geometry and saved `bundled-1` definitions? Existing Rust Rhai geometry scripts
   are a separate working feature. An embedded JS interpreter or indefinite
   handwritten TS generator runtime does not meet the agreed destination.
4. **Persistence and offline policy.** What Rust-owned adapters preserve the
   current IndexedDB/asset/archive contract and durable ordering? Which explicit
   browser bootstrap boundary handles service-worker install/update, static
   subpaths and lazy worker/kernel assets? An offline-capable architecture needs
   evidence for update/reload recovery, not just a static bundle.
5. **Parity and performance evidence.** Which copied saved documents and complete
   edit/Undo/save/reload/case/export workflows form the compatibility corpus?
   How are visual, keyboard/focus, responsive and accessibility checks enforced?
   How will independent WASM instances, copies, caches and startup sizes be
   measured against the retained controls in a runnable environment?
6. **Concurrent contract evolution.** When encoder/VIK work is committed, which
   model, electrical, firmware, case and catalogue interfaces change the Rust
   application boundary? Keep its qualification gates and approved scope while
   refreshing the baseline; do not silently migrate a stale document model.

## Next recommended decision

Agree the **Rust application coordinator's ownership and durability contract**
before implementation: its public boundary to Dioxus and workers, authoritative
state versus view drafts, commit/save/Undo ordering, cancellation/recovery and
explicit browser adapters. Include representative saved-document and gesture
acceptance fixtures. This provides a seam for vertical migration without
mechanically copying React controllers or reopening settled core/CAD behavior.

Use the existing [03 architecture/acceptance ticket](../.scratch/dioxus-browser-trial/issues/03-trial-architecture-acceptance.md)
as linked provenance under Addy's specification/API/ADR workflow; extend the
decision brief to the full application scope before choosing prototype details.
The decision remains open. Layout/case trials and migration implementation are
subsequent work, not actions taken in this session.
