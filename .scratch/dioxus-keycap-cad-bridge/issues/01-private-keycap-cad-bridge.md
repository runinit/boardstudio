# BND.1: Prove private Dioxus access to existing Rust keycap CAD export

**Parent:** BND.1 — `.scratch/dioxus-frontend-v1/tasks.json`

**What to build:** Add the React-equivalent `Export keycap STEP` action to the Dioxus Keycaps Inspector. Resolve keycap specs for the current accepted board with Core, invoke the already packaged Rust `build_keycaps` export in an export-owned CAD worker, and download STEP bytes only while the captured session/document/board/revision is still current. This is the STEP-action slice of F6C.5; shared Keymap/Keycaps 3D preview and viewer acceptance remain open.

**Blocked by:** None (can start immediately).

**Status:** implementation in progress; paired source-stamped browser acceptance remains open.

- [ ] The Keycaps Inspector shows `Export keycap STEP`, disabled only when there are no projected physical keys, matching the pinned React readiness behavior.
- [ ] Core receives `ResolveKeycaps` for the accepted document and active board with `cases: null`; currentness is checked after resolution and against the exact revision before CAD starts.
- [ ] Core error findings surface in order with their existing messages; no specs show `Choose a keycap profile in Keymap before export`; worker/CAD failures remain visible as an alert.
- [ ] The private worker invokes the packaged `build_keycaps` with the resolved specs and export flag. It does not use the Case assembly, rendered meshes, or another scene as manufacturing authority.
- [ ] A successful current result downloads `<document name>-keycaps.step` with `model/step` media type and a valid `ISO-10303-21;` signature. The existing CAD builder retains stable `keycap:{id}` and `keycap-legend:{id}` preview body IDs.
- [ ] The independent export owner validates session/document/board/token/revision before and after each await. Cancellation, changed scope/revision, worker failure, or page disposal prevents stale bytes from reaching download; synchronous CAD work may finish inside the worker but its late reply is suppressed.
- [ ] Use a byte-retained representative fixture; record source/build provenance, project/board/revision, Core findings/spec count, worker request identity, filename/media type, STEP size/signature, and browser errors against pinned React.
- [ ] The existing worker still demonstrates chunk/yield cancellation for preview generation before the private bridge ticket is fully closed. This STEP packet does not implement preview or the shared 3D viewer.
- [ ] Record the exact page/worker source boundary and current owner. The page binary uses one dedicated `CadWorker::request_keycaps_step` library method across the crate boundary; the request wire stays private. No public `CadOperation`/`CadRequest` change, generated contract, CAD engine function, or duplicate builder is needed.
- [ ] Update the refactoring register with evidence-backed findings or “No new refactoring takeaway observed.” Required integrated independent review and broader F6C.5/F7.3/F8.4 acceptance remain open; this child does not close them.
