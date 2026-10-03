# BND.1 source contract — accepted Keycaps STEP action

Source freeze: `0a19f622859c8997cff322a90b47c6f92ff8d33b` on the isolated Keycaps geometry worktree.

The page binary declares `mod runtime` and imports `boardstudio_web::cad_worker::CadWorker`; the web library declares `pub mod cad_worker`. The page and library are separate crates, so the page cannot call a `pub(crate)` worker method. The source packet therefore exposes one specific `CadWorker::request_keycaps_step` method across that boundary. Its `KeycapsStepRequest` and `KeycapsStepInput` wire types stay private to the library; public `CadRequest`, `CadOperation`, generated contracts, and CAD builder APIs are unchanged.

The page resolves `CoreRequest::ResolveKeycaps` from the exact accepted document and active board with `cases: None`. The private request carries `KeycapSpec` values plus `CadSnapshotIdentity` into the existing module-worker queue. The packaged CAD module already re-exports `build_keycaps`; its builder accepts `{ revision, specs, export: true }`, constructs the cap and optional legend solids, and returns STEP bytes. Preview body IDs are `keycap:{id}` and `keycap-legend:{id}`. The STEP page path ignores Case scenes and rendered meshes as manufacturing input.

`Runtime` owns the operation identity, accepted session/document/board/revision checks, a separate `CadWorker` in `export_workers`, cancellation/disposal, reply validation, artifact filename/MIME type, and browser delivery. A cancellation while Core resolves prevents CAD start. A cancellation while the synchronous CAD call runs closes the export-owned worker; both the worker's post-call cancellation check and the page's post-await owner check suppress stale output.

Paired browser/download acceptance is pending the root's source-stamped combined build at 34749. This source contract is not BND.1/F6C.5/F7.3/F8.4 closure.
