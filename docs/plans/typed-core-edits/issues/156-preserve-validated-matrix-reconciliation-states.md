# 156: Preserve validated matrix reconciliation states across later edits and Open

Status: resolved
Type: build
Blocked by: [152](152-reconcile-authored-matrix-cells.md), [153](153-validate-independent-exceptions-with-matrix-owners.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [111](111-integrate-mirrored-half-reconciliation.md)

## Observed activation failures

111 dependency integration a24e0c64 compiles, but public Core selected-cell removal followed by
a source MoveParts refuses with “The reconciliation matrix does not have fixed active membership”.
The removal itself succeeds and retires durable associations; its stored deleted/disabled cell
is rejected by the next fixed-membership view. Keep that regression; replacing removal with a
different operation does not establish the required later-edit behavior.

The existing public companion override/Open test also refuses “A canonical matrix companion
association is missing” when a Local/Omitted child is stored as an exception without a child
correspondence. Existing matrix identity and exception provenance must be validated without
inventing a source association for unrelated legacy local Parts.

Original REDs and read-only diagnosis: main `.evidence/ticket111/integration/compiler/`
(public integration log and handoff.md). These failures are preparation gaps, not activation
permission gates. 111 remains unmerged.

## Scope and invariants

Own only existing private fixed-membership/projection validation in
`core/src/mirrored_half/reconcile.rs` and established private tests there. Root may assign
existing private preparation tests if needed. Reuse materialization, reflection and lifecycle
semantics; do not add a parallel engine, visibility, schema, operation vocabulary or net policy.

Preserve valid explicit lifecycle tombstones as inactive coordinates, never silently reactivate
them. Missing coordinates still use existing implicit active defaults. Require exact consistent
source/mirror/projected active and inactive membership, unique coordinates and valid owners;
refuse contradictory, malformed, stale or aliased input. Preserve authored cell payload.
This does not permit arbitrary disabled cells or waive 152's malformed-membership refusal.

For Local/Omitted child exceptions, reuse exact Matrix ownership, canonical slot identity and
existing exception semantics. Preserve companion modes/tweaks and Local geometry; no inferred
independent-Part link or unconditional missing-association fallback. Refuse ambiguous source,
wrong-board/slot ownership, inconsistent correspondence and invalid exception data atomically.

## Acceptance

- [x] Private regressions reproduce both failures before production changes.
- [x] Valid selected-cell tombstones survive later source and mirror edits and prepared Open,
      without resurrection, lost authored data or recreated retired associations.
- [x] Local/Omitted matrix child exceptions without a child correspondence reopen correctly
      with exact Matrix provenance and existing companion mode/tweak semantics.
- [x] Invalid disabled/deleted combinations, coordinate/ownership aliases, stale associations
      and foreign/ambiguous exception identities remain atomic refusals.
- [x] Repeated reconciliation is stable and unrelated pairs/matrices remain unchanged.
- [x] Independent Standards/Spec review, meaningful compiled mutations and affected/full checks
      pass; root owns baseline updates, local merge and post-merge checks.
- [x] After integration into 111, original public regressions pass without changing their
      selected-cell operation or weakening companion override/history assertions.

No production activation is included. Keep all work unpushed.

## Answer

The existing private membership view now retains exact paired inactive tombstones, rejects
contradictory flags, mismatched inactive slots and stale generated members/associations, and
continues filling only truly missing active coordinates. Real prepared removal followed by
source and mirror edits, all-slots-inactive state, Open and repeated preparation are covered.

Legacy unassociated Local and Omitted child records follow the existing model contract: their
mirror item remains materialized and has no inferred source. Exact owner/slot validation admits
that state without creating a correspondence. Its original Part payload and pose are carried
through materialization exactly; serialized pair/exception data remains stable. Associated
Omitted lifecycle semantics remain unchanged. No production route, schema or visibility changed.

Both independent review axes accept the implementation and test-only Open follow-up; the initial
Spec finding for missing legacy-child prepared Open proof is closed. The final focused suite
passes 230 tests. Seven compiled mutation probes ran: six were killed, and one local validation
perturbation survived without changing valid-document geometry or persisted exception metadata;
that result is retained and is not counted as a kill. All sources restored exactly.

Full `check.py repo test lint --update-native-baseline` passes with 1,843 native tests. The baseline
adds exactly seven new test names and removes none. Candidate code was also integrated into the
isolated 111 draft: both original public regressions now pass unchanged, while five unrelated 111
failures remain visible. This does not claim full 111 or parent 19 acceptance.

Evidence: main `.evidence/ticket156/` (handoff, review initial/delta1, mutation-run and honest
summary, full-checks, native-baseline-delta and activation-proof). Local dev integration and
post-merge checks are the next coordinator steps; nothing is pushed.
