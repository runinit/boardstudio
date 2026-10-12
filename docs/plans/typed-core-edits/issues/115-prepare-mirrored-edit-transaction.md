# 115: Prepare inert mirrored-edit transaction composition

Status: ready-for-agent
Type: build
Blocked by: [114](114-mirrored-edit-compatibility-constraints.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Compose the verified forward kernels, exception helpers and inverse adapters into an inert
transaction preparation path. Final production routing belongs only to 111.

## Scope and evidence

`core/src/lib.rs::{edit,apply}` at `2478fb151` clone/apply/resolve/prune before publishing.
`constraints::resolve_edit` consumes the operation after apply, so the canonicalized request
must reach downstream resolution too. Halves validation must inspect the final reconciled draft.

- Prepare private draft composition: canonicalize request, execute existing operation,
  resolve final authored layout/constraints/components, reconcile all linked domains,
  clean removed associations, validate, and compute actual changed IDs.
- Include exception/Unlink helper composition only through 109's source-verified callable route
  and internally derived context. Do not call a private mechanical dissolution helper directly,
  duplicate its logic, widen visibility or add hardware fields to public Unlink.
- Keep common production edit/apply routing untouched until 111. If existing transaction helpers
  prevent an inert private seam, report the exact route as needs-info before altering architecture.
- Refusals and no-ops produce no partial accepted mutation. Existing legacy/reuse-board paths retain
  behavior; new native pairs do not acquire legacy property-bag links.

## File ownership

Exclusive: private transaction preparation inside `core/src/lib.rs` or existing private child
modules, private mirrored-half helpers, association validation in `core/src/mechanical/halves.rs`,
and transaction-focused mirrored-half tests. Production call sites and publication/history routing
are read-only. No new EditOperation arms, visibility widening, hardware API fields or goldens.

## Test-first and acceptance

- [ ] Direct prepared-draft red/green tests exercise canonical request through downstream
      resolution, final reconciliation/validation and precise source/mirror/membership IDs.
- [ ] Unrelated edits and ReplaceDocument cannot leave dangling association records.
- [ ] Refused preparation leaves accepted bytes unchanged; source/mirror objects are not
      resolved from stale pre-apply inputs; production route remains unchanged.
- [ ] JSON-equivalent `UnlinkHalves {pairId}` preparation follows 109's existing callable
      mechanical edit route with internally normalized context. Nonempty new-board/reuse-board
      closure fixtures preserve effective/stored geometry and exactly-once hole memberships;
      no caller-supplied hardware field or empty-context shortcut is used.

Independent verifier: mutate canonical operation propagation, reconciliation/cleanup order and
final validation separately. History/Open/preview are deliberately owned by 116/117.
