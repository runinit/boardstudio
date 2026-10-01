# Capability map: full-Rust BoardStudio

Status: **scope approved by the user**, 2026-10-01. The user's “approved”
response accepts the six boundaries, dependency direction, packaging choices,
blocking resolutions and milestone/probe scope below. This completes Phase 0
of spec-driven-development; it does not establish runtime validation or trial
acceptance. Module specifications have their own review gates.

## Capability map

Dependencies point from consumer to provider. A dependency on a scene/request
contract does not require linking the provider's entire implementation.

| Stable module id | Owns | Depends on | Boundary and packaging justification |
| --- | --- | --- | --- |
| `document-engine` | Durable document semantics, validation, resolved design geometry, revisions, edit transactions, preview/commit and Undo/Redo | — | Retain `core/` and `contracts/rust/`. These capabilities already have a Rust authority and public request tests. Browser storage and editor selection do not belong to the document engine. |
| `editor-session` | Session identity, serialized commands, accepted snapshots/save acknowledgments, selection, 2D camera/navigation, gestures/drafts, generation and export coordination | `document-engine`, `generators-cad-export` through contracts and session-owned ports | Use one headless Rust application crate, with session and interaction modules inside it. This separates lifetimes and sequencing from Dioxus and browser APIs, allowing native characterization of failure and ordering. No crate per gesture or workspace. |
| `rendering` | Scene consumption, presentation geometry, 2D scene-to-SVG mapping, 3D camera/GPU state, picking, frame scheduling and disposal policy | Scene contracts supplied by `document-engine` and `generators-cad-export` | Retain `renderer/` for three-d/GPU algorithms; keep the SVG mapping as a module used by the web presentation. Different 2D/3D backends justify separate adapters, not a second document engine. Renderer-private payloads remain adapter details. |
| `host-platform` | Worker/executor adapters, browser events/canvas attachment, IndexedDB/file/asset I/O, clocks, browser resource cleanup and offline/static-host integration | `editor-session`, `document-engine`, `generators-cad-export`, `rendering` | Put web adapters and worker entrypoints in a web application package. They implement the ports defined by their consumers. Browser types and generated WASM imports stay here; a future native host can replace this layer without changing document/session semantics. |
| `generators-cad-export` | Parameter-dependent generator evaluation, mechanical preparation/CAD caches, model conversion, artifact construction and export-format semantics | `document-engine` | Keep existing core artifact/archive modules and `cad/wasm/`; add Rust generator/service modules only as needed. The row is a capability family with separate worker lifetimes, not a proposed umbrella crate. Heavy CAD build prerequisites justify its existing package boundary. |
| `dioxus-presentation` | Components, semantic controls, panel/form presentation drafts, focus display and subscriptions to read models | `editor-session`, `host-platform`, `rendering` | Share the web application package with host adapters, in separate modules. Only this layer depends on Dioxus. Its signals expose session read models; they do not become a second writable domain or session store. |

The existing shared Rust contract package remains a dependency leaf. Providers
own the meaning of their boundary contracts even when the types live there.
The engine and generation services never import the renderer or UI; session
ports never import their host implementations. Host composition supplies those
implementations. This removes a potential session/host dependency cycle.

Validation/build order for the first slice: existing document/contracts →
session ports and required generation/scene contracts → existing service and
renderer adapters → web host → Dioxus workflow. This is dependency order, not
an instruction to finish every capability before testing one workflow.

## Blocking decisions: approved resolutions

