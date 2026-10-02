# F7.3 private Case viewer interface review

**Revise before large implementation: two material ownership/behavior corrections.** The existing `Rc<CadScene>` entry seam and private shared-viewer placement are usable. No repository edits, Cargo or browser execution in this review. Source diagnosis only; no newly reproduced behavioral bug is claimed.

Reviewed integration HEAD `805e7eedf3d041f56d50c97d7868594296c14395` and pinned React `5a472a9426e6e38993361da402cd4ec730feb369`. React renderClient current blob matches pinned source (`7d408a8a2ca2593149a33ff68a38fd7c196cbb8b`). Proposed interface is the coordinator's message, not a saved authored Rust contract.

## Required corrections

1. **Empty pick is a no-op; keep layer selection separate from Session part selection.** `app/src/renderClient.ts:242–243` invokes onPick only for truthy IDs. `AssemblyScene.tsx:119–122` changes mechanical-layer selection for gasket/stack/pcb/battery IDs and separately reports a picked ID. `Workbench.tsx:1386` maps a real part to choosePart or a module to Parts navigation; it does not submit arbitrary renderer layer IDs as part selections. F7.1 already explicitly says an empty pick does not mutate selection. Therefore `id: None` must not clear Session selection. Unmapped IDs also do not fabricate domain IDs. Use the existing root selection adapter only for fresh, eligible real-part mappings; retain Case layer/body/finding presentation ownership separately.

2. **Not all display state is transient.** `useCaseWorkspace.tsx:25–30,87` owns selectedMechanicalLayer and CaseDisplay above the viewer. `caseDisplay.ts:8–35` retains hidden IDs and color overrides in memory and localStorage under `boardstudio:case-display:{projectId}:{selectedInstanceId-or-boardId}`; unavailable storage does not destroy the in-memory choice. `AssemblyScene.tsx:79–91` keeps camera/view mode, explosion and section controls transient. The minimal interface needs parent-owned display input and a scoped display-change callback, or an explicitly named equivalent private owner shared with Case Inspector/tree. A viewer-only transient hidden/colors copy breaks reference persistence and cross-control synchronization.

## Cleared corrected minimum

A Case-specific wrapper may accept `scene: Rc<CadScene>`, `selected_layer: String`, immutable private `CaseDisplay` (hidden/colors), resolved light/dark palette input, `on_pick: EventHandler<ScopedViewerPick>`, and `on_display_change: EventHandler<ScopedDisplayChange>`. Exact Rust names are author choices. Existing theme context may replace an explicit palette prop if its owner/read path is named. Callbacks carry the captured viewer identity; they do not submit domain operations inside the generic renderer.

`ScopedViewerPick` may retain Scope, viewer-instance ID, projection generation, renderer scene sequence and optional render ID. The viewer must compare these against its *current* owner immediately before emission; the parent must re-read current Runtime Scope, accepted token and current Case source/mapping before any Session action. Comparing event fields with themselves is not a guard. Define how expected identity is available to any deferred parent callback. A stale retained CAD scene can remain visible/navigable, but cannot authorize edits or selection against a newer accepted snapshot.

Current `CaseCanvas(scene)` has **no selected-layer seam**: it only provides scene/camera, uses alive-only mount checks and reports via Runtime. This proposal introduces a private presentation input, not a pre-existing Session property. `Runtime::cad_scene()` filters full Scope but does not alone prove token freshness. Preview/exact replacements can share document revision, so a new Rc/source projection must advance projection generation, independently from strictly increasing renderer scene sequence. Guard status and error callbacks too, including same-scope remounts.

Build Case projection from the existing matching `captured_case_document`/`captured_case_scene` seam when document-derived geometry is needed; CadScene.snapshot is the immutable accepted snapshot, not proof that its canonical document already represents the selected physical instance. Reuse its existing result/mechanical/contour data. No new provider/API or canonical-scene alias is authorized.

Keep `CaseSharedViewer` as a Case-to-typed-projection wrapper over one private generic shared viewer. Its `Rc<CadScene>` input must not become a mandatory input for future Parts samples or canonical consumers. Preserve F7.1's sample-projection generation and consumer mappings. Coordinator owns mount/module registration, shared CSS and parent selection/display state; author owns named private modules/host additions. Record exact handoff before editing shared files.

Once the author adopts corrections1–2 and explicitly records current-owner guarding, private host/DTO and Case wrapper work may proceed without a new public-boundary decision. Root-owned integration remains serial. Public empty-pick neutrality, selection/layer distinction, display persistence and scope-switch/retry checks remain required; implementation/source inspection is not runtime acceptance.

Source SHA-256:
- cad_presentation.rs `229ea7d6d7a2df5cae8d20aa7fd2bee2c89f0eea3d77073c4320c34a0e649b3b`
- runtime.rs `7567e9d3742b0a40f510bca01a10f74617a631d83ecf8adb93e310925dbe3532`
- AssemblyScene.tsx `f0cb88c22e26c9788cfd9b10452ff40e88ff89d6944a6bba618068d247340416`
- renderClient.ts `1e6ebb99b4b3e7ca34570dd6dd39d094e514bc7a56100f52cfac6ef5639216bf`
- caseDisplay.ts `ab7b76ab603aedb1ac82b37633e7a6fcd906add396b324f7830b24a55329db44`
- useCaseWorkspace.tsx `80cecb584fb2c6639fb5b7b7a11cd0b82aad13641e19139d5590fbca991c047e`

RF: no new architectural takeaway; RF-002/RF-012 remain the existing boundaries.
