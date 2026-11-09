# 31: Aggregate reflection and fixtures

Status: resolved
Type: build
Blocked by: [28](28-aggregate-anchor-core.md) (aggregate shape and evaluation)
Parent: [deferred plan](../deferred-items-plan.md) · Research: [deferred-items-research.md](../deferred-items-research.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Plan slices A6 (linked reflection) and A8 (fixtures) only. Does
not change evaluation, Follow, validation, lifecycle, or UI.

## Outcome

Aggregate targets map through the pair correspondence by identity, with
relative-negate rules applying to the resolved aggregate pose rather than per
part (mirror of the whole candidate, ticket-20 transported-filter precedent).
Upstream-pinned average and intersection goldens are generated through
`scripts/generate-ergogen-parity-fixtures.cjs` (spec §10).

## First RED / GREEN slice

Fail a public-route test: linked Add with an average aggregate, asserting the
counterpart resolves the mirrored whole-candidate pose. Seam:
`core/src/matrix/layout.rs::sync` with kernel `FollowBasis`.

## Subsequent acceptance

- Non-origin-axis Follow transport over aggregates; actual-Local part keys in
  linked pairs. Tests mirror ticket 20's (linked Add, transport, Local).
- Average + intersection goldens pinned upstream through the fixture
  generation script; tolerance precedent spec §10 (`1e-6` upstream /
  `1e-9` local). No bulk-regenerated unrelated snapshots.

## Checks

Core integration tests via public routes; affected quick/browser checks;
fixture script run documented. Normal tests do not invoke Ergogen/Node.

## Progress

Implementation on branch `docs/ergogen-layout-comparison`, ticket kept claimed
(no resolve, no push). Production: `anchors.rs::add` maps every aggregate part
through the pair correspondence by identity in linked groups (method and
Follow cross unchanged), lifting the ticket-28 refusal; `layout.rs::sync`
remaps the whole anchor via a new `remap_anchor` helper (single key or every
part, missing counterpart refuses naming the layout) so the mirror resolves
the whole candidate; the relative negate stays in `reflected`/kernel
`FollowBasis` transport, never per part. Counterpart ownership check reads all
targets, covering both arms uniformly.

Proving tests (all via public `Open`/`Edit` routes, new file
`core/tests/matrix_aggregate_reflection.rs`): linked Add with an average
aggregate mirrors the whole candidate (`claw` (19,-19) / `claw-mirror`
(1,-19) about x=10, parts remapped by identity, one-Undo removes both);
non-origin-axis Follow transport over aggregates (Follow X off keeps source
X=0 while the partner keeps the transported X=20); mirror dependent follows
the actual Local part pose through the mean ((5,2.5) + (0,-19) = (5,-16.5),
layouts stay linked).

Fixtures: `aggregate-average` and `aggregate-intersect` cases in
`scripts/ergogen-parity/cases.cjs` with upstream aggregate anchors; the pinned
`8286e18f` oracle returns probe poses (19,0,0) and (0,38,0) respectively, and
the regenerated `core/tests/fixtures/ergogen-layout-parity.json` diff is
additions-only (40→42 cases). `matrix_kernel.rs` resolves aggregate anchors in
the fixture helper and now counts 401 consumed poses; its placeholder part
poses are distinct per part so intersect never sees a spurious parallel pair
before the refresh corrects them. Known limit: Open of a hand-authored doc
with stale part poses plus an intersect anchor can refuse on the stale poses;
production docs always carry refreshed poses.

## Answer

Resolved on branch `docs/ergogen-layout-comparison` jointly with tickets 28–
30 and 32. Slice A6 landed as RED `7e383e7d7` then GREEN in the shared
lane-2/3 commit `b7dd85794`, whose linked-add counterpart-mapping hunk
(`anchors.rs::add` maps every aggregate part through the pair correspondence
by identity) and `layout.rs` whole-candidate remap (`remap_anchor`,
`reflected`/`sync` transport) belong to this ticket (the same commit's
validation/remap/shift hunks belong to
[30](30-aggregate-validation-lifecycle.md#answer)). Slice A8 landed as the
upstream-pinned average/intersection parity goldens `f1e38c56d` (style
`2778899e5`; progress recorded in `313975ab3`). The dual review (spec +
standards) PASSED with no open findings. Joint native baseline `6c0e4f30`
(+53/−0). Joint gates green: quick
`check: all steps passed: repo, tooling, lint, typecheck, test, browser`
(468.3s) and full
`check: all steps passed: repo, tooling, lint, build, test, browser`
(419.5s).
