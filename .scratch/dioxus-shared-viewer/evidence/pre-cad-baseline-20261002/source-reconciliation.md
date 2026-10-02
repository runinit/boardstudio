# Pre-CAD Case `PcbPreview` source reconciliation

Reviewed against integration baseline `42b9fdef` and the isolated shared-viewer branch through `f6dd310b` (2026-10-02). This source audit distinguishes a Core PCB preview from generated mechanical Case CAD; it is planning/implementation evidence, not a claim of paired acceptance.

## Source ownership

- TypeScript `AssemblyPreview.prepareBoard` selects two different Core artifact sources. An enabled `BoardReference` reads the referenced board asset and invokes `PreviewBoard`. An ordinary/native project board invokes the existing export preview path, which calls `PreparePreview`, executes the ordered existing Ergogen-to-KiCad conversion with reserved-net/model-path rules, and calls `FinishPreview`. Both return the Core `PcbPreview` used for board surfaces, holes, contours, thickness, model transform rows and diagnostics.
- The paired layered-Sofle archive contains six document model Assets and no `BoardReference`, so the baseline route is the native project-board preview. Its React preview is available before a Case body exists.
- TypeScript checks request sequence and document revision, publishes the accepted board/reference with the top-level decoded-mesh `models: []`, and only then awaits model decode. `PcbPreview.models` placement/path/reference/transform rows remain intact in `board`. Successful meshes accumulate locally and publish together after the entire `Promise.all` settles; missing/decode error messages may publish per row earlier while the request remains current.
- Mechanical Case generation is separate. Dioxus `Runtime` emits `CadScene` after Case preparation/CAD-worker completion; its `result` contains mechanical bodies, stack and physical contours. It is not a Core `PcbPreview` and is not the baseline board source.
- Dioxus `CaseViewer` currently requires `Rc<CadScene>`. Its `project_case_scene` uses the selected physical document/board and current contour projection but sends empty board surfaces, holes and models. No current page `Runtime` path calls `PreviewBoard`/`PreparePreview`/`FinishPreview` for the pre-CAD Case baseline.

## Existing private work and remaining capability

- The current integration has commit `87891883` adding a model-delivery helper and `a93eb523` adding a private module worker around existing `exportErgogenForms`. These are partial capabilities, not a producer integration: the worker does not call Core Prepare/Finish, retain an accepted `PcbPreview`, or mount it in Case. The provider helper still requires a weak `CadScene` pointer for source liveness.
- The existing model-delivery helper's `ModelOwnerIdentity` stores `Weak<CadScene>` as its live source guard. This is valid for generated Case scene overlay delivery but cannot identify the pre-CAD baseline preview. The provider's SHA verification, decoder, cache and retry ownership should stay in Issue07; revise its identity to a private accepted-preview/source owner that exists before `CadScene`.
- Existing `captured_case_document`/`captured_case_scene` perform the accepted physical-instance projection and are usable before Case CAD. Native Issue08 input should take the matching effective document/board and contours from this accepted source projection. The mechanical `CadScene` remains optional overlay state and cannot provide producer identity or be faked as a prerequisite.

## Minimal contract and exact gates

Keep ownership singular: Issue08 owns the native `PcbPreview` producer and coordinates one model batch; Issue07 owns document-byte verification/model delivery lifecycle; Issue12 owns the renderer consumer. Do not add a producer ticket or provider copy if these reviewed existing tickets can express the narrow capability.

Issue12 may start when (a) F7.1's reviewed private viewer input/update seam is reachable in the current page binary; (b) Issue08 exposes a current-scope accepted native `PcbPreview` produced from the accepted effective physical document/board and source-projected physical contours before any mechanical `CadScene`, preserving snapshot/revision/job identities and board-first publication; and (c) Issue07's private provider can own pending/partial/failed/delivered snapshots using that same pre-CAD source identity without requiring `CadScene`. Full Issue07/08/Case/F7.3 public acceptance, authored Case bodies, and INT.2/BND.1 are not start blockers; canonical F7.3 joins remain unchanged.

The exact Sofle journey exercises the native route. Imported `BoardReference` source/asset/pose handling remains a separate Issue07 path. A generated Case board/body result is not a substitute for either `PcbPreview` route.

## Refactoring register

No new RF ID: this is additional source-backed evidence for existing RF-003 (engine/artifact capabilities are not consistently exposed through host/provider/scene projections). Preserve RF-006 for canonical-versus-physical-instance identity. Preserve RF-002 if implementation encounters the library/page-binary reachability boundary. Missing scene data alone is current parity work, not a separate design finding.
