# 154: Reserve staged source circuit references during mirror planning

Status: claimed
Type: build
Blocked by: [152](152-reconcile-authored-matrix-cells.md), [153](153-validate-independent-exceptions-with-matrix-owners.md), [155](155-admit-hosted-addition-correspondences.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [146](146-mirror-side-circuit-embeds-land-as-typed.md), [111](111-integrate-mirrored-half-reconciliation.md)

## Source-proven preparation gap

111's mirror-authored Haptic circuit embed refuses “The selected circuit member reference C1
is already in use”. Inverse planning allocates against prior mirror-board references before
source embed; forward AddCircuit validates against all staged Parts, including the new source
member C1. Existing ordinary circuit allocation is host-board scoped; mirrored derivation already
reserves document-wide references. Evidence: main `.evidence/ticket154/reference-audit.md`.

## Scope

Reuse the existing canonical source embed/allocator to determine staged source references;
reserve all relevant existing document references plus those staged source references when
planning mirror references. Keep source embed output, the existing naming rule and the global
forward validator unchanged. Do not invent a naming policy, relax uniqueness, rename existing
Parts or widen visibility. Own `core/src/mirrored_half/inverse/hosted.rs`, its private tests and
`reconcile/prepare/tests.rs` for the prepared-path regression. Root assigns any further path.
Keep production routing and member-pose activation wiring in 111.

## Acceptance

- [ ] Prepared embed regression first fails at the staged source/mirror reference collision.
- [ ] Source reference stays unchanged; selected mirror references avoid existing document and
      staged source conflicts, using the existing allocator; IDs and references stay stable.
- [ ] Cover an unrelated-board reference collision, several circuit members, repeated
      reconciliation and subsequent edits from either half; no unrelated Part is renamed.
- [ ] Independent Standards/Spec review, meaningful reservation mutations and required checks
      pass; no helper visibility, schema or production route changes.

Root owns full checks/baseline/integration. This closes missing full-composition evidence from146.
