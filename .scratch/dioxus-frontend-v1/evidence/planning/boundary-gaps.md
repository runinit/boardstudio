# Keycap CAD and export boundary follow-up

## Finding

**The keycap geometry/STEP engine already exists as a callable Rust-to-WASM export; the Dioxus host adapter and request path do not.** The likely smallest reuse is to route existing `KeycapSpec` data through the already-packaged Cadrum WASM module from the existing Dioxus CAD worker. This needs private web-host/worker integration and payload validation, not a new core/CAD algorithm or new public wasm-bindgen function. This is a source-based feasibility conclusion, not an implementation or tested-call claim.

The React `CaseClient` and its Vite worker are not directly callable from the Dioxus Rust page with the current build. The Dioxus release copies the separately built Rust CAD worker and generated CAD WASM package into static assets; it does not bundle the React `app/src/case.worker.ts` or `app/src/CaseClient.ts` into that page. Reuse the engine/module through Dioxus's own private host path rather than maintaining two app runtimes or porting keycap solid generation.

## Existing path

1. The local Keycaps control in `app/src/ui/createKeymapWorkspace.tsx` calls the shared export callback with `keycaps-step`; the preview adapter in `app/src/assemblyPreview.ts` also resolves specs and asks CAD for preview geometry.
2. `app/src/exports/keycaps.ts` requests Core `resolve-keycaps` for the active document/board with `cases: null`; it rejects resolver errors, error findings, and empty spec sets, then calls `CaseClient.keycaps(revision, specs, true)`, checks the result revision, and emits `<document-name>-keycaps.step` as `model/step`.
3. `app/src/CaseClient.ts:65-80` sends `{kind:'keycaps', revision, specs, exportStep}` to a dedicated worker and rejects a stale revision or an aborted/superseded response. `app/src/case.worker.ts:34-38` dispatches to `buildKeycaps` and transfers STEP and body mesh buffers.
4. `cad/src/index.ts:50-63` is a JS wrapper over the Cadrum WASM kernel. Preview mode builds groups of eight and yields between groups so cancellation can be observed. Export mode calls the kernel once with `export: true` and returns STEP plus meshes.
5. `cad/wasm/src/model/keycaps.rs:22-25` already exports `#[wasm_bindgen] build_keycaps(JsValue)`. It accepts `{revision, specs, export}` using existing `KeycapSpec`; output contains revision, cap meshes named `keycap:<stable-spec-id>`, separate legend inlay meshes `keycap-legend:<stable-spec-id>`, and, in export mode, STEP from the same placed solids. It validates dimensions/profile geometry and caps requests at 4096 specs. `cad/wasm/src/lib.rs` already re-exports it; no new function or public CAD/core request is needed.
6. The Dioxus build assembles `m1_cad_worker` and copies `cad/wasm/pkg` into the application assets (`scripts/build-m1.py:51-74`). `web/src/cad_worker.rs` already dynamically imports that module, invokes named WASM exports by `Reflect`/`Function`, yields to receive cancel messages, transfers mesh/STEP buffers, and has a correlated worker request lifecycle.
7. The missing bridge is between `CoreReply::KeycapsResolved`/`KeycapSpec` and the Dioxus host CAD worker. Rust Core already implements `ResolveKeycaps`, and `KeycapSpec` is in the shared generated contract. However, `web/src/cad_jobs.rs::CadOperation` has only Preview, Exact, ExportStep, and ReadStep; its `CadRequest` has `prepared` case IR or STEP bytes, but no keycap specs. The private worker `WireRequest` mirrors that case-only typed payload; `run_request` dispatches only the four current operations. `Runtime` owns one case CAD worker and has no keycap method/result consumer.

## Smallest reuse option and boundary

Add a **crate-private keycap request path in the Dioxus CAD host** that forwards the existing revision and `KeycapSpec[]` payload to the already-built CAD module's `build_keycaps` export. It should preserve the existing `CadSnapshotIdentity` checks (session epoch, document, board, instance, snapshot token, revision), echo/correlate request and job IDs, validate result revision and payload, and map returned bodies/STEP bytes to the existing viewer/download consumers. Use the existing accepted Core `ResolveKeycaps` result as input; don't calculate specs again in the CAD worker.

Avoid treating a new public variant on `boardstudio_web::cad_jobs::CadOperation` / `CadRequest` as automatically approved: `web/src/lib.rs` publicly exposes those Rust modules and the types are `pub`, even though the worker wire is not a generated end-user contract. Prefer a private host/worker command and crate-private Runtime method, or get explicit approval if implementation genuinely requires changing externally visible crate types. The existing Cadrum WASM export itself is already public and packaged; neither a new engine export nor CoreRequest/CoreReply/schema change is indicated by this gap.

