# Draft F7.3 capability child — Canonical Layout board scene and picks

**Parent:** F7.3 — Single shared assembly viewer and model-preview adapter. Operational child only; preserve the existing canonical F7.3 and F3.6 rows and all 62 canonical task records.

**Category:** Behavior-preserving adapter composition relative to pinned React `AssemblyViewer` / `AssemblyPreview`, current accepted Core artifact contracts and existing wasm Renderer behavior.

**Source baseline:** root frozen commit `9d34f3672f4f05cb3771bc5cd358508b13e9c31`; React `5a472a9426e6e38993361da402cd4ec730feb369`. Source hashes and owner/seam analysis: [source inventory](../evidence/11-f7-canonical-layout-source-picks/source-seam-inventory.md). Related Layout view-group plan: [issue 10](10-layout-view-mode-group.md).

## Problem statement

Layout's 2D workspace has no callable Dioxus path that supplies the common renderer with the canonical selected-board assembly and returns a current, board-scoped part pick. The existing Case viewer inputs are physical-instance projections. Using them for Layout can apply reflected instance geometry to a canonical board. The existing wasm Renderer and Core artifact protocols already provide scene, preview and pick primitives; the missing behavior is a private page-side canonical source/owner/selection adapter that connects them to the one shared viewer.

## What to build

Establish one private, callable F7 capability for a canonical Layout board source and supported part picks. The capability accepts an existing immutable accepted snapshot and the exact active Session scope/board identity, produces the canonical board preview through existing Core/artifact and renderer paths, feeds the one shared viewer, and maps an accepted renderer pick back to the unique part in that selected canonical board.

Canonical source always means the original accepted `ProjectDoc` and accepted board scene. A current Session `Scope` can retain `instance_id: Some(...)` from Case while the visible workspace is Layout. Retain that complete scope in the owner identity and freshness checks, but do not use `instance_id` to choose or transform Layout geometry. Do not normalize or replace the captured `Scope`; do not use `captured_case_document`, `captured_case_scene`, `NativePreviewSnapshot`, Case mechanical defaults, or a physical-instance model owner to construct the Layout input. When the active full scope or view owner changes, reject stale output and stale picks.

For an authored board, reuse the accepted Core `PreparePreview` → existing preview-generator worker → `FinishPreview` path with the canonical document and selected board contours. For an enabled imported `BoardReference`, use the existing `PreviewBoard` artifact operation with bytes from the matching accepted document asset. Require `PcbPreview.revision` to match the accepted revision. Preserve React's board/model asset resolution using existing verified asset bytes, model mapping and renderer decoders; do not implement geometry, Core generation, CAD decoding or a second mesh cache in the Layout consumer.

Pass the resulting canonical `PcbPreview`, resolved model meshes and exact source identity through the existing F7 shared viewer and page-private renderer host. Use the existing wasm Renderer `setScene`/`pick` operations. A picked renderer model reference is a `PcbModel.reference`, not its model/cache `id`; resolve it only when the accepted preview contains that reference and it identifies exactly one part on the source board. Then deliver the existing Layout selection callback only while the same full scope, accepted snapshot token/revision, board, workspace/view owner and viewer generation remain current. Empty-board/layer picks, unknown or ambiguous references, stale identity, and superseded async completion are harmless no-ops.

The capability is an internal prerequisite, not a second renderer and not a complete view-group UI. It does not make the entire F7.3 or F3.6 parent complete. F3.6 consumes it for the separately owned 2D / 3D assembly / Footprints selector and return-to-2D behavior.

## Start and acceptance joins

- **Start after:** the accepted F7.1 private adapter feasibility/contract decision and a proven same-page-binary call path. This child may start before unrelated F7.3 consumers or the full F7.3 acceptance are complete. F3.1/Layout UI completion is not a private source-provider start blocker; its selection callback is the consumer seam.
- **F3.6 readiness output:** this child is considered callable only when the accepted canonical source producer, common-viewer input, and owner-checked part-pick resolver compile and pass their named source/behavior checks. A planning document or isolated projection helper alone does not clear F3.6's capability gate.
- **Acceptance joins:** F7.3 full shared-viewer acceptance; F3.1 current Layout selection/scope integration and F3.6 mounted view-group/pick journey. These remain joins, not blanket starts for independent F7 consumer work.
- **No graph change:** keep `tasks.json`, F7.1/F7.3/F3.6 dependency rationales and 62 parent records unchanged. This is a local operational child; the coordinator alone reconciles canonical graph edges if later required.

## Contract and ownership

