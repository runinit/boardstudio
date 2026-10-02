# Proposed F7.3 model-delivery child split

**Status:** planning support for unpublished drafts 07 and 08. The archive-backed private ports contract has final Spec and Standards clearance at SHA-256 `8db0731d531d7dccbed90154c093d4fa381f7db8991e990db84c1592056ddfd5`; its retained contract/reviews are in `.scratch/dioxus-shared-viewer/evidence/model-delivery-private-ports/`. The corrected KiCad bridge proposal is retained at `.scratch/dioxus-shared-viewer/evidence/model-delivery-private-ports/kicad-bridge-proposal.md` (SHA-256 `de3bb2a43f8d4db5ee6235819a1c1129e07193e57b779078d3b91b952763702a`), independently Spec/Standards clear, and defers unarchived static model bytes; ticket 08 preserves that limit. The child-07 author module is reviewed and cherry-picked as integration `87891883` (worker `837cb105`), but remains unregistered/uncompiled. Both ticket drafts also require separate independent ticket review before publication. These files do not change the 62-parent graph, publication count (39), or statuses.

## Dependency and integration interpretation

```text
Canonical F7.1 start
       │
       ▼
07 imported/archive-backed BoardReference models
       │  (establishes the common verified-SHA decode/cache/scene delivery path)
       ▼
08 generated Ergogen PreviewBoard models
```

- Child 07 has no ticket blocker beyond canonical F7.1. INT.2 and BND.1 remain F7.3 acceptance joins, not child start blockers.
- Child 08 is sequenced after 07 as a bounded implementation handoff because its preview must consume and exercise the same private verified-byte, decode, cache, per-model error and scene-join path; a second model pipeline would be a duplicate authority. This does not add a canonical parent edge or acceptance join.
- Published issue 02 remains the Case integration-acceptance join. Issues 03–06 are unchanged; they consume the one shared viewer and their existing source-checked model-delivery requirements. These local child cuts add no canonical parent or parent edge.
- Canonical F7.3 remains `start_after: [F7.1]`, `acceptance_after: [INT.2, BND.1]`. F7.8 is still the later cross-workflow join. Neither child closes issue 02, F7.3, INT.2, BND.1, or F7.8.

## Author ownership and serialized root ports

| Work | Isolated author-owned work | Root-owned serial integration ports | Review/public verifier evidence |
|---|---|---|---|
| 07 archive-backed models | One private Case model-delivery module and focused tests. It owns model-row identity, resolution order, bounded decoded-mesh sharing, per-row states and batch supersession; no edits to Runtime/renderer host or shared mount. | `web/src/runtime.rs`/CoreWorker: exact `BoardReference.assetId`→document Asset SHA→verified board-source read→existing `PreviewBoard` artifact under captured current-batch identity, plus SHA-verified model bytes and existing `ReadStep`; private page-host snapshot/extensions plus source-sync: forward decode from the same initialized WASM module while preserving reusable `web/src/renderer_host.rs`; shared viewer/Case mount and build registration. Land these shared-file patches serially. | Independent source review of exact private contract; public imported `BoardReference` archive fixture with real STEP/STL/WRL bytes, model transform/pick joins, failure/retry, pending-owner supersession, current accepted document, route/build identity, visible-UI close/reopen using durable verified SHA bytes, and root/subpath offline checks only where exercised. |
| 08 generated board models | One private generated-preview adapter, dedicated worker source, and focused tests; only consumes child 07's stable archive-backed model-delivery seam. It owns ordered worker result adaptation, generated path/model-binding projection and stale rejection, not native/Core algorithms or a static model provider. | `web/src/runtime.rs` / CoreWorker wrapper for exact PreparePreview and FinishPreview; page module registration; Vite/Rolldown worker bundle; `scripts/build-m1.py` offline precache/root-subpath packaging; Case scene composition/CSS and shared build. Root owns all shared ports/build files and serializes integration. | Independent source reviews of bridge and bundle boundaries; real Ergogen Case fixture with Core plan/finish using accepted-provenance effective physical document/current-scene contours, exact generated IDs/document-Asset resolution, model pick, error/stale path, visible missing state for unarchived IDs; seven-command worker/offline hashes at both routes. |

