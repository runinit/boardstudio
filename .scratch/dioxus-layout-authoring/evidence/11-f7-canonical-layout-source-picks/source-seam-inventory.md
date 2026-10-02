# F7 canonical Layout source and pick capability — source inventory

**Frozen root source:** `9d34f3672f4f05cb3771bc5cd358508b13e9c31` (verified as the root worktree HEAD; relevant application files were clean at that commit). **React oracle:** `5a472a9426e6e38993361da402cd4ec730feb369`. **Planning scope:** source-only contract inventory; no application code executed or modified.

## Existing contracts that are sufficient

| Existing operation/type | Source | What it provides |
| --- | --- | --- |
| Accepted source | `boardstudio_application::AcceptedSnapshot`, `Scope`, `SnapshotToken` | Immutable accepted `ProjectDoc`, accepted `SceneDelta`, epoch/token/revision identity and full current scope. `Scope` includes optional `instance_id`; keep it in freshness identity even when projecting canonical Layout geometry. |
| Generated-board preview | `ArtifactRequest::PreparePreview` + `PrepareExportRequest`; `ArtifactReply::PreparePreview`; `ArtifactRequest::FinishPreview`; `ArtifactReply::PreviewBoard` / `PcbPreview` | Existing Rust Core board preview protocol. `PrepareExportRequest` carries snapshot token, expected revision, document, `ExportTarget::Board`, contours and model paths. The existing private preview-generator worker runs approved export jobs between Core's prepare and finish operations. |
| Imported-board preview | `ArtifactRequest::PreviewBoard { source, revision }` → `ArtifactReply::PreviewBoard` | Existing direct source-file preview operation for the enabled `BoardReference` path. The matching source bytes are an existing accepted document asset resolved by SHA through the page's existing byte store. |
| Renderer scene update and picking | existing wasm `Renderer.setScene` / `Renderer.pick` and page-private `RendererPageHost::{mount, submit_scene, pick_at_client}` | Existing assembly-scene submission and reference-valued pick primitive. `RendererPageHost` also wraps already exported display/camera/handle operations. No new renderer or wasm export is indicated. |
| Pick mapping | `case_preview::part_for_native_preview_reference` | Existing private pure mapping validates preview revision, selected board membership, preview reference membership and uniqueness before returning a Part ID. It does not itself require a Case projection or physical lease; reuse/extract this rule for canonical picks while preserving its tests. `native_preview_pick_part_id` is separately Case-lease scoped and is not the Layout pick entrypoint. |
| Asset/model source selection | `presentation/model_delivery.rs::{select_model_asset, VerifiedModelBytes, ...}` and existing `Runtime` asset/store ports | Existing digest verification, archived/packaged precedence, asset-source resolution, decoding and mesh caching pieces. Owner currently depends on `CasePreviewOwnerLease`; canonical Layout needs owner-valid integration, not bypass of the lease checks or an unrelated mesh cache. |

## Observed missing private call path

1. `web/src/main.rs` declares `presentation`, `runtime`, `renderer_host_page` and `case_preview` as page-binary private modules. This is the correct reachability location for the consumer adapter: page-side `Runtime`, `CoreWorker` and `RendererPageHost` can be called here without adding a method to the separately built `boardstudio_web` library. `web/src/lib.rs` publicly exposes `renderer_host` and `cad_jobs`; do not add Layout-only public methods/types there. This preserves RF-002's crate boundary.
2. `Runtime::prepare_native_case_preview` already demonstrates private page-side Core artifact prepare / preview-worker / finish and current-owner checks. However `capture_native_preview` starts with `captured_case_document`, `captured_case_scene`, and a Case physical identity; it rejects a physical-instance mismatch and excludes imported `BoardReference` previews. It is not a Layout implementation to call with a changed `instance_id`.
3. `presentation/shared_viewer.rs::CaseSharedViewer` accepts `ViewerSource::Cad` or `ViewerSource::Native`; `project_source` routes those to `project_case_scene` and `project_native_preview`. `project_case_scene` calls `captured_case_document`. `project_native_preview` requires the native preview owner scope/token and renders `PcbPreview` values, but that source is still produced by the Case capture. `SharedViewer` already owns renderer mount/update/pick/lifecycle, `ViewerIdentity`, sequence validation and stale pointer rejection. The missing seam is a canonical source branch/input to this shared owner, not another component that owns a canvas.
4. `presentation/case_viewer.rs` validates Case selection against Case selection state and dispatches native-preview picks through `native_preview_pick_part_id`. Layout has no current common-viewer source/pick callback. A new canonical source must route only through the existing Layout part-selection owner.
5. `presentation/model_delivery.rs` has reusable path resolution/verified asset/mesh pieces, but its source owner is Case preview lease-bound. The public Case baseline RF-003 also records missing real mesh source-to-provider-to-viewer wiring and 0/90 model mapping. Do not treat this helper or renderer `PcbModel` descriptors as proof that a real loaded model is available or pickable; the acceptance needs one real component model and the current canonical source owner.
6. `presentation/layout_workspace.rs` currently owns command controls and Footprints only; it does not produce a renderer scene or mount 3D. `presentation.rs` owns workspace/scope composition and root registration. Layout's existing accepted 2D scene has board contours in `AcceptedSnapshot.scene`; 2D selection, session camera and scope stay under their existing owners.

## Canonical source contract guardrails