Cancellation needs a deliberate port, not a blanket claim of worker preemption. The current Rust CAD worker can observe cancellation between case bodies and after synchronous Cadrum calls, but a synchronous `build_keycaps` invocation cannot be interrupted mid-kernel call. React preview improves this by invoking the existing WASM function in chunks of eight with worker yields; STEP export is one synchronous invocation. A Dioxus adapter can preserve the chunk/yield behavior for 3D preview and suppress results after cancellation/scope change. For STEP, it can stop queued work and reject late results, but should not promise mid-kernel cancellation unless the engine gains a cooperative mechanism (not authorized here).

## F6/F7 dependencies to update

- **F6C.5 / Keycaps consumer:** existing Keycap STEP and generated 3D preview currently cannot complete through the Dioxus CAD facade, despite the engine being available. Split or gate the CAD-service dependency explicitly. `ResolveKeycaps` fit findings and 2D Keycaps fields/canvas can proceed on fixtures; 3D cap/legend meshes, Retry/revision behavior, and `Export keycap STEP` completion depend on this adapter. STEP keeps the source's no-spec/error/finding guards and `model/step` artifact delivery. Keycap STEP export intentionally resolves with `cases: null`; do not accidentally change its current Case-contact semantics while plugging the adapter.
- **F7.1:** add a short early feasibility proof that the existing generated Cadrum module's `build_keycaps` export can be called through a crate-private Dioxus worker path and produces contract-valid `CaseResult` body/STEP payloads for the same fixture specs. Inspect cancellation/yield behavior and public Rust visibility before choosing the wrapper shape. This is the early keycap-provider gate only; it should not block Case forms, ordinary common viewer fixture work, or M1 case generation.
- **F7.3:** consume the adapter's cap and legend body meshes through the one shared viewer. Preserve stable keycap/legend IDs, colors, scene/snapshot scope, visibility and retries; do not add a Keycaps-specific viewer. F7's generic viewer can continue with fixture meshes until the keycap provider join is ready.
- **F8.4:** the Export-route firmware work can proceed with Core provider fixtures; the local keycap STEP handoff depends on F6C.5's CAD adapter. Do not say Rust Runtime already has a Keycap STEP provider. Reuse the same adapter/action as F6; no duplicate output path.

Suggested status label: **Existing engine/provider function; missing Dioxus private host adapter.** This is not a missing CAD algorithm or an established public-contract gap. If feasibility shows the existing module cannot be loaded/invoked or the private wrapper cannot return/validate required data, record the exact blocker for a separate contract decision; do not drop keycap preview/export or ship a permanent placeholder.

## Session-owned export commits: implemented path and remaining proof

The earlier planning diagnosis was correct for the pre-F8.3a Session, but
is superseded by the mounted full/draft KiCad adapter. `application::Event::ExportCommit`
and `ExportCommitRequest::{ApplyElectrical, ProtectElectricalHandoff}` now carry
the child operation ID plus the owning export's operation ID, captured snapshot
token and `Scope` (`application/src/session.rs`, lines 266–285, 875–896). Session
rejects a stale owner at submission and again when pumping the queued intent,
requires the plan's accepted document revision, and routes the request through
the ordinary Core queue (`session.rs`, lines 1228–1237, 1392–1409, 1475–1492).

The active Core request and pending persistence retain that exact owner. On a
successful save, Session cancels other exports, installs the new accepted
snapshot, and advances only the matching export's token when operation, scope
and old token all match (`session.rs`, lines 1540–1549, 1568–1600). Failed
persistence does not install or adopt a snapshot; unrelated accepted commits,
navigation, close, or stale scope/token continue to cancel the operation. This
uses normal Core application, persistence, accepted-state, and undo/history
ownership; it does not introduce another document writer or Core protocol.

`Runtime::pcb_handoff_bytes` consumes this seam in the required order: resolve
the selected-board electrical plan, apply only when not already present, adopt
the accepted result and resolve again, build/package from that accepted
snapshot, then protect only after packaging succeeds. It captures session,
document, board/instance scope, token/revision, executor epoch and Core worker
identity and rechecks them around awaits and before delivery
(`web/src/runtime.rs`, lines 3820–3974, 4052–4125). The final archive is paired
with the post-protection token and that token is used by the last owner check,
artifact record and `ExportFinished` (`runtime.rs`, lines 2116–2179). This
post-commit delivery-token handoff was corrected in the integrated source after
the initial 34769 candidate.

