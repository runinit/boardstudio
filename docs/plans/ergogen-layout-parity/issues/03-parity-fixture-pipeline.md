# 03: Design the parity fixture pipeline

Status: ready-for-agent
Type: research
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

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
