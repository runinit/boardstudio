# 03: Design the parity fixture pipeline

Status: resolved
Type: research
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0036-matrices-follow-ergogen-point-layout.md)

## Question

Parity is proven by golden fixtures generated from pinned upstream ergogen and asserted by Rust
tests. Normal test runs need no Node and no ergogen (ADR-0034; the project rule that tests run
with standard tools).

Find and recommend:

- how the generator pins ergogen: a commit, the minimal dependencies (`makerjs`, `mathjs`; not
  `hull` or `kle-serial`), a lockfile, `--ignore-scripts`, and where the checkout lives
  (outside the repo or cached);
- where the generator script lives (`scripts/`), how it is run, and how it records the ergogen
  commit in the fixture;
- the fixture format: an ergogen config, the equivalent BoardStudio matrix parameters, and the
  expected poses per key, including mirrored keys;
- the tolerance, and how the Rust test maps ergogen names to `(matrix, row, column)`;
- licence obligations for running ergogen (MIT) and committing its outputs;
- whether the existing `core/tests/fixtures/matrix-projections.json` oracle is deleted or kept as
  a BoardStudio-only regression for extended features.

## Answer

- Recommend an opt-in `scripts/generate-ergogen-parity-fixtures.cjs` that calls pinned Ergogen `points.parse` directly and writes committed JSON goldens; keep its source checkout in the user cache.
- Pin Ergogen to full commit `8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37` (4.2.1), with exact Maker.js/Math.js versions and a checked-in npm lockfile; skip `hull` and `kle-serial`.
- Record upstream config, equivalent BoardStudio matrices, source commit/version, named expected poses, and `(matrix,row,column)`; preserve Ergogen names for traceability, including `mirror_` keys.
- In Rust, compare projected poses by coordinate identity with the existing `1e-9` absolute tolerance; keep `matrix-projections.json` for BoardStudio-specific behavior.
- Preserve upstream MIT and dependency Apache-2.0 notices with the tooling if software/dependencies are redistributed; keep provenance in the fixture.
- Full recommendation, fixture example, source citations, and the limited harness evidence: [Ergogen parity fixture pipeline](../../../research/2026-10-22-ergogen-parity-fixture-pipeline.md).

## Comments

- 2026-10-22 (map owner): Keeping `matrix-projections.json` unchanged conflicts with the wipe
  decision (ADR-0034, which replaces the old oracle). The new kernel changes custom-origin
  geometry, so its expected poses cannot stay. Its BoardStudio-only cases (Y mirror, custom
  origins, saved poses) are re-expressed as regenerated extended-feature regressions; the
  TypeScript-captured values are not kept.
- 2026-10-22 ([kernel ticket](04-layout-kernel-order-of-operations.md#answer)): `1e-9` is
  too tight for ergogen goldens. Maker.js rounds every rotated point to `1e-7`, and the largest
  error measured was `1.13e-7`. Ergogen goldens compare within `1e-6`; BoardStudio-only
  regressions keep `1e-9`.
