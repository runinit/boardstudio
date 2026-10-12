# 111: Atomically activate prepared mirrored-half reconciliation

Status: ready-for-agent
Type: build
Blocked by: [117](117-prepare-mirrored-preview-history-performance.md)
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
- Stop/report if connection needs new derivation rules, missing operation-family support,
  new context APIs or significant transaction redesign. Those are preparation blockers,
  not extra implementation hidden inside this activation ticket.

## File ownership

Exclusive: bounded call-site/dispatch/vocabulary edits in `core/src/lib.rs` and
`core/src/model.rs`, necessary private helper wiring, a small set of Session/Core smoke cases
in established mirrored-half test files. No new subsystem implementation, storage/schema
migration, preview backup engine, performance harness, golden edits or visibility changes.
Root assigns any additional path before editing.

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