The actual full/draft downloads and assembly correction are retained in the
[34769 receipt](../evidence/export-pcb-handoff-20261003/34769-RECEIPT.md) and
[34770 correction leg](../evidence/export-pcb-handoff-20261003/34770-HEADING-GREEN.md).
Those journeys used a fixture whose generated wiring was already applied, so
they do **not** qualify the branch that applies wiring, a packaging failure's
no-protection behavior, stale/external-mutation races, or persisted history,
undo/redo and reopen. These are remaining behavioral/evidence limits, not an
unresolved API feasibility or missing owner-token transition. BND.2's source
boundary is implemented; task status and remaining acceptance stay with the
root task ledger.

## Actionable early feasibility tasks

1. **Keycap CAD host feasibility (F7.1, before consumer completion):** trace built asset import/worker startup; identify a crate-private request/result route to `build_keycaps`; verify existing wasm bindings, KeycapSpec serialization, body/legend ID mapping, STEP result validation, job/revision/scope correlation, and preview cancellation granularity. Use an already-built package/fixture for the later implementation check; this review itself did not build or invoke WASM.
2. **F6C.5 split/readiness:** keep resolver findings and 2D edits fixture-ready. Mark live CAD preview meshes and keycap STEP as dependent on the private CAD adapter; preserve them as required acceptance, not an out-of-scope feature.
3. **F7.3 integration:** define the shared viewer input as already-resolved body meshes plus source IDs/material/display metadata. Make Keycap preview a consumer of the shared adapter and delay only the real generated-mesh join, not all viewer development.
4. **PCB export sequencing (BND.2/F8.2):** source implementation is present in `Event::ExportCommit`, `ExportCommitRequest`, Session's retained `ExportCommitOwner` through Core/persistence, and the Runtime's `commit_pcb_handoff`/capture guards. Reuse the exact owner/token/scope behavior described above; do not re-open API feasibility. Remaining qualification is specifically the conditional apply branch, failed-package-before-protection behavior, stale/external mutation suppression, and accepted history/reopen evidence. See the bounded 34769/34770 receipts and their stated limits.

## Source evidence index

Pinned React reference (`5a472a9426e6e38993361da402cd4ec730feb369`): `app/src/CaseClient.ts:65-80`, `app/src/case.worker.ts:8-14,34-38`, `app/src/exports/keycaps.ts:3-16`, `app/src/assemblyPreview.ts:140-163`, `cad/src/index.ts:50-63`.

Current Rust/Dioxus baseline (`c827c4e69389a77b1f0e8d647ce86a4c41529611`): `cad/wasm/src/model/keycaps.rs:7-25,247-300`, `cad/wasm/src/lib.rs:1-5`, `web/src/cad_jobs.rs:22-42,99-150`, `web/src/cad_worker.rs:45-55,645-780`, `web/src/runtime.rs:339-365,422-450`, `scripts/build-m1.py:51-74`, `application/src/session.rs:861-895,1334-1350,1685-1690,1749-1780`.

## Current-source continuation — 2026-10-03

The initial missing-host-path finding above is historical. The current Keycaps source now has a dedicated `CadWorker::request_keycaps_step` / `request_keycaps_preview` facade across the page/library crate boundary. `KeycapsInput` and `KeycapsCadRequest` remain private wire structs; the normal public `CadRequest`/`CadOperation`, Core protocol and CAD engine are unchanged. The page-facing method is public because the binary and library are separate Rust crates, which is the narrowly authorized exception recorded in the Keycaps issue and RF-001.

The remaining source-backed BND.1 delta is preview scheduling: the pinned `cad/src/index.ts::buildKeycaps` invokes the existing WASM `build_keycaps` export in groups of eight and yields between groups, but the initial Dioxus worker path sent the complete preview vector in one invocation. Its single synchronous call could not process worker cancellation between batches. The current worker adapter mirrors the existing groups-of-eight policy, yields through the worker event loop, checks cancellation if a worker cancel message arrives before continuing, verifies each batch revision, and concatenates provider-owned cap/legend bodies in order. The Keycaps Runtime currently handles unmount/supersession by terminating its feature-owned worker immediately, so that explicit message path is unreachable from this product route. Candidate34769 qualified the pending-preview unmount, no-late-publication and successful subsequent retry path; the message route itself was not claimed. Each active batch remains synchronous and STEP remains one synchronous call; neither claims kernel preemption. Existing accepted identity, Runtime generation/currentness checks, reply correlation and stable IDs remain authoritative. This bounded route is not BND.1 or F6C.5 acceptance.
