# 115: Prepare inert mirrored-edit transaction composition

Status: needs-info
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
  duplicate its logic, widen that helper's visibility or add hardware fields to public Unlink.
- Keep common production edit/apply routing untouched until 111. If existing transaction helpers
  prevent an inert private seam, report the exact route as needs-info before altering architecture.
- The sole approved visibility exception is `constraints::resolve_prepared` as `pub(crate)` for this
  inert preparation path. Keep the `constraints` module private and leave `resolve_edit` and all
  existing production call sites unchanged.
- Refusals and no-ops produce no partial accepted mutation. Existing legacy/reuse-board paths retain
  behavior; new native pairs do not acquire legacy property-bag links.

## File ownership

Exclusive: private transaction preparation inside `core/src/lib.rs` or existing private child
modules, private mirrored-half helpers, association validation in `core/src/mechanical/halves.rs`,
and transaction-focused mirrored-half tests. Production call sites and publication/history routing
are read-only. No new EditOperation arms, visibility widening other than the single approved
`constraints::resolve_prepared` `pub(crate)` exception, hardware API fields or goldens.

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

## Inherits from 139

[139](139-generalized-mirrored-constraint-model.md) handed 115 the transaction half of its
criteria: the canonical operation must reach `apply` and both `resolve_edit` calls with framed
constraints in play, and a refusal must leave document, revision and history untouched. The
resolver to call is `resolve_prepared` in `core/src/constraints.rs`: it is a private fn today and
needs a `pub(crate)` widening for 115 (framed records are supplied privately until 111), and the
locked/conflicting-movement refusal fixture (design.md:278) belongs here, where movement can be
expressed.

## Owner decision (2026-10-12)

The owner approved exactly one visibility change for this ticket: make
`constraints::resolve_prepared` `pub(crate)`. Keep the parent `constraints` module private; do not
widen other functions or types, change the `resolve_edit` signature, or edit its existing production
call sites. This is crate-internal access for the inert prepared transaction only; production
activation remains with 111.

## Needs-info (2026-10-12)

The source audit found that the existing lifecycle and forward-reconciliation helpers are private
inside `mirrored_half::reconcile::lifecycle` and `mirrored_half::reconcile`. The transaction
coordinator in `CoreEngine::edit` cannot call them through the current module path. Reusing those
kernels needs a new crate-internal preparation entry point and the minimum visibility path to reach
it; this would exceed the sole approved `resolve_prepared` exception. No source architecture was
changed. Per this ticket's scope rule, pause implementation until the owner either approves that
narrow coordinator seam or chooses a visibility-preserving redesign.

## Inherited from 110 and 145

- Feed the `Canonical.operation` to apply and to both downstream `resolve_edit` calls, and use
  `Canonical.additions` (`PartAddition`) as the independent-Add transition's source/mirror pair.
- `Canonical.matrix_additions` (`MatrixAddition`) plans a new matrix pair with its members;
  the transition that creates the mirror matrix and records its correspondences is
  [145](145-mirror-new-matrices-on-a-mirrored-board.md). Do not mint matrix correspondences here.
- A moved linked companion translates, then forward validation refuses authored placement on a
  linked slot; surface that refusal as the existing validation error (not covered end to end by 110).
- Parts owned by a linked embedded circuit are refused by 110; 112 translates them.
