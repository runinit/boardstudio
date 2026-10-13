# 154: Reserve staged source circuit references during mirror planning

Status: resolved
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

- [x] Prepared embed regression first fails at the staged source/mirror reference collision.
- [x] Source reference stays unchanged; selected mirror references avoid existing document and
      staged source conflicts, using the existing allocator; IDs and references stay stable.
- [x] Cover an unrelated-board reference collision, several circuit members, repeated
      reconciliation and subsequent edits from either half; no unrelated Part is renamed.
- [x] Independent Standards/Spec review, meaningful reservation mutations and required checks
      pass; no helper visibility, schema or production route changes.

Root owns full checks/baseline/integration. This closes missing full-composition evidence from146.

## Answer (2026-10-13)

Mirror circuit planning reuses the actual canonical source embed on a private scratch draft,
reserves all prior document references and every staged source member reference, then uses the
existing target allocator. Source naming stays board-scoped, existing Parts are not renamed,
and the global forward uniqueness validator is unchanged. Canonical application reuses that
same source operation. Several members and unrelated-board references are covered explicitly.

The inverse and prepared regressions first failed at the expected staged C1 collision. Source
members keep C1/C2, an unrelated board keeps C3, and mirror members receive C4/C5. The old
standalone-versus-paired reference oracle was corrected explicitly, retaining its ID, definition
and net assertions; the original failure is preserved. A new propagated-pose assertion then
exposed missing durable hosted admission, independently fixed and integrated by 155. With 155
present, source Q1(23,11) derives mirror Q1(177,11), and mirror Q2(177,12) derives source Q2(23,12),
with identities and references stable across repeat reconciliation.

Independent Standards and Spec reviews accept the original scope and pose-only delta; the
pose-coverage finding is closed. Focused inverse hosted 31 and reconciliation 223 tests pass.
The broader mirrored-half baseline passes 369 tests; all four compiled reservation/canonical-op
mutations were killed with exact restoration. Full repo/test/lint passes with all 1,836 native
tests executed; the baseline adds exactly two names. Main evidence: `.evidence/ticket154/`.

No helper visibility, naming policy, schema or production route changed. 111's member-pose and
production activation wiring remains separate. Local integration and post-merge checks are
pending before 111 resumes.
