# 152: Reconcile authored per-cell matrix data and implicit active slots

Status: claimed
Type: build
Blocked by: [150](150-prepare-matrixless-half-exceptions.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [111](111-integrate-mirrored-half-reconciliation.md), [145](145-mirror-new-matrices-on-a-mirrored-board.md)

## Source-proven preparation gap

111's real Core smoke cases refuse ordinary saved synthetic matrices at
`build_fixed_membership_view`: cells with definitionId, variant or assembliesLocal are rejected.
A separate complete-active-cell-vector requirement refuses implicit default cells. `set_matrix`
and `layout::reflected` preserve/use authored cell data; removing fields is not normalization.
Evidence: main `.evidence/ticket152/cell-representation-audit.md` and 111 runtime failed logs.

## Existing policy to reuse

Tickets 18/145 already establish that source-local cell companions have no mirror counterpart
or correspondence, target-local companions remain independent, and the primary key stays linked.
31's linked companion modes/tweaks and 106's explicit Local/Omitted states remain intact.
Preserve all authored cell definition/variant/assembly data, including None versus Some(false).
No new product decision, visibility widening, source geometry or derivation engine is authorized.

## Scope

Extend the existing private fixed-membership/projection seam using the existing matrix
materializer/reflection helpers and exact validated ownership ledger. Fill only implicit active
slots with existing materializer defaults, retaining existing cells and identities. Keep the
current disabled/deleted-cell refusal unless an existing validated membership transition owns it.
Own `core/src/mirrored_half/reconcile.rs`, `reconcile/prepare.rs` and their private tests;
`core/src/matrix/layout.rs` only if the existing reflection helper needs a bounded correction.
Read the ordinary synthetic fixture without stripping fields or changing it to hide the gap.
Root assigns any further path. Keep production hooks, schema/format, public operation vocabulary
and helper visibility unchanged; 111 remains an unmerged activation draft.

## Acceptance

- [ ] Direct regression fails at the representation/membership guard before the fix.
- [ ] Default cells, explicit default/alternate definition IDs, variant values and None/false/true
      local-assembly flags preserve their authored meaning and exact stored fields.
- [ ] Source-only and independent target-local companions coexist with linked primary keys;
      existing linked companion modes/tweaks and child Local/Omitted rules remain unchanged.
- [ ] Implicit active slots normalize without lost authored fields or invented/dropped IDs;
      disabled/deleted or ambiguous ownership remains an atomic located refusal.
- [ ] Repeated reconciliation and prepared Open/edit are stable, including the unchanged modern
      synthetic fixture used by 111; source data and unrelated matrices/pairs remain correct.
- [ ] Independent Standards/Spec review, meaningful mutations and affected/full checks pass.

No activation is included. Root owns baseline/golden updates, full checks, integration and graph
refresh. Source changes are serial with 153 on these shared private paths.
