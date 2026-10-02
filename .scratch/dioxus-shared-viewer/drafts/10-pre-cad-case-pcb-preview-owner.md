## Problem Statement

In the React Case workbench, the selected physical assembly's PCB preview is available before authored Case bodies or generated mechanical CAD exist. The saved Sofle archive exercises the ordinary native project-board route and renders its board/model assembly in this state. Dioxus currently creates its Case viewer only from a generated mechanical `CadScene`; its projection contains the board's thickness and physical outline but empty surfaces, holes and model rows. The result is a blank baseline assembly view.

This is a source-boundary mismatch, not missing Case CAD geometry. Native project-board preview (`PreparePreview` → existing Ergogen/KiCad job conversion → `FinishPreview`) and imported `BoardReference` preview (`PreviewBoard`) return a Core `PcbPreview`. Generated mechanical Case output comes from the separate Case preparation/CAD worker and is an optional overlay. A `CadScene` cannot be required as the source of the baseline preview or as its owner identity.

## Solution

Give the current accepted physical Case projection a private, pre-CAD owner that can request and retain its Core-validated `PcbPreview`, then deliver decoded model meshes through the existing verified provider. Publish the accepted board immediately with all Core model-placement rows intact and an empty decoded-mesh list while delivery is pending. Preserve React's successful-mesh publication after the complete decode `Promise.all` settles; per-row error messages may arrive earlier. Keep every update bound to the same current preview owner. The Case viewer consumer mounts this result independently of mechanical Case CAD and combines generated bodies only when a current mechanical scene exists.

Issue08 remains the sole native project-board producer and model-delivery coordinator. Issue07 remains the sole verified document-byte/decoder/cache provider. Issue12 owns the separate consumer projection into the shared Case renderer. An enabled imported `BoardReference` continues to use its existing `PreviewBoard` producer path under Issue07's current owner, not a duplicate native worker. The canonical F7.3 start and acceptance graph remains unchanged.

## User Stories

1. As a Case designer opening a saved native-board project, I want to see the selected physical PCB assembly before creating a Case body, so that the assembly is useful as the baseline for case work.
2. As a Case designer, I want the preview to use the current physical board's actual Core-generated contours, surfaces, holes, thickness and model transforms, so that the rendered assembly represents the accepted project rather than placeholder geometry.
3. As a Case designer waiting for large model files, I want the accepted board to appear before model decoding finishes, so that model latency does not blank or delay the assembly.
4. As a Case designer, I want the accepted PCB geometry to appear before the model decode batch finishes, then all successfully decoded models to appear when that batch settles while missing or invalid rows remain individually reported, so that slow or bad model assets never hide the board or other successful parts.
5. As a Case designer switching physical instances or editing the project during preview generation, I want stale preview and model results discarded, so that geometry from an old physical scope never replaces the current assembly.
6. As a Case designer using an imported routed board, I want the existing `BoardReference` source and transform rules preserved, so that the native producer is not incorrectly substituted for the imported-board path.
7. As a maintainer, I want the producer, asset provider, and renderer consumer to retain separate private ownership, so that the migration reuses Core and renderer contracts without creating a second model provider or public API.
8. As a maintainer, I want this pre-CAD route to remain distinct from generated mechanical Case bodies, so that producing the baseline PCB does not depend on Case generation readiness.

## Implementation Decisions

