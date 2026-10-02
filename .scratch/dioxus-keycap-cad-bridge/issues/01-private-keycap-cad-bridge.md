# BND.1: Prove private Dioxus access to existing Rust keycap CAD export

**Parent:** BND.1 — `.scratch/dioxus-frontend-v1/tasks.json`

**What to build:** Prove a private page-side request path that invokes the already packaged Rust keycap CAD export for a current accepted board snapshot, safely scopes results to that snapshot, supports cancellable preview work, and suppresses late STEP results after scope replacement or disposal. This establishes the concrete bridge needed by later keycap 3D and STEP slices.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Use the existing Rust `build_keycaps` WASM export with a representative current fixture and document its existing asset/build inputs and stable cap/legend body identities.
- [ ] Add or prove only crate-private request/host composition around existing worker facilities; do not widen `web::cad_jobs` or other public API/member visibility and do not add a CAD engine function or format.
- [ ] Validate request identity, accepted scope/revision, body/result ownership, worker failure, and disposal. A stale or superseded reply cannot become visible or download.
- [ ] Demonstrate chunk/yield cancellation for preview generation. STEP generation is non-preemptible; prove its eventual late result is suppressed after cancellation, scope/revision change, or disposal.
- [ ] Retain fixture, exact commands/results, and boundary evidence. If current callable capability proves insufficient, stop at the concrete gap and leave dependent keycap CAD acceptance open for a separate public-contract decision; do not invent a facade.
- [ ] Update the refactoring register with evidence-backed findings or “No new refactoring takeaway observed,” and obtain the required preimplementation contract review plus independent Astra review of the integrated proof.