The author must not edit root-owned files while the root port patch/build queue owns them. Root should land each port group as a named bounded patch and re-review the resulting private call path before declaring the vertical slice runnable. Native/Core tests and decoder unit tests support but do not replace integrated page/public acceptance.

## Acceptance matrix

| Slice | Real starting state and user-visible behavior | Source/domain invariants | Failure/history/offline checks | Does not close |
|---|---|---|---|---|
| 07 imported/archive reference | Import a genuine React-produced archive with enabled `BoardReference` and modelAssets. Case shows real model meshes from SHA-addressed STEP/STP, STL and WRL bytes. | The producer path resolves `BoardReference.assetId`→document Asset SHA→verified board bytes→existing Core `PreviewBoard` under captured batch identity. Then preserve existing `BoardReference.modelAssets[path]` lookup precedence, SHA verification, `PcbModel.id` loaded-mesh key, reference-based selection through current board membership, board reference pose/elevation, and flipped-instance reference suppression. One immutable SHA mesh may serve multiple model rows without copying buffers or collapsing their IDs. | Missing/bad bytes, invalid shapes, unsupported extension, worker failure/retry, healthy-row retention, same-SHA new batch, scope/projection/snapshot supersession, unmount/remount, complete/current renderer sequencing. | Generated Ergon/KiCad board production, unarchived bundled model bytes, authored Case mesh/STEP, issue02 integration acceptance, full F7.3/INT.2/BND.1. |
| 08 generated-preview reference | Real accepted Ergogen Case is previewed; actual generated PCB models appear and pick to current source Parts. | Existing PreparePreview from the accepted-provenance effective physical Case document plus contours from the current `CadScene`→ordered private JS worker→FinishPreview; verify canonical and flipped physical-instance cases against React; existing `exportErgogenForms`, `modelBindings`, reserved-net semantics, safe paths, stable model IDs, stable asset IDs/aliases, ticket07 byte/decode path. No Core algorithm/API/schema/export changes. | Exact order/token/revision/job checks, safe-path/model failures, stale worker/preview/asset/decode, private worker bundle with no unresolved imports; retained document assets in verified durable SHA storage through offline re-open/re-render; only worker/static application chunks are precached in root/subpath manifests, with visible missing-model state for unarchived IDs. | Authored Case mesh/STEP generation/export, broad shared viewer consumer acceptance, issue02 integration acceptance, F7.3/INT.2/BND.1/F7.8.

## Source-grounded seam inventory

The split uses source contracts already recorded by F7.1 and the two model-delivery proposals; no new producer or facade is presumed:

- Imported-board source: `app/src/assemblyPreview.ts::prepareBoard`, `BoardReference.modelAssets`, `app/src/renderClient.ts::readMeshModel`, `app/src/modelMesh.ts`, `BrowserStore::load_asset`, `Runtime.assets`, existing `CadWorker`/`CadOperation::ReadStep`, private `RendererPageHost`, and `shared_viewer.rs::project_case_scene`.
- Generated source: `app/src/export.worker.ts::{netAllocator,runErgogenJobs,finishRequest}`, `kicad/src/ergogen.ts::exportErgogenForms`, `ergogen/src/index.ts::{modelAssetId,modelBindings}`, `Core PreparePreview/FinishPreview` in `core/src/artifact/kicad/{planning,output}.rs` and `core/src/artifact/preview.rs`, `scripts/build-m1.py`, plus the existing `@boardstudio/v2-app` Vite/Rolldown workspace toolchain.
- Existing graph authority: `.scratch/dioxus-frontend-v1/tasks.json` F7.3 and `.scratch/dioxus-shared-viewer/issues/01-common-viewer-contract.md`; published issue 02 and consumer 03–06 remain intact.

## Refactoring handoff

The known Rust page/library boundary and partial reflective renderer host remain RF-002/RF-012; semantic scope identity remains RF-006. This decomposition adds no new structural finding. Record “No new refactoring takeaway observed” if implementation reveals no additional source-backed issue; do not create new RF IDs just for private adapters or work decomposition.
