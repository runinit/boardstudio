# Dispatch contract — BND.1 private keycap CAD bridge and STEP action

## Ownership and seam

This ticket has no start dependencies in the canonical 62-task graph. It supplies the existing BND.1 prerequisite for F6C.5 and implements the React Keycaps-panel STEP action against an accepted board snapshot. The existing Rust CAD WASM package exports `build_keycaps`, and the page's `CadWorker` already initializes that package in an independent module worker. Keep the adapter private in the page/worker composition; do not widen `boardstudio_web` or generated contracts.

Do not add a CAD engine function, format, or duplicate builder. A private worker wire shape may carry the existing `KeycapSpec` values and accepted snapshot identity to `build_keycaps`; it must remain distinct from public `CadOperation`/`CadRequest` types. This implementation is authorized to add private request and host composition needed to complete the download action.

## Source and behavior

Parent requirements are in `.scratch/dioxus-frontend-v1/tasks.json`; the existing implementation is `cad/wasm/src/model/keycaps.rs` and its export from `cad/wasm/src/lib.rs`. The builder accepts the current revision and Core-resolved specs, returns STEP bytes when `export` is true, and includes stable `keycap:{id}` and optional `keycap-legend:{id}` preview body identities. `scripts/build-m1.py` is relevant to packaged module provenance.

Resolve the current board through Core `ResolveKeycaps` with `cases: null`, matching `app/src/exports/keycaps.ts`; preserve its revision, errors, empty-spec message, filename, STEP MIME type, and delivery behavior. Prove identity and ownership against the accepted board snapshot and scope/revision before and after every await. STEP generation is synchronous/non-preemptible inside the CAD worker; cancellation, scope/revision replacement, or disposal must close or invalidate the export-owned worker and suppress its late reply so it cannot download. Do not use the current Case scene as manufacturing input. F6C.5's shared Keymap/Keycaps 3D viewer, preview, layer, retry, and full user-journey criteria remain open.

## Checks and evidence

Retain a reproducible representative fixture, source/build provenance, exact command/results, worker and stale-owner evidence, and actual downloaded STEP bytes. Compare the pinned React action/readiness/errors and verify the STEP signature and stable body identity coverage from the existing CAD builder. Apply the shared acceptance at `../dioxus-frontend-tranche-1/ACCEPTANCE.md` only to this slice; the full Keycaps/Keymap 3D viewer and broader F6C.5/F7.3/F8.4 acceptance remain open. Update the existing refactoring register or record “No new refactoring takeaway observed.”
