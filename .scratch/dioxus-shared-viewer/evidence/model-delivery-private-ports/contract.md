# F7.3a model delivery: private ports and implementation boundary

**Status:** implementation specification for independent review; no source implementation or acceptance claim.  
**Worker base:** `/home/chris/.local/share/boardstudio/worktrees/frontend-shared-viewer-20261002`, HEAD `84c300a49ec6474e07c49f2d670c9e8205f2c14d`.  
**Reviewed contract:** integration worktree `.scratch/dioxus-shared-viewer/evidence/model-delivery-contract-reviewed/contract.md`, SHA-256 `e3529ac79f1b7d69211a8fa4cdc9e4d1170ec39a3c6510ab458dddc3a634b946`.  
**Ticket:** published issue 02, continuing F7.3a as its existing Case slice. Preserve start-after F7.1 and acceptance joins INT.2 + BND.1; this spec creates no parent prerequisite or join.  
**Scope:** archived/document-local model asset resolution, existing STEP/STL/WRL decoding, and private Case viewer delivery/lifecycle. Native KiCad preview generation/allocator/source rewriting is a separate author-owned seam. No Core/schema/renderer-export/public API changes.

## Source-grounded decision

Use the exact document asset identity chain already used by React and the archive format:

```text
PcbPreview.models[i].path
  -> BoardReference.modelAssets[path]                 (when imported-board reference applies)
  -> native-preview path table ID                     (when native producer supplied one)
  -> existing Ergogen modelAssetId(path)               (last source-compatible lookup)
  -> ProjectDoc.assets[id].sha256
  -> Runtime's current-import bytes by SHA, then verified BrowserStore bytes by SHA
  -> decoder selected by the Asset's existing filename extension
  -> LoadedModelInput { id: PcbModel.id, mesh }
```

The model path and display name are not byte identities. Do not use the path as an asset key, derive a fake asset ID, or map a renderer model ID to a Part ID. For an imported board, `BoardReference.modelAssets` has first lookup priority, as in React `AssemblyPreview.prepareBoard`; the native path table comes next, followed by the version-matched Ergogen `modelAssetId` helper. A final ID must resolve to a `ProjectDoc.assets` row for the archive-backed path in this ticket. Its `sha256` is the only key passed to persistent storage.

`BrowserStore::load_asset(sha256)` already returns `None` for a missing object, rejects non-`Uint8Array` storage, and recomputes SHA-256 before yielding the bytes. `Runtime` also has an in-memory SHA-keyed `assets` map used while an archive has just been imported and before persistence completes. Today the map is private to `runtime.rs`; direct `BrowserStore` reads alone can therefore miss valid current-import bytes. The Runtime-owned byte port below is needed to consume both existing stores without copying data into a second provider. Preserve integrity checking for the in-memory hit too; a cache hit cannot weaken the same SHA contract.

React has a third byte source for an unarchived `ergogen:model:<vendor>/<filename>` ID: `app/src/bundledModels.ts` uses a Vite glob and returns a static URL. Dioxus does not currently package that byte registry. The retained archive packs used bundled models as ordinary document Assets with SHA-addressed bytes, so the archived Case path is supported without that third provider. For a genuinely unarchived bundled ID, report the existing missing-model state; do not guess a static URL. A private static manifest/copy step would be a separate build-port decision, preserving existing IDs and upstream filename rewrites. It is not a new F7.3 start blocker and is not silently implemented here.

## Concrete private module and root-owned ports

### Author-owned file

Create one new page-presentation module, `web/src/presentation/model_delivery.rs`, declared privately by the page-binary presentation composition. It owns typed model request/result identity, asset-ID resolution order, bounded decoded-mesh reuse, per-model pending/missing/error outcome, and merge into scene rows. It consumes producers/ports; it does not implement board preview generation, define a new storage backend, alter the renderer DTO, or become a shared public crate service.