- Capture the accepted Case `Scope`, snapshot token, document revision, selected physical board/instance, preview generation, effective physical document, and contours from the existing accepted physical projection. This is a private preview-source owner and exists before any mechanical `CadScene`.
- For an ordinary native project board, use the existing Core `PreparePreview` and `FinishPreview` artifact operations with the existing private ordered job conversion. Preserve reserved-net allocation, source serialization, model-path rewriting, exact token/revision/job correlation, and Core's existing validator.
- For an enabled imported board reference, preserve the existing `PreviewBoard` operation and reference source/assets/transform semantics owned by the imported-board path. Do not submit it through native Ergogen conversion.
- Require the result revision to equal the captured accepted document revision. Recheck scope, accepted token, source projection, preview generation, and worker identity after each asynchronous boundary. Publish no stale result or stale error.
- Keep model-delivery owner identity rooted in the accepted Case preview projection (`Scope`, token/revision, viewer/projection generation, and preview batch), not in `Weak<CadScene>`. A mechanical scene can be attached as an optional overlay but cannot be a prerequisite or identity surrogate. Do not widen library visibility or public Rust/wire/document contracts.
- Publish immutable same-owner snapshots: first the accepted `PcbPreview` with its original `PcbPreview.models` placement rows intact and an empty decoded-mesh list; then publish the successful `LoadedModelInput` mesh collection after all decode tasks settle. Per-row missing/decode error messages may publish before batch settlement, while a model-batch failure never removes the accepted board. Do not stream successful mesh rows before the current React `Promise.all` boundary unless a separate UX change is reviewed. Preserve exact `PcbModel.id` renderer joins and `PcbModel.reference` selection identity.
- Issue07 owns verified document bytes, SHA validation, decoding, cache, retry and row lifecycle production; Issue08 chooses asset identity for native preview rows and coordinates the single batch; Issue12 consumes snapshots into the renderer. Do not implement parallel decoding, byte storage, or retry in the viewer.
- Keep generated Case body meshes and mechanical stack as independent optional overlays. Do not fabricate a mechanical `CadScene`, synthesize PCB surfaces/holes/models, or infer a `PcbPreview` from `CadResult`.
- Use only the retained Sofle archive's document assets for the exact baseline. Unarchived static Ergogen model delivery remains deferred.
- Add source-backed RF-003 evidence for the missing pre-CAD shared-scene projection and RF-006 evidence for physical-scope identity. Do not create duplicate RF IDs; preserve RF-002 if a crate/API boundary is involved.

## Testing Decisions

- Prefer paired public React/Dioxus Case journeys using the exact saved layered-Sofle archive and isolated browser profiles. Before adding a Case body, assert that React obtains the native accepted `PcbPreview` and that Dioxus' producer owner reaches the same Core result using the same selected physical document/contours. Distinguish `PcbPreview.models` placement rows from the snapshot's decoded mesh collection: the first board snapshot retains all placement rows and has no decoded meshes. The consumer/render assertion belongs to Issue12; do not claim this producer slice completes it.
- At the producer seam, prove `PreparePreview` → conversion worker → `FinishPreview` request order and job/token/revision correlation, exact accepted revision, physical contour ownership, and stable model identities against the pinned TypeScript reference.
- Hold model decoding pending and observe the board-first snapshot with placement rows retained and the decoded mesh collection empty; then settle successful, missing, and malformed rows. Successful meshes publish as one completed batch after all decode tasks settle, while error messages may publish per row earlier. Verify every update retains the same owner/batch identity.
- Supersede during prepare, worker conversion, finish, byte access and decode; change selected board/instance and accepted revision. Only current-owner results may publish, and a model failure must not discard the accepted board.
- Exercise a flipped physical instance and an enabled imported-board reference separately. Verify the existing reference suppression rule and ensure imported reference input never takes the native conversion path.
- Keep native Rust/page-binary checks and module worker tests as supporting evidence. The public renderer/selection/save-reopen paired journeys remain required by the downstream consumer/parent acceptance.

## Out of Scope

- Implementing or changing Core preview generation/validation, Ergogen/KiCad algorithms, the native worker conversion rules, or `PcbPreview` schema.
- Creating a second model byte provider, SHA store, decoder, cache or retry policy.
- Requiring generated mechanical Case CAD to render the baseline board, or adding PCB data to `CadResult`/`CadScene`.
- Replacing the imported `BoardReference` producer or applying its pose/board transforms twice.
- Supporting unarchived static/bundled model assets, network fetches, public APIs, schema/file-format changes, or public visibility changes.
- Completing Issue12's renderer projection, full F7.3, INT.2/BND.1, or F7.8 acceptance.

## Further Notes

The TypeScript owner is `AssemblyPreview.prepareBoard`: it selects `PreviewBoard` for an enabled reference and the native export preview otherwise; after a revision/current-sequence check it publishes `{ board, reference, models: [], pending: false }` before awaiting model decode. Here `board.models` retains the `PcbPreview` placement rows; the top-level `models` array is the decoded mesh collection. Successful meshes accumulate locally and publish after `Promise.all` settles, while missing/decode error messages may publish as individual tasks fail. Case mechanical generation remains a separate path. The exact Sofle archive has six document model Assets and no `BoardReference`, so its producer path is native; it does not prove unarchived bundled-model delivery.

The current Dioxus `CadScene` is emitted only by Case preparation/CAD worker completion. The shared Case projection reads the selected physical document and contours but hardcodes board surfaces, holes and models empty. `model_delivery.rs` currently anchors provider identity through a weak `CadScene` pointer, which cannot represent this baseline owner. Amend the existing Issue07/Issue08 contracts around this private source identity rather than inserting a dummy scene or adding another provider.
