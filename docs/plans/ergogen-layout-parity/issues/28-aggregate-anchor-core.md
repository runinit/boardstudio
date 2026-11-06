# 28: Aggregate anchor Core (shape, evaluation, Follow)

Status: resolved
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

## Lane-1 spec-review fixes (on top of `750ff680a`)

- MUST-FIX 1 (specified target union): the union is enforced as the document
  grammar in `core/src/model.rs` — a flat `{matrixId,row,column,follow}` arm
  and an aggregate `{follow,aggregate}` arm with deny-guarded arms matched
  explicitly (an untagged enum would swallow each arm's inner refusal, e.g.
  the unknown-method error). Both-arms documents refuse at admission;
  saved single-key anchors reopen byte-identical. No `targets()` consumer
  changes were needed (struct shape preserved, so `layout.rs`,
  `geometry.rs`, `mirrored_half` keep compiling untouched).
- MUST-FIX 2 (A5 lifecycle): `detach_out_of_range`, `remap_compacted`,
  `shift_compacted` and the per-part counterpart mapping in linked `add`
  are reverted to the pre-lane single-key rules; linked `add` with an
  aggregate anchor refuses explicitly until the later reflection ticket.
  No A5 tests existed. Lifecycle remap per research §1.4.5 stays with A5.
- MUST-FIX 3 (golden + tolerance): the Follow test pins literal
  `(14.330127,0,0)` (x at the upstream 1e-6 precedent; y/r exact zeros) and
  the intersect guard uses the BoardStudio-only 1e-9 precedent, pinned by a
  kernel unit test on both sides of the threshold.
- Reorder test drives both `doc.matrices` and layout member order reversed
  end to end and asserts dependency order in both scenes.

## Answer

Resolved on branch `docs/ergogen-layout-comparison` jointly with tickets
29–32. Slices A1/A3/A4/A7 landed as RED `b8ec8c9c7` then GREEN `4665c5fb0`
(shape, average/intersection evaluation, Follow over aggregates; docs
`f2e75b10e`, style `750ff680a`), followed by the lane-1 spec-review fixes
`e7b7d062d` (single|aggregate union enforced as the admission grammar),
`c15ec34bc` (A5 lifecycle hunks reverted out of the core lane) and
`6bcee0ea8` (upstream partial-x golden pinned, local 1e-9 intersect guard),
recorded in `436b5dbc6`. The spec re-review and the standards re-review both
PASSED with no open findings. Joint native baseline `6c0e4f30` (+53/−0).
Joint gates green: quick
`check: all steps passed: repo, tooling, lint, typecheck, test, browser`
(468.3s) and full
`check: all steps passed: repo, tooling, lint, build, test, browser`
(419.5s).
