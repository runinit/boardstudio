# 08: Reject missing prepared input at the CAD worker receiver boundary

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)

**What to build:** A raw CAD worker Preview frame with neither keycaps nor prepared case input returns the existing failed-reply protocol instead of reaching `prepared.expect("request validated")`. The queue settles and the same worker remains usable for a subsequent valid request.

## Research decision (2026-10-19)

Narrow to a reachable receiver-boundary defect. The page client validates input, but `receive_worker_message` accepts raw `WireRequest` frames and does not establish prepared-input presence before `run_request`'s Preview branch. A client-only validation test does not cover this boundary. Use existing error/reply types and preserve captured request/job/snapshot identity; no new error hierarchy. See [research 08](../../../research/rust-best-practices-ticket-validation.md#08--bounded-cleanup-not-a-blanket-panic-purge).

Drop the broad panic inventory: the inspected keycaps, circular-radius, KiCad net, Session, electrical, archive and mechanical lookups have guards or invariants. Local bindings may improve nearby code when it is otherwise touched, but are not required bug fixes. Likewise the OutlineChange wildcard is an enum-evolution concern, not a demonstrated malformed-input path; if later changed, enumerate variants explicitly rather than hiding future variants behind a generic error. The CAD jobs mismatch path can be assessed separately with a reachable protocol counterexample. Do not expand this worker fix into those refactors.

Tickets 01/02 do not supply this boundary validation; 07 provides wider lint coverage but is not a prerequisite. Keep the existing filename to preserve ticket links.

## Acceptance criteria

- [ ] A raw worker Preview frame with no keycaps/prepared input produces a descriptive failed reply carrying the captured request ID, job ID, operation and snapshot identity, without a WASM panic or unresolved pending request.
- [ ] Worker/browser regression tests bypass the page-client validator, observe queue settlement, then complete a valid request on the same worker. Retain successful prepared-case and keycap Preview controls.
- [ ] Use the current worker failure path, cancellation semantics and error types; unrelated guarded unwraps and protocol/enum redesigns stay out of scope.
- [ ] Native host tests, affected worker/browser tests and native/WASM lint checks pass. Record the focused commands and evidence; do not claim client-only tests exercise the raw receiver.
