# 157: Classify matrix membership by identity rather than order

Status: resolved
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

- [x] Compiled regression fails for the expected order-only classification before the fix.
- [x] Reordered identical members compare unchanged; real additions/removals and changed duplicate
      multiplicity compare changed; missing and present-empty remain distinct.
- [x] Real private prepared Splay/Placement on full authored-companion fixtures work from source
      and mirror, preserve authored fields/companion rules and unrelated matrices, and remain
      stable through repeated preparation/Open.
- [x] Both original public111 authored Splay/Placement regressions pass without stripping companions
      or weakening position/isolation assertions.
- [x] Independent Standards/Spec review, compiled mutation probes and affected/full checks pass.
- [x] Root records native-baseline additions, local merge and post-merge checks. No push.

## Comments

Claimed after156 merged at96eaeabe0 with identical candidate tree and postmerge repo/test/lint
passing all1843 native tests; bookkeeping2186dd36. Root owns integration/fullchecks/baseline.

## Answer

One private parent helper compares sorted borrowed member IDs. Both lifecycle and fixed
reconciliation call it; the duplicated ordered-vector predicates are removed. Multiplicity,
missing-versus-empty and existing identity lookup remain intact. No schema, visibility,
production activation or authored-placement guard changed.

The order-only regression compiled and failed before the fix. Three classification tests and
one full authored-companion test pass, covering source/mirror Splay and Placement, exact authored
fields/relative companion pose, stable repeat/Open and unrelated-matrix isolation. All 51 private
preparation tests pass; the integrated111 private suite passes65. Both independent review axes
accept the implementation and equivalent Clippy assertion repair. Four compiled mutation probes
were killed, none survived or failed compilation, with exact restoration.

Fresh full repo/test/lint passes with1,847 native tests. Baseline adds exactly four names and
removes none. Source34681c75 is integrated into isolated111 at a83b158f. The original authored
Placement public test now passes; Splay accepts and passes both absolute position assertions
after correcting its pre-existing expected x sign to the established positive-rotation formula
19 - 21*sin(12deg). This test-only oracle correction is independently reviewed; no assertion,
fixture, tolerance or production behavior was weakened. Full public111 now60/62, with only the
separate Local controller net and associated Local circuit payload failures remaining.

Evidence: main `.evidence/ticket157/` (review, delta1, oracle-delta, mutation-run, full-checks-final,
native-baseline-delta and activation-proof). Initial lint failure is retained; the fresh rerun
passes. Final source13765c3f merged locally into dev at e95d8e6e, with identical candidate and
merge trees. Post-merge repo/test/lint passes with all1,847 baselined native tests executed;
exact HEAD and commands are in post-merge-checks.json/log.111 activation and parent19 remain
open; no push.