- Runtime accepted snapshot, Core worker and existing scoped owner remain authorities. No new Session event, Core command/reply, serialized wire field, file-format field, public Rust API, `boardstudio_web` visibility, wasm-bindgen export, worker, or renderer is added.
- Put any new private source/selection projection in the page binary's feature-private presentation modules, where it can call the page-private `Runtime` and `RendererPageHost`. Do not place consumer-only operations on public `boardstudio_web::renderer_host`, `cad_jobs`, or another public library module. Keep renderer behavior in existing F7 `SharedViewer`/`RendererPageHost` owners. The coordinator owns page root/runtime/shared-viewer/global CSS registration and mounts the bounded callback; the capability author owns the new private Layout source/pick leaf and its tests. Avoid touching overlapping shared files without the serial coordinator handoff.
- Keep source owner identity separate from renderer sequence and model batch generation. Include the current full `Scope` (including optional instance), session epoch/document, accepted token/revision/scene identity, selected board and active viewer/source generation. Never assume independently incremented identities can be compared numerically. Reject after await if Runtime/core worker/session or source lease changes.
- Reuse the existing pure board/reference membership mapper where its contract fits, and preserve its `PcbPreview.revision`, board membership and uniqueness checks. Extend/extract only if necessary; retain or strengthen those tests.
- Keep the provider distinction explicit: canonical board (Layout/Keymap/Keycaps), physical-instance Case, isolated Parts sample. This child adds only canonical Layout source/pick wiring. Case and Parts inputs remain under their current owner contracts.

## Acceptance criteria

- [ ] On a two-board accepted document with a flipped physical Case instance selected elsewhere, a Layout source capture retains the exact full current `Scope` as owner while its renderer input comes from the canonical document and canonical selected-board contours. Assert no reflected coordinates/winding, Case defaults, physical projection, document mutation, or instance-dependent output enters Layout.
- [ ] For an authored board, the canonical path uses existing `PreparePreview`/preview-generator/`FinishPreview` contracts. For a selected enabled imported `BoardReference`, it uses existing `PreviewBoard` and the matching verified document asset. In both paths reject wrong document/board/revision/token and unsupported response identity.
- [ ] The shared viewer receives the existing assembly scene shape with accepted `PcbPreview` surfaces/holes/contours/model descriptors and verified decoded mesh inputs. Exercise a real component model rather than treating an empty board shell as a valid part-picking proof. Preserve model ID/cache identity separately from the `reference` emitted for selection.
- [ ] A current renderer pick for a reference present once on the selected board maps to exactly that canonical Part through the existing Layout selection callback. A reference absent from the preview, ambiguous reference, part on another board, body/layer-only pick, or old viewer/source owner cannot change selection.
- [ ] Switch active board, document/project session, accepted revision/token, Case instance, workspace or view owner while preview/model work is pending; completion and picks from the prior full owner are ignored. Cover same-board/session ABA with a changed viewer/source generation. Do not equate preview, renderer and model-batch sequence counters.
- [ ] Reuse one `SharedViewer` and existing renderer host/wasm exports for mount, scene update, picking, resize/DPR, theme, context loss, failure and disposal. No Layout-specific renderer, duplicated scene conversion engine, or new public visibility/API/wire/file schema.
- [ ] Source-level and unit checks prove page binary reachability for the private provider and negative stale/wrong-scope cases. Then F3.6 performs the public paired React/Dioxus 2D→3D→pick→2D browser journey using one identical saved archive/source identities; record it as a separate F3.6 acceptance join, not as complete here.
- [ ] Preserve RF-002, RF-003 and RF-012 as current boundary evidence. Record “No new refactoring takeaway observed” unless implementation demonstrates a new defect; any new duplicate owner/DTO or unproven provider is recorded before changing source. Do not refactor the public host or introduce a generalized viewer rewrite in this child.

## Out of scope

The Layout toolbar/view-group UI and camera-return behavior (F3.6 consumer); common renderer controls or lifecycle implementation (F7.3 owner); Case physical-instance projection; Parts isolated sample; Keycaps/module generation or provider changes outside the existing common viewer's consumed inputs; new Core/CAD geometry behavior; public API/wasm/wire/schema/file changes; replacing the Reflect-based renderer wrapper; global packaging/CI; closing F7.3, F3.6 or F3.7.

## Stop conditions

Stop this child before source edits if F7.1 cannot approve the same-crate private call path, if canonical preview generation requires a new Core/renderer public operation, or if existing asset/preview protocols cannot represent the reference's canonical scene. Send the exact missing operation and evidence for a separate architecture decision. Do not repair the gap by routing canonical Layout through Case projection, weakening current-owner checks, changing `Scope`, or adding an unreviewed API.