| Decision | Approved resolution |
| --- | --- |
| Meaning of full Rust | All application-owned runtime behavior ends in Rust: domain, session/interactions, rendering coordination, host policy, generators and export. Generated bindings/bootstrap, pinned framework/runtime dependencies, data/assets and development/test tooling may remain. Cadrum/OCCT's C++ kernel remains; this is an application-language destination, not a pure-Rust dependency tree. Authored JS policy remains application logic even when emitted by a build script. |
| Required platforms | Browser web application on a static host, desktop editing and existing compact layouts. First milestone acceptance targets the existing Chromium workflow, including root and `/boardstudio/` deployment paths and cached offline reopen. Desktop/mobile native applications and a server are outside this milestone. Firefox/WebKit support needs separate evidence before any broader browser-support claim. |
| State ownership | Engine: authoritative document/history/geometry. Session: operation queue, accepted document/scene, durable revision, scope, selection, 2D camera and transaction/generation identity. Dioxus: presentation-only drafts/focus/panels. Renderer: live 3D camera/GPU resources. Host: actual browser handles and I/O transactions. Copies of documents are immutable read models or captured jobs, not independent writable stores. |
| Worker/executor and transport | Keep a long-lived core worker with one CoreEngine; use its public typed `handle` inside the worker. Keep preview CAD, export CAD and scene preparation off the UI thread with separate bounded lifetimes. Session/UI and the existing HTML canvas renderer remain on the page thread. Rust owns scheduling; generated bootstrap initializes worker WASM. Preserve public payload encodings, adding scoped transport identity outside them; transfer owned binary buffers and measure remaining copies. |
| Preview, cancel and stale results | Preview never commits, saves or adds Undo. Session owns a captured gesture and final pointer sample; commit is one transaction. Supersession invalidates both requests and display eligibility; scope/revision/generation checks remain mandatory after every await. Cancel every caller to a terminal outcome. Cooperative CAD cancellation occurs between yielded operations, not inside synchronous OCCT. Consume valid raced cache deltas before rejecting their stale display. |
| Renderer-independent scenes | Treat existing semantic geometry and prepared mesh outputs as inputs to a neutral scene contract: stable object/body IDs, units/transforms, scope/revision/generation, full/delta identity, material intent and fidelity/readiness. Keep selection/camera overlays separate from durable geometry. No Dioxus elements, GPU handles, OCCT objects or browser types in the neutral contract. Adapt to current renderer entrypoints without widening private APIs. |
| Persistence and export | Preserve engine-commit → IndexedDB transaction completion → publish accepted snapshot ordering. Use an explicit recovery state on save failure: retain the pending committed snapshot, retry its save without replaying the edit, and gate later mutations/exports until resolved. Rust owns serialization/archive validation; host owns storage/file delivery. Exports use a captured committed scope and readiness checks, with separate preview/export caches. |
| Remaining JS/TS and dependencies | Retain React only as the working reference/fallback during staged replacement. Port supported generator semantics and CAD/browser orchestration to Rust; preserve imported KiCad/model source as data. Do not use a JS interpreter or frozen default footprints as proof of full-Rust generator parity. Every temporary application bridge needs an owner, removal item and tested exit condition before introduction. |
| Framework/toolchain | Select Dioxus **0.7.10** and matching **dioxus-cli 0.7.10** for the trial, with web/minimal plus mounted-element support and Rust 1.98.0 retained. Rechecked official releases on 2026-10-01: 0.7.10 is stable/latest; 0.8 is prerelease. The isolated dependency probe resolves Dioxus/core with current binding pins; CLI execution, WASM packaging and browser initialization are still gates. |
| Transition | Separate Dioxus trial origin and storage namespace, copied fixtures, explicit archive exchange with React. Replace complete vertical workflows with one authority per active document. Later production adoption requires paired behavior/format/resource gates and controlled writer handoff; do not run React and Dioxus coordinators as simultaneous writers. Rollback remains the working reference plus preserved source documents. |

Rationale, alternatives, preserved contracts, intentional changes and validation
needs for every change are in [accepted ADR 0003](docs/adr/0003-rust-application-ownership.md).
The save-failure recovery state is an approved intentional behavior change, not a
claim that the current implementation already provides it.

## First representative milestone

Use one isolated workflow spanning both accepted trial requirements:
open a copied supported project → select/drag/numeric edit with preview and
single-step Undo/Redo → save/archive/reload → change a case setting → exact
worker generation → inspect in the existing Rust 3D renderer → export STEP
from the accepted committed revision. Include Escape/pointercancel, stale
project/board results, save failure/retry, worker failure/close and teardown.

Use the existing [REVIUNG41 archive](docs/design/evidence/board-outlines/reviung41-original.boardstudio)
for saved-layout/outline behavior, plus a copied saved result from the
[Sofle gasket demo](app/src/demos/sofle.ts) for split-board/case workflows.
Keep IDs/parameters/assets faithful; compare actions and outputs with React.
Saving a Dioxus copy must round-trip back through React and the Rust archive
boundary. Do not reinterpret generator definitions while loading these fixtures.

This trial exercises embedded supported part definitions and case/STEP output;
dynamic generator authoring, complete PCB/firmware export and every workspace
are later scope. Loading without preserving their existing data is not enough.
This omission means the trial can assess the selected workflows but cannot
establish whole-application/full-Rust completion.

