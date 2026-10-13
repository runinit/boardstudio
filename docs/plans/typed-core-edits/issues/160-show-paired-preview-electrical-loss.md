# 160: Show electrical loss in paired Preview through transient findings

Status: claimed
Type: build
Blocked by: [159](159-preserve-associated-local-circuit-owner-state.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [111](111-integrate-mirrored-half-reconciliation.md), [147](147-mirror-keycap-settings-and-plain-nets.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Contract

Before Commit, show the exact pins removed by a prepared paired Preview, including Part ID/reference, pad ID and net ID/name. If the staged result removes the net or a board registration, show those actual removals too. A changedIds signal and later Commit proof do not establish visible Preview loss. Preserve existing net retention semantics, accepted document/revision/history/readiness, and ordinary unpaired Preview.

Use existing SceneDelta.findings with Info severity and Pcb scope. Derive notices from accepted versus already reconciled staged data; no second resolver, net policy, public model field/type, visibility grant, or Preview lifecycle. Notice IDs must be stable and unambiguous for arbitrary IDs. Notices are passive; do not weaken accepted finding navigation guards. Do not require machine consumers to parse messages.

## Preparation and activation split

On dev, prepare a private inert notice builder and its tests in core/src/lib.rs plus existing test files, and the existing UI consumer adapter. Root connects the helper only in isolated111's paired prepared Preview branch. Coherent111 activation remains unmerged until native/public/WASM/browser acceptance and final review pass.

Existing Session display_preview already transports and clears SceneDelta. Use its transaction/revision ownership guards. No accepted snapshot mutation or replacement transient state engine is needed.

The existing findings list stays based on the accepted scene. Append only reserved
`preview:electrical:` notices from transient display_preview for the inspector and footer.
Do not promote ordinary Preview-only diagnostics or change unpaired finding presentation.
The activated page test may add its captured private JSON fixture beside the existing page
module; keep fixture provenance and complete its unrelated synthetic definition defects
explicitly, without suppressing validators.


Assigned Core preparation scope: core/src/lib.rs, private notice builder and inline tests.
Keep the builder test-only on dev with cfg(test); root removes that guard and invokes it only
in isolated 111's prepared paired Preview branch. No dead-code suppression or visibility grant.
Assigned application scope: application/tests/durable_session.rs; application/src/session.rs
only for an actual compiled regression in existing notice ownership/clearing, no new lifecycle.
Assigned UI scope: web/src/presentation.rs and web/crates/layout/src/layout_findings.rs including
its existing component tests. Draft the full Editor browser case for
web/src/presentation/layout_layers_tests.rs as an evidence patch until root applies it in
isolated 111 with the notice producer active. Do not add a failing full-page case on inert dev.
keycaps_fit::presented_findings already includes Info; no shared keycaps helper change expected.
Root owns ticket/map/handoff, baseline, integration, public Core assertions and activated
browser/WASM proof. No production routing or public model changes on dev.

## Acceptance

- [ ] Exact pin-loss notices include whole-net removals; no removed net/registration notice when a surviving Local circuit reference retains them.
- [ ] Notice generation is deterministic, collision-safe and inert for retained pins/nets/registrations; unpaired Preview unchanged.
- [ ] Original public controller Rejoin case shows exact pin/net/registration loss in Preview and retains accepted isolation/history assertions, then independently checks Commit.
- [ ] Original public Local circuit Omit case shows linked-pin loss while its net and registration remain; no policy/oracle weakening.
- [ ] Session delivers then clears notices, and cancellation/stale replies cannot restore or clear a newer Preview's notices.
- [ ] Actual mounted browser displays exact Part/pad/net notices before Commit; clear/cancel removes them. An Info-only scene exposes the findings panel/footer. Preview rows have no navigation action; accepted diagnostic navigation remains guarded.
- [ ] Existing native/WASM boundary and serde proof confirms transport; independent Standards/Spec review, meaningful mutations and affected/full checks pass.
- [ ] Local integration and postmerge evidence recorded; no push.

## Comments

Claimed after ticket 159 resolved with both final review axes and post-merge repository/test/lint passing all 1,856 native tests. Local dev bookkeeping is `7acda250`. Coordinator decision and existing consumer inventory are in main .evidence/ticket160/coordinator-decision.md. Selected existing Findings interface fulfills the authorized visible-loss requirement without a new public contract or visibility. Stop for concrete approval if implementation needs a new visibility/API grant or ownership/Preview policy.