The module's narrow data seam must make model ownership independent of renderer scene submission:

```rust
struct ModelOwnerIdentity {
    scope: Scope,
    snapshot_token: SnapshotToken,
    viewer_instance: u64,
    projection_generation: u64,
    source_scene: Weak<CadScene>, // pointer-equal to the current captured scene
}
struct ModelBatchIdentity {
    owner: ModelOwnerIdentity,
    accepted_revision: u64,
    batch_generation: u64, // advances when this owner requests a replacement PcbPreview
}
struct ResolvedAsset { asset_id: String, sha256: String, filename: String }
struct DeliveredModel { id: String, mesh: Rc<DecodedMesh> } // id is exact PcbModel.id
```

Do not embed the whole `ViewerIdentity` in `ModelOwnerIdentity`: its `renderer_sequence` is an independent submission counter. Comparing it would invalidate pending model delivery whenever an initial board scene or another healthy mesh update submits a newer renderer scene. Model work is current when its scope, snapshot token, viewer instance, projection generation, captured `CadScene` pointer, and batch generation still match. The Case viewer owner allocates a separate strictly increasing renderer sequence for each new complete/partial immutable scene submission; `RendererPageHost` enforces it and preserves the renderer's accepted/stale boolean. A current completion from this batch merges into the latest model set and submits with a fresh scene sequence; a stale owner/batch cannot publish.

Types may stay module-private. `DecodedMesh`/its `Rc` handle is a private representation of positions/normals and optional renderer colors; it must not duplicate or rename a renderer/Core public DTO. Asset selection resolves each `PcbPreview.models` row to `ResolvedAsset`; byte reads and format decode come from the ports below. The output preserves a one-to-one row identity: every delivered mesh uses exactly the row's `PcbModel.id`, while render picking continues to return `PcbModel.reference`. Keep this `ModelBatchIdentity` separate from the renderer scene's submission sequence in the types and APIs, not only in comments.

### Root-owned Runtime port

Add a crate-private async Runtime operation in `web/src/runtime.rs` for verified model bytes by SHA-256. It checks the current import map first, validates a hit against its requested SHA, then falls back to the existing `BrowserStore::load_asset`; its result distinguishes missing from storage/integrity failure. It accepts no model path or UI filename. It does not expose the Runtime map or add a `BrowserStore` provider API. The model owner rechecks its viewer identity after the await before publishing or beginning decode.

Add a separate crate-private STEP read port at the Runtime/CAD boundary; do not embed another CAD worker in the presentation module. It submits existing `CadOperation::ReadStep` with unique request ID and job ID, `prepared: None`, source bytes, and the exact current `CadSnapshotIdentity` derived from the captured accepted snapshot and effective Case projection. Use/retain the existing scope-bound `CadWorker` lifecycle (or a root-approved private worker lease if Runtime ownership requires it), call existing reply validation, and check caller-current before and after awaiting. The existing worker enforces nonempty input up to 32 MiB; `validate_reply` checks request ID, job ID, operation, full snapshot identity, finite triplet mesh buffers, and bounds. No new CAD operation or result shape is needed.

### Root-owned renderer module port

Add a private page-only model decoder on `RendererPageHost`, backed by the same initialized renderer WASM namespace already imported during `RendererHost::mount`. React memoizes `loadRenderer()` and invokes the existing `decodeStl`/`decodeWrl` exports on that module; the current Dioxus host imports/initializes that namespace but does not retain or call its decoder exports. Root owns the required private host storage/forwarding changes across the renderer-host source/snapshot synchronization boundary. Update the host source and its page snapshot only as a named, exact page-normalization delta; keep `renderer_host_source_sync` green and retain a diff proving the page host still matches normalized shared-host source. Keep the reusable `boardstudio_web` library host surface and generated WASM exports unchanged; do not instantiate a second renderer/WASM module just for decode.