Before implementing the slice, define its characterization/oracles and paired
measurements under [CONSTRAINTS.md](CONSTRAINTS.md). Preserve frozen budgets;
startup/size/crossing thresholds require measured reference variance first.
The [assessment's failed and unavailable gates](docs/dioxus-context-baseline.md#baseline-validation)
remain unresolved; successful dependency resolution does not turn them green.

## Isolated integration prototypes

These are bounded investigations, not implementation tickets. Only P0 has run.
Each later probe gets its own throwaway branch/worktree or standalone scratch
package, copied assets and isolated storage. Its record names the question,
source revision, tool versions, observations and verdict. No prototype code,
lockfile or temporary bridge is promoted automatically.

| Probe | Question and scope | Evidence / stop condition |
| --- | --- | --- |
| P0 dependency resolution — completed | Resolve Dioxus 0.7.10's minimal web/mounted dependency graph with existing core and exact browser-binding pins. No application code, CAD link or CLI installation. | `cargo generate-lockfile` exits 0; all resolved Dioxus packages are 0.7.10 and binding versions remain 0.2.129 / 0.3.106 / 0.6.5. [Standalone probe and limitations](.scratch/dioxus-scope-review/prototypes/dependency-resolution/README.md). Compilation/runtime remain unverified. |
| P1 worker/CAD packaging — proposed | One Rust core worker and one Rust-managed existing CAD package; copied prepared-case input, buffer transfer, cancellation between bodies, crash/close. Test the pinned CLI's release assets at root and subpath. | Establish URLs/initialization, caller settlement, latest-result/cache correctness and copy/size costs. If separate CAD-package interop fails, stop and compare a separately isolated direct-link experiment; do not widen APIs or change the kernel silently. |
| P2 Dioxus/canvas lifecycle — proposed | One SVG gesture surface and one existing renderer canvas. Exercise mount, pointer capture/final sample/cancel, resize/DPR, reactive rerun and unmount. No workspace port. | Prove exact-version DOM downcast, keyboard/focus behavior, one-step transaction semantics, frame cleanup and no late publication/leaked GPU resources. |
| P3 durability/offline — proposed | Copied project/assets in a scratch IndexedDB with the existing schema, save failure/retry and archive exchange; cold/cached static load. Test Rust-owned service-worker policy through a generated loader separately. | Faithful round trip, atomic completion/abort, recovery, cache scope/update behavior and available cached assets. Cold unavailable lazy assets must fail honestly. If Rust service-worker integration fails, report it before proposing any temporary authored-JS policy exception. |

## Specification index and review gates

Phase 0 was approved on 2026-10-01, including Chromium as the first acceptance
target, retained non-Rust dependency allowances and save-failure recovery.
Runtime probes, detailed implementation plans, production code and cutover
remain subject to their own scoped evidence and review.

| Module id | Specification | Status |
| --- | --- | --- |
| `editor-session` | [Representative-milestone session contracts](SPEC-editor-session.md) | Authorized in advance; technical validation; existing providers |
| `document-engine` | [Existing provider contract](SPEC-document-engine.md) | M1 technical validation; no engine rewrite |
| `generators-cad-export` | [Preparation, CAD/cache and artifact contracts](SPEC-generators-cad-export.md) | M1 technical validation; broader generator parity remains outside M1 |
| `rendering` | [Neutral scene consumption and backend lifecycle](SPEC-rendering.md) | M1 technical validation; P2 runtime evidence pending |
| `host-platform` | [Browser executor, storage and platform contracts](SPEC-host-platform.md) | M1 technical validation; P1/P3 runtime evidence pending |
| `dioxus-presentation` | [Presentation/subscription contracts](SPEC-dioxus-presentation.md) | M1 technical validation; paired browser evidence pending |

At the original scope review only editor-session was specified. No whole-migration
implementation plan or task backlog follows from scope approval.

Continuation on 2026-10-01 completes the five remaining specifications for the
same M1 scope. The user's **“Assume everything is approved in advance”**
authorizes phase continuation for defined bounded work without routine approval
pauses. All six specs receive technical cross-contract validation; neither
advance approval nor a spec substitutes for runtime/parity evidence. The current
bounded task set is [P1-r1](tasks/plan.md), recorded in [the run ledger](docs/migration/RUN.md).

Accepted inputs remain the [Wayfinder map](.scratch/dioxus-browser-trial/map.md),
[context assessment](docs/dioxus-context-baseline.md) and existing accepted ADRs.
Their unresolved trial tickets are not closed by scope approval.
