# Mechanical effective-projection seam and bounded cost

A Standards review of integration source `6509f557c2a16df0a1f5ce19296eeb635df68256` identified a private stateless route for projecting proposed Case mechanical configuration while retaining the existing CAD projection authority. The source review SHA-256 is `2087597b98103465c800ba2e798f5c7b6b11df57c0d06e60828bc2aa4c57bd4d`; this retained note records its findings for RF-005/RF-006.

`AcceptedSnapshot` carries the accepted document and scene Arcs plus identity fields. `captured_case_document` validates the snapshot epoch, document ID and scene revision, then applies existing effective-Case policy. It does not consult or mutate Session, publish acceptance, or make the token authoritative. For a private proposed-document projection, validate the real accepted snapshot, full Scope, proposal ID and revision first; place the proposed canonical document into a short-lived `AcceptedSnapshot`-shaped carrier that shares the original scene Arc/token/epoch, project through `captured_case_document`, and immediately discard the carrier. Return only the projected document/configuration needed for canonical ownership writeback. The carrier is projection input only: it must never enter Session, accepted read models, CAD caches/job preparation, export or operation identity.

For matching contours, use `captured_case_scene` on the original accepted snapshot because this bounded proposal changes mechanical settings while canonical board geometry is unchanged. The helper applies physical-instance reflection and winding reversal. Never project an already-effective document a second time or persist reflected parts. Keep the original snapshot identity for post-await guards and exact Core request/reply correlation.

The copy cost is bounded but nonzero: each public projection calls `effective_case_inputs`, which clones both the full document and scene even when that projection returns only one output. Calling both helpers therefore performs two document and two scene projections, with the opposite output discarded each time, in addition to separately justified proposal/final construction. This is a per-resolution cost observation, not a once-total-copy claim or measured performance result. Avoid per-render calls and unnecessary caller clones.

This seam adds no public API or visibility, no duplicate policy/default-material module and no new authority. Required implementation proof remains: fresh accepted-snapshot/scope checks, canonical and flipped physical-instance behavior, matching contours without double reflection, unchanged accepted document/history, and stale rejection. This review performed no source edit, Cargo run or public acceptance.

## Source locations

- `application/src/session.rs:51` — `AcceptedSnapshot` identity and Arc-carried inputs.
- `web/src/cad_jobs.rs:241-272` — `captured_case_scene` and `captured_case_document` validation/projection entrypoints.
- `web/src/cad_jobs.rs:470` — shared `effective_case_inputs` projection implementation.
- `web/src/runtime.rs:583` — existing contour capture through `captured_case_scene`.
- `web/src/presentation/case_controller.rs:51` — current private controller use of `captured_case_document`.

These locations were inspected as review evidence; this note does not claim the proposed carrier implementation exists or is compiled.
