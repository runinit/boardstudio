# 28: Aggregate anchor Core (shape, evaluation, Follow)

Status: claimed
Type: build
Blocked by: none (decisions locked in [deferred-items-plan.md](../deferred-items-plan.md))
Parent: [deferred plan](../deferred-items-plan.md) · Research: [deferred-items-research.md](../deferred-items-research.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Core eval/kernel slices A1, A3, A4, A7 only. Part validation (A2),
lifecycle (A5), reflection (A6), fixtures (A8) and UI (A9) belong to later
tickets. The GLOSSARY `Matrix anchor` entry (singular key) needs a
`domain-modeling` update for the aggregate arm, per the map's standing rule.

## Outcome

`MatrixAnchor` gains an aggregate arm —
`anchor.target: { single: {matrixId,row,column} } | { aggregate: { method, parts: [single…] } }`
with top-level `follow` unchanged — and the kernel evaluates `average`
(arithmetic x/y/r mean, upstream naive rotation mean) and `intersect`
(heading-axis intersection, rotation 0) over single-key parts. `follow`
filters the completed aggregate candidate exactly as today.

## First RED / GREEN slice

Fail a public `Open`/`Edit` test creating a 1×3 matrix with an average
aggregate of two earlier-matrix keys, then move one part key and assert the
dependant follows the mean. Build it through the real seams
(`model.rs` shape, `evaluation.rs::order` multi-target visit,
`kernel.rs::compose_matrix_frame`), no UI.

## Subsequent acceptance

- Matrices evaluate in dependency order over the union of part targets,
  independent of document list order; cycles through any part refuse naming
  the matrix. Tests: reordered lists evaluate identically; 2-cycle and
  self-part refuse atomically.
- Intersection resolves two part poses to the heading-axis intersection with
  rotation 0 (upstream Y-axis-line construction); parallel lines refuse at
  evaluation without mutating the document. Pure line-intersection helper,
  unit-tested. Tolerance precedent: spec §10.
- `follow` filters the completed aggregate candidate as today; unfollowed
  components come from world zero. Cover the partial-x-only upstream
  observation (`(14.330127,0,0)`) as a BoardStudio golden.
- Locked decisions honored: naive `r` mean with no tolerance refusal,
  single-key parts only, no nested aggregates, unknown methods refused.

## Checks

Core integration tests via public `Open`/`Edit` routes (spec §11 seam rule);
affected quick/browser checks; full-gate evidence for the batch. Normal tests
do not invoke Ergogen/Node.
