# 152: Reconcile authored per-cell matrix data and implicit active slots

Status: resolved
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

- [x] Direct regression fails at the representation/membership guard before the fix.
- [x] Default cells, explicit default/alternate definition IDs, variant values and None/false/true
      local-assembly flags preserve their authored meaning and exact stored fields.
- [x] Source-only and independent target-local companions coexist with linked primary keys;
      existing linked companion modes/tweaks and child Local/Omitted rules remain unchanged.
- [x] Implicit active slots normalize without lost authored fields or invented/dropped IDs;
      disabled/deleted or ambiguous ownership remains an atomic located refusal.
- [x] Repeated reconciliation and prepared Open/edit are stable, including the unchanged modern
      synthetic fixture used by 111; source data and unrelated matrices/pairs remain correct.
- [x] Independent Standards/Spec review, meaningful mutations and affected/full checks pass.

No activation is included. Root owns baseline/golden updates, full checks, integration and graph
refresh. Source changes are serial with 153 on these shared private paths.

## Answer

The private fixed-membership view now preserves authored cell definitions, variants and explicit
local-assembly flags. Independent local companions retain their own hardware and membership;
linked keys and existing companion modes/exceptions continue through the shared materializer.
Only implicit active cells are filled with materializer defaults, retaining complete authored
source records and source order. Both halves normalize sparse inputs and repeated reconciliation
reports no changes. No production route, schema or helper visibility changed.

The original representation/sparse regressions failed behaviorally before the fix. Independent
Standards and Spec reviews accepted the frozen change and subsequent coverage/lint-only deltas.
The final focused reconciliation suite passes 214 tests, including generated alternate-definition
parts, distinct target-local hardware, and the unchanged modern synthetic fixture through private
Open/transaction-preview preparation with accepted-document isolation.

Nine compiled mutation probes were killed with exact restoration and no survivors or compile
failures. Those probes used the reviewed behavior snapshot before three equivalent lint cleanups;
the final delta has both-axis review and full verification. `check.py repo test lint` passes with
all 1,826 native tests, and the five new names were registered using the baseline command.
Evidence is in main-checkout `.evidence/ticket152/` (gitignored). Local integration/post-merge
verification is recorded in the handoff/map; nothing is pushed. Ticket 111 stays unpublished
while 153/154 finish their independent preparation contracts.

Local merge: `39f15a5e` (source `e72f7307`). The merged Git tree and reviewed source hashes
match the candidate exactly; post-merge repo/test/lint passes with all 1,826 native tests.
Two accidental duplicate task patches in the primary checkout were preserved in local recovery
commit `7ce6b130` before integration; none was discarded or activated outside its task.