- Read canonical geometry and `ProjectDoc` directly from the exact `AcceptedSnapshot`; choose `Scope.board_id` only after validating document ID, session epoch, active board and `snapshot.scene.revision == document.revision`.
- Preserve the **complete captured current `Scope`** in owner freshness identity, including optional `instance_id`. The active Session may still carry a previously selected Case instance while workspace is Layout. `instance_id` is freshness metadata here, not a projection instruction: never normalize the scope or use it to reflect/offset Layout geometry. A scope change invalidates work; canonical output remains independent of that optional instance.
- For authored boards, the existing preview protocol uses the canonical accepted document and only the selected board's accepted contours. For an enabled imported `BoardReference`, consume the matching accepted asset bytes via `PreviewBoard`. In either path require exact request ID, document/scope/source owner, preview revision and accepted token/revision before publishing.
- The renderer input retains current assembly JSON contract (`kind=assembly`; canonical board thickness/contours/surfaces/holes/model descriptors, loaded mesh inputs, layers, display, theme and renderer sequence). Preserve renderer sequence as its own domain, unrelated to preview generation/model batch counters.
- A supported pick is the renderer's model **reference**. Resolve only against the matching current `PcbPreview` and exactly one matching Part in `board.part_ids`. Mesh/cache `PcbModel.id`, a renderer layer ID, unknown/duplicate references, and wrong-board parts do not select a part. Recheck the full current Runtime Scope, accepted token/revision, board, Layout workspace/view generation, source owner lease and viewer identity at callback delivery.
- Source work/model delivery is canceled or its result ignored on project/session replacement, accepted revision/token change, board/scope change (including `instance_id` change), workspace/view change, Core worker replacement, source owner replacement and unmount. Do not use numeric sequence equality as proof of common ownership.

## Proposed bounded ownership and handoff

- Capability author owns one new page-binary private Layout source/pick leaf and colocated source/mapper tests. It should contain no canvas or renderer lifecycle and no copies of Core/Core-generation policy.
- The coordinator owns any edits to `web/src/main.rs`, `web/src/runtime.rs`, `web/src/presentation.rs`, `web/src/presentation/shared_viewer.rs`, `web/src/renderer_host_page.rs`, and shared/global CSS. Handoff those call sites as a small serial patch so the common viewer stays one owner and feature modules remain disjoint.
- Feature output is a private current canonical source projection plus an owner-checked pick-to-Part callback. F3.6 owns the 2D/3D/Footprints controls, command/context chrome, cancellation and preserved 2D camera. F7.3 owns the common renderer and all viewer lifecycle/display capabilities.
- Start after accepted F7.1 private-call reachability/contract and the exact source capability listed above. Do not wait for unrelated F7.3 consumers. Acceptance still joins full F7.3 and mounted F3.1/F3.6 Layout selection/view journeys. This local packet does not change `tasks.json` or any of its 62 parent rows.

## RF handoff

This contract intersects existing RF-002 (public library/binary private reachability), RF-003 (engine/provider/host capability discovery and actual model delivery), and RF-012 (partial reflective RendererHost wrappers). Preserve those findings and their validation criteria. **No new refactoring takeaway observed at planning time.** If implementation reveals duplicated canonical/Case producer policy, separate owner-counter coupling, an unsafe asset path, or another source-backed design defect, record it with evidence before making a structural change. A future typed viewer port or tagged canonical/physical/sample source model remains a deferred post-port proposal, not part of this adapter capability.

## Frozen source hashes (SHA-256)

| Source at root commit `9d34f3672f4f05cb3771bc5cd358508b13e9c31` | SHA-256 |
| --- | --- |
| `web/src/main.rs` | `3aa104c2c2d2eabc91ae6f6de38db0394a4a5cdd06dc995b4350121cd2e5c037` |
| `web/src/runtime.rs` | `ff1333eedf03e7fa3e7c14f334a7b94211ab27f31c85d9f79303c19ceb218512` |
| `web/src/case_preview.rs` | `989e10d68125251e11c784031bc22b6365eec9ce5e652e88ad3b7a2db55e898d` |
| `web/src/presentation/shared_viewer.rs` | `d459f3ad1470968c13bff0994765e0120d5a1d22cab8c34e342fdc85c7f8752e` |
| `web/src/presentation/case_viewer.rs` | `4cb2f09c48da19fc9b3deaf262a03b9781e8ce574f5b8e5b64cb8515a8643b45` |
| `web/src/presentation/model_delivery.rs` | `c056edd95139022fa72fb9649b09404de4014a13de7569e6a779df5e1e3a6ab7` |
| `web/src/renderer_host_page_extensions.rs` | `57c3cd439dfc434069a0199dc61d06aa5fbe647922dc3452806435b34d0dc907` |
| `web/src/presentation/layout_workspace.rs` | `80c7ae1921a4c11893177d3c5df504654dd8ad52faf27bebb4717f00d6f864cc` |
| `core/src/model.rs` | `e2ea75daa133195a25a877ef33e03e9a971f49ead3d70ef10a8f1d51dcb2efea` |
| `renderer/src/wasm.rs` | `01aac59ad3b484384a3b32f4e204316617c2bfa84d99e1836c932a323d891b60` |
| pinned React `app/src/assemblyPreview.ts` | `1a530930423b66a36caaccd2600e34ad4bf2a9b7acc6251c2719f923532e512f` |
| pinned React `app/src/ExportClient.ts` | `28e851fdcad480ae2179e3b4addc0194b4a0a45e86c0196a36e9f31f74dac578` |
| pinned React `app/src/ui/AssemblyViewer.tsx` | `40bd398d6b2e9dad397e26ed0bfedf1f1dcf0ae7709a85efccbe57bf0cb980cd` |
