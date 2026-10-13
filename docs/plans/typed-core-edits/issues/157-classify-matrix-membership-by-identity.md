# 157: Classify matrix membership by identity rather than order

Status: claimed
Type: build
Blocked by: [156](156-preserve-validated-matrix-reconciliation-states.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [111](111-integrate-mirrored-half-reconciliation.md)

## Observed failure

Full authored-companion SetMatrixSplay and SetMatrixPlacement public previews in111 refuse as
unsupported structural transitions. Pair creation appends companion IDs after keys; set_matrix
rebuilds the same member identities in cell order. Two duplicated private predicates compare
ordered part_ids vectors, selecting structural preparation for geometry-only edits. Evidence:
main `.evidence/ticket111/integration/authored-matrix-pose-diagnosis.md` and ticket157 readiness notes.

## Scope

One private membership-classification helper in reconcile.rs, consumed by its lifecycle and
prepare child modules. Compare exact identities independent of ordering, retaining duplicate
multiplicity and missing-matrix versus present-empty distinction. Preserve identity lookup and
existing ownership validation. No stored-field normalization, schema, production activation,
visibility widening or new placement policy. Keep the structural authored/constrained-placement
guard unchanged; diagnose any distinct fixed-reconciliation refusal separately.

## Acceptance

- [ ] Compiled regression fails for the expected order-only classification before the fix.
- [ ] Reordered identical members compare unchanged; real additions/removals and changed duplicate
      multiplicity compare changed; missing and present-empty remain distinct.
- [ ] Real private prepared Splay/Placement on full authored-companion fixtures work from source
      and mirror, preserve authored fields/companion rules and unrelated matrices, and remain
      stable through repeated preparation/Open.
- [ ] Both original public111 authored Splay/Placement regressions pass without stripping companions
      or weakening position/isolation assertions.
- [ ] Independent Standards/Spec review, compiled mutation probes and affected/full checks pass.
- [ ] Root records native-baseline additions, local merge and post-merge checks. No push.

## Comments

Claimed after156 merged at96eaeabe0 with identical candidate tree and postmerge repo/test/lint
passing all1843 native tests; bookkeeping2186dd36. Root owns integration/fullchecks/baseline.