The page port selects `decodeStl` for `.stl`, `decodeWrl` for `.wrl`, case-insensitively. It validates returned typed arrays before scene submission: nonempty triangle positions with length divisible by nine, equal-length normals, finite values, and optional RGB colors with the same element count as positions. Reject unsupported extensions and byte inputs outside 1 byte–32 MiB with the same visible error category as the current React model flow. STEP results receive the same complete-triangle/finite shape gate in addition to the existing worker's stricter reply and bounds validation.

### Root-owned Case/shared-viewer integration ports

Root owns page-binary module registration, mount/composition/build, `Runtime`, and renderer host changes. The Case projection/mount must pass `PcbPreview.models`, the enabled effective `BoardReference` (pose and elevation), and the decoded `LoadedModelInput` rows into the existing scene shape. Preserve the existing flipped-physical-instance reference-disable rule. Do not transform each model in the frontend; the renderer applies `PcbModel` source pose/side/offset/rotation/scale and the board reference pose/elevation. On repeated assets, construct scene rows from shared decoded `Rc`/JS typed-array handles so each row changes only the exact `id`; do not copy model buffers per instance in the Dioxus projection.

The native board producer returns `PcbPreview` and the native path table through the separate KiCad job-bridge ownership. Model delivery consumes those results. It must not call raw Ergogen render, regenerate source, or take ownership of reserved-net allocation, footprint/object form serialization, unresolved model-path rewriting, legacy-arc upgrade, or plan token/revision/job correlation.

## Async identity, cache, and failure rules

1. A model batch captures `ModelOwnerIdentity`, accepted revision, batch generation, and board/model rows. The current Case viewer already has owner scope, snapshot token, viewer instance, projection generation, plus a `SourceGuard` that checks active Runtime scope, accepted token, and exact current `CadScene` pointer. Reuse those checks; do not compare the model batch to the renderer submission counter or rely on document revision alone.
2. Require `PcbPreview.revision == accepted document revision`. Check owner and batch after asset-byte await, after STEP or renderer decode, and before adding a mesh to the current delivered rows. At submission, independently allocate the next renderer sequence and preserve its accepted/stale result. A current model batch can survive newer renderer scene submissions; a late completion from a replaced owner/batch cannot mutate current state, publish an error, or attach under a later board/projection.
3. Keep a bounded SHA-keyed cache with separate states for immutable completed mesh and pending decode task. A completed immutable SHA mesh may be reused by a later batch. A pending task belongs to the exact `ModelBatchIdentity`: reuse it only within that batch. When a new batch requests the same SHA while an older task is pending, replace/restart that cache slot for the new batch; do not make the new batch inherit the old task's cancellation/stale failure. Give each task a private generation/token; an old completion may update/evict a cache entry only if that exact task still occupies the slot. This prevents stale cleanup from deleting a newer replacement. Delete failed current tasks so the existing retry action can retry. Keep capacity at 80 entries, as React `AssemblyPreview` does. Clear scope-owned cache on document/board/instance replacement and viewer close/unmount. Do not keep a second long-lived raw-byte cache in the presentation layer.
4. A stale owner/batch cannot publish a failure message either. Cancellation/supersession is silent for the old batch; only an error from the current batch becomes visible. Current per-model missing/decode errors are reported against `PcbModel.reference`; successful models still deliver. A missing row produces no placeholder mesh. Preserve the existing model retry affordance. Worker closure/error settles its waiters using existing `CadWorker` behavior; renderer teardown remains `RendererHost` responsibility.
5. Keep preview metadata per model row. Mesh reuse by SHA must not collapse model rows: one asset mesh can back many rows, each with its own exact `PcbModel.id` for transform join. Never send asset SHA, asset ID, model path, or reference as `LoadedModelInput.id`.
6. On pick, the renderer returns `PcbModel.reference`. The workflow owner resolves that exact reference through the captured current board's `partIds` and current document Parts. Unknown reference, model ID, or missing current Part maps to no domain selection. This port does not handle generated module/keycap producer IDs.

