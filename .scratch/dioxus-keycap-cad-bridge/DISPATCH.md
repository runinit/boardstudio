# Dispatch contract — BND.1 private keycap CAD bridge

## Ownership and seam

This ticket has no start or acceptance dependencies in the canonical 62-task graph. Before implementation, perform and record a source contract review that names the exact page-binary/library/worker callable seam, current owner crate, accepted snapshot inputs, and result path to the already packaged Rust keycap builder. The page binary and the publicly imported `boardstudio_web` library are separate crate boundaries; a library `pub(crate)` function is not callable from the page binary. Prove a private consumer-crate wrapper or an existing sufficient facade before coding.

The current web CAD operation enum/request is already public. Do not add a `CadOperation` variant, request field, public member, or visibility change. Do not add a CAD engine function, format, or duplicate builder. If the packaged `build_keycaps` export cannot be reached within the present contracts, stop with the smallest precise boundary proposal; dependent keycap CAD work remains open for an explicit separate decision.

## Source and behavior

Parent requirements are in `.scratch/dioxus-frontend-v1/tasks.json`; the existing implementation is `cad/wasm/src/model/keycaps.rs` and its export from `cad/wasm/src/lib.rs`. Record the builder's exact asset/build inputs and stable `keycap:{id}` and `keycap-legend:{id}` body identities. `scripts/build-m1.py` is relevant to packaged module provenance. Current `web/src/cad_jobs.rs`, `web/src/cad_worker.rs`, and `web/src/runtime.rs` are case-oriented and do not by themselves demonstrate the required keycap path.

Prove identity and ownership against the accepted board snapshot and scope/revision. Validate worker failure and disposal. Preview work must demonstrate chunk/yield cancellation; STEP generation is non-preemptible, so demonstrate that its eventual reply is suppressed after cancellation, scope/revision replacement, or disposal and can neither render nor download. Do not claim ResolveKeycaps or generic case STEP establishes this bridge.

## Checks and evidence

Retain a reproducible representative fixture, source/build provenance, exact commands/results, boundary proof, cancellation and late-reply traces, and independent Astra review of the integrated proof. Apply the shared acceptance at `../dioxus-frontend-tranche-1/ACCEPTANCE.md` and the relevant lifecycle, stale-result, error, and repository checks. This ticket proves a private existing-capability path only; it does not close later F6C.5/F7.3/F8.4 acceptance or authorize a public API change. Update the existing refactoring register or record “No new refactoring takeaway observed.”
