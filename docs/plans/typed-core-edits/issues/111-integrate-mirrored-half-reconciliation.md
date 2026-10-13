# 111: Atomically activate prepared mirrored-half reconciliation

Status: claimed
Type: build
Blocked by: [117](117-prepare-mirrored-preview-history-performance.md), [146](146-mirror-side-circuit-embeds-land-as-typed.md), [147](147-mirror-keycap-settings-and-plain-nets.md), [148](148-mirror-electrical-configuration-for-added-matrices.md), [149](149-targeted-mirrored-set-mechanical.md), [150](150-prepare-matrixless-half-exceptions.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Connect verified inert preparation from 105–110 and 112–117 to production in one coherent
activation. This ticket does not build another reconciliation/translation/lifecycle engine
and does not perform parent 19's complete acceptance review.

## Prerequisites and bounded activation

- All preparatory children are independently resolved. 109's nonempty-closure Unlink route
  and 114's compatibility/constraint design must be concretely settled, not waived.
- Wire prepared canonicalization/draft composition before accepted edit publication, Open
  repair before final Open validation, and prepared paired preview isolation/history behavior.
- Add the agreed `SetHalfException`/`UnlinkHalves` EditOperation vocabulary and dispatch to
  already verified lifecycle helpers. Derive required Unlink context internally via 109's
  reviewed callable route; no public hardware fields or visibility widening.
- Keep existing unpaired/legacy/reuse-board behavior. Ensure downstream resolution receives
  canonical operation and final validation sees reconciled state. No intermediate partial
  production activation or per-domain feature flags.
- Route explicit Instance-targeted `SetMechanical` through 149's preparation even when the target
  is unpaired or disabled. A native-pair-only dispatch condition would leave that approved request
  at the raw apply refusal. Omitted targets retain their existing board-slot route.
- Stop/report if connection needs new derivation rules, missing operation-family support,
  new context APIs or significant transaction redesign. Those are preparation blockers,
  not extra implementation hidden inside this activation ticket.

## File ownership

Exclusive: bounded call-site/dispatch/vocabulary edits in `core/src/lib.rs` and
`core/src/model.rs`, necessary private helper wiring, a small set of Session/Core smoke cases
in established mirrored-half test files. The inherited 139 framed-constraint persistence and
v5→v6 migration below are part of this activation; no additional storage/schema migration,
new subsystem implementation, preview backup engine, performance harness, golden edits or
additional visibility changes beyond the inherited serialized-model interface and the explicitly
approved Open entry point specified below.
Root assigns any additional path before editing.

Root assignment (2026-10-13 readiness audit): the existing approved inherited work also owns
`core/src/constraints.rs`, `core/src/constraints/framed_tests.rs`,
`core/src/constraints/relation.rs`, `core/src/constraints/endpoints.rs`,
`core/src/matrix/deletion.rs`, `core/src/migrate.rs`,
`core/src/archive.rs`, `core/tests/part_offset_constraint.rs`, and
`web/crates/host/src/host/storage_lifetime_tests.rs` for exhaustive framed handling, arithmetic
pins, migration and current-version/round-trip coverage. Existing private operation wiring owns
`core/src/mirrored_half/lifecycle.rs`, `core/src/mirrored_half/reconcile.rs`, and
`core/src/mirrored_half/reconcile/prepare.rs` for exception dispatch and circuit `member_poses`.
This assigns paths for already specified requirements; it adds no new visibility grant or
derivation policy. Inspect any additional exhaustive-match errors before expanding ownership.

The accepted 139 design explicitly keeps names private unless they are fields of the serialized
model, and explicitly authorizes the read-only `ProjectDoc::constraint_driving_part` facade.
Persisting `Constraint::Framed` therefore includes publishing its `ConstraintEndpoint` and
`ConstraintRelation` data types through `model`; this is the inherited model interface, not a
new helper visibility grant. Keep the compiled `FramedConstraint` candidate, private constraints
module, compiler/solver records and domain helpers private. These exact paths/interfaces are
assigned here so activation does not invent an additional permission gate for already approved
139 work; any further helper visibility widening still requires the owner's approval.

## Test-first and acceptance

- [ ] Production-seam smoke cases first fail because runtime edit/Open hooks are absent.
- [ ] One representative rotated-map linked edit from mirror, local/Omit/Rejoin/Unlink
      (including nonempty linked closure context), repaired Open and paired preview reaches
      the prepared paths atomically; one Undo/Redo restores both halves.
- [ ] Refusal publishes no partial data; unchanged legacy/unpaired behavior remains covered.
- [ ] No new reconciliation or operation-family logic was required to activate; every helper
      and compatibility decision has independent preparatory verification evidence.

Independent verifier: mutation-check each newly connected runtime route, including canonical
request propagation and refusal publication. Run affected checks in root-granted slots.
Root then schedules a **separate fresh verifier for parent 19's entire acceptance**: all item
kinds/both directions, save/reopen, exceptions/unlink, full preview/history matrix, actual
activated performance and native/WASM boundary evidence. Parent remains open until that
verification holds; activation smoke coverage alone cannot resolve it. Root owns full batch
check, transcript blessing, integration and graph refresh.

## Inherits from 139

[139](139-generalized-mirrored-constraint-model.md) deliberately did not add the persisted
`Constraint::Framed` arm or bump the format: the accepted design lands both here, together, so an
older build refuses a framed document instead of dropping the record. The private serde shapes
are `FramedConstraint`/`ConstraintEndpoint`/`ConstraintRelation` in `core/src/constraints/`; a
test there pins that a framed record is not storable today. 139 covered: the stored solver
arithmetic is untouched and the private serde shapes exist. 111 still owns 139's bullet 5, the
original wording of which is "Existing horizontal/vertical Mirror and Offset documents preserve
behavior and remain loadable; serialization/version/migration choices are tested where
required", and so: the v5 to v6 stamp-only migration candidate and its idempotence on current
text, a framed serde round-trip with both endpoint kinds and `mirror-line` (design, "Independent
synthetic geometry oracles" and the compatibility tests), the v7-rejection test (the current
newer-version input 6 becomes 7), the current-version assertions, and the archive/host
round-trip tests. Before the `Framed` arm persists, the companion and mirror-line arithmetic
in `core/src/constraints/` must stay bit-pinned: positions and headings are persisted and
hashed, so a reordered operation or a folded zero is a format change. Today
`companion_rules_are_bit_pinned_to_the_arithmetic_that_was_reviewed` (companion rules, forward
and inverse, with negative zeros) and `axis_aligned_mirrors_reproduce_the_stored_solver_bit_for_bit`
pin it; the slanted-axis reflection is pinned only to 1e-9 and needs its own literal bit
patterns before it persists.

## Inherited from 110 and 145

- Consume `inverse::canonicalize` (110) for every translated variant; its `Coverage` ledger says
  which variants are translated, delegated (`ApplyMechanicalEdit`), deliberately unchanged
  (bindings) or owned by 112-114. Wire matrix additions only together with
  [145](145-mirror-new-matrices-on-a-mirrored-board.md): `matrix_additions` is a plan, not a
  transition.
- A linked companion `MoveParts` translates geometrically but the forward validation refuses
  authored companion placement on linked slots; that refusal is untested end to end and
  surfaces here and in 115.

## Note from 146's review: apply circuit `member_poses`

112's mirror-side circuit refusal is gone (146). A prepared mirror-side `EmbedModuleCircuit` is
therefore only correct when `Canonical::hosted.circuits[].member_poses` is applied inside the
**same prepared transaction as the source embed, before forward projection**. Applying the
canonical embed alone silently mis-places mirror members. `HostedTransition::AddCircuit`
(`reconcile.rs`) carries ids and references only, so 111 must extend it with the member poses.

Acceptance: a mirror-side embed with members at local `y != 0` and a nonzero heading, on Front and
Back, driven through the real prepared path and `reconcile`, lands every member as typed; a later
edit from either half round-trips. Also cover a typed heading that wraps past 0/360 and a
definition member that is already on the back side (146's unit test does not).

## Readiness audit (2026-10-13)

115–117 are merged but remain inert. For matrix-backed exceptions, the existing
`prepare_edit_transition` adapter can compose the tested private `transition_exception_context`
and `prepare_lifecycle_transition` in the same module when the agreed operation vocabulary is
added; no new interface is needed for that wiring. Unlink uses the reviewed
`mechanical::halves::unlink_pair` route.

Do not infer complete exception coverage from that path: [150](150-prepare-matrixless-half-exceptions.md)
prepares the missing matrixless path. `prepare_edit_transition` currently
returns without a transition when the pair has no Matrix correspondence, while prepared Open
already supports matrixless pairs with linked independent Parts. Resolve matrixless Local/Omit/
Rejoin through existing lifecycle semantics before full activation under 150's private scope.
The inherited v6 work and circuit pose wiring remain incomplete; this audit is source evidence,
not activation acceptance.

## Electrical readiness follow-ups from 147

- **Circuit reference to a newly private plain net:** `reconcile/settings.rs` skips plain-net
  projection whenever either net ID appears in any embedded circuit's `net_ids` or `ports`.
  The private-net predicate in `mechanical/halves.rs` counts source pins, not circuit references;
  separately, lifecycle Omit treats a circuit `ports`/`net_ids` reference as use. There is no
  current runtime case proving the expected result when a Local or mirror-only circuit names a
  corresponded plain net whose source becomes private. Add that bounded case before declaring
  this edge settled. Do not infer a policy or waive the discrepancy from these implementation
  details.
- **Mirror-authored pin on a linked controller:** 147's accepted projection rule drops this
  mirror-only pin when projecting from the source; the existing private-net test covers only a
  mirror-owned pin on an uncorresponded part. Add a paired-preview assertion that makes this
  loss visible. This is evidence for the known rule, not a request to change ownership semantics.
- **Removed-net copper:** 147 explicitly leaves copper referencing a removed private net alone;
  validation does not check trace/via `netId`. Parent acceptance, refusal and preview review must
  account for that acknowledged outcome without inventing copper cleanup policy.
- **Registration and names:** raw `SetNet` registers on the document's first board; the residual
  non-first-board registration issue is general editor behavior. A linked mirror `SetNet` is
  inverse-mapped to its source net before apply. Existing-net name ownership is already decided
  by 147/ADR-0010; these are not new permission gates.

If resolving either pending evidence item requires a new ownership, net-lifetime or preview rule,
stop 111 activation and delegate that bounded preparation; do not add the rule inside activation.
These notes record follow-up evidence only and complete no acceptance item.

## Approved Open entry point (2026-10-13)

The owner approved making the existing `mirrored_half::reconcile::prepare::prepare_open`
`pub(crate)` and re-exporting it through the existing private `reconcile` and `mirrored_half`
modules. This three-line bridge lets production Open consume 116's verified source-authoritative
draft and located repair findings. It does not grant visibility changes to other helpers or
modules, and Open remains separate from edit-transaction canonicalization. The reviewable
proposal is recorded in `.evidence/ticket111/open-seam-proposal.patch`; 111 applies the bridge
with its coherent production activation after 150 is verified.

## Activation ownership (2026-10-13)

All preparatory prerequisites through 150 are merged locally; 150's post-merge repository,
native-test and lint gates passed (1,821 tests). This branch claims coherent 111 activation.
The model/persistence writer owns the assigned model, constraints, migration, archive and host
storage paths above, plus `core/tests/current_schema.rs` for the known current-format assertion
and `core/src/matrix/layout.rs` / `placement.rs` only for the accepted authority-aware driver
queries. The runtime writer owns lib and mirrored-half dispatch/preparation/inverse paths,
established mirrored-half smoke tests, and `core/tests/support/boundary.rs` /
`core/tests/boundary_parity.rs` for the additive paired scenario. Root retains the golden, full
checks, mutation batch, performance/browser verification and local integration.

Writers share one branch with disjoint file ownership. The model writer first publishes the
exact operation/Framed data shapes and compiler inventory; runtime wiring starts from that
readiness handoff. No intermediate production activation is merged. These assignments add no
visibility beyond the inherited serialized model/read-only facade and approved Open entry point.
New preparation/policy needs must be reported rather than hidden in activation.

Model readiness: SetHalfException's Rust item field remains `kind: MirroredItemKind`, but its
wire key is `itemKind` because JSON `kind` is already the EditOperation discriminator. Its
`HalfExceptionState` accepts Local/Omitted/Linked without adding Linked to stored exception
overlays. This is a serialization clarification of the agreed vocabulary, not a new behavior.