## Concrete acceptance and regression evidence

- Use the retained React `REVIUNG41` archive, archive SHA-256 `833533fa7fee58180c4721be54325c65b8600a935f75ac44a95c9de9059ab93e`. It has 89 Parts and four archive Assets; retained React source/QA records 124 model instances, which is an expected runtime count rather than a fresh measurement. Record the observed `PcbPreview.models` row count and assert every exact `PcbModel.id` survives; verify the four SHA assets are loaded/decoded once each. Keep the retained SW8 -> `main-right-keys-SW8` pick bridge; do not invent a model-to-Part ID.
- Add a real STL and real WRL asset case through the actual page byte provider + `RendererPageHost` decoder + scene-input path. Assert each decoded mesh is attached under the exact originating `PcbModel.id`; decoder-only unit tests are supporting evidence, not acceptance.
- Exercise STEP and STP via the existing ReadStep worker port; assert exact job/snapshot identity validation, finite mesh, bounds, and the 32 MiB limit. Exercise missing storage row, SHA mismatch, unsupported extension, decoder error, and retry without losing already healthy model rows.
- Supersede while each stage is pending: asset read; STL/WRL decode; STEP worker reply; and scene submission. Change board, physical instance, snapshot, projection, and unmount/remount. Assert old completions publish neither meshes nor messages and are not submitted with a current renderer sequence. Verify worker and GPU/listener cleanup through existing owner lifetimes.
- Supply a nonidentity BoardReference pose/elevation; separately exercise a flipped physical Case projection with reference disabled. Check model transforms/pick identity against pinned React. Preserve generated-keycap suppression when generated keycaps are present.
- Do not claim unarchived `ergogen:model:*` byte delivery unless a static-resource manifest is added and the copied source hash/asset map is verified. Current page resource packaging proves the version-matched Ergogen source module but not its model file bytes.

## Source seams inspected at worker HEAD

- `web/src/host/storage.rs::BrowserStore::load_asset`: IndexedDB key is SHA-256; validates value type and recomputes digest.
- `web/src/runtime.rs::Runtime.assets`, archive import/save, `Runtime::cad_scene`, and `generate_current`: current import bytes are SHA-keyed; CAD worker is scoped and closed on scope change; current Case generation validates replies and checks job/snapshot guards.
- `web/src/cad_jobs.rs::{CadSnapshotIdentity,CadRequest,validate_reply}` and `web/src/cad_worker.rs::{CadWorker::request,validate_request,ReadStep}`: exact request/job/snapshot correlation and current 32 MiB STEP limit/mesh validation.
- `web/src/renderer_host_page_extensions.rs::RendererPageHost` and included `renderer_host_page_base.rs::RendererHost::mount`: private page host wraps the mounted renderer, but the imported module namespace is currently local to mount and decode is not bridged.
- `web/src/presentation/shared_viewer.rs::{ViewerIdentity,SourceGuard,project_case_scene}`: projection currently sends empty board/top-level model arrays and retains Case CAD body meshes only.
- `app/src/assemblyPreview.ts::{prepareBoard,convertModel}` and `app/src/renderClient.ts::{loadRenderer,readMeshModel}`: source precedence, SHA cache, 80-entry bound, retry eviction, STEP/STP vs STL/WRL routes, 32 MiB bound, and existing WASM decode functions.
- `app/src/bundledModels.ts::bundledModelBytes`, `ergogen/src/index.ts::modelAssetId`, `scripts/web/build-layout-generators.mjs`: stable Ergogen IDs/path helper are version-matched into Dioxus layout-generator assets; Vite's byte registry is not.

Source hashes for the cited current worker files and contract are retained in `/tmp/frontend-run/model-delivery-source-hashes.txt`. No Cargo, browser, implementation, API, build, publication, or worktree mutation was performed.
