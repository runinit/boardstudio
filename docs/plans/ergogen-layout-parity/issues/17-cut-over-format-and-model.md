# 17: Cut over the persisted layout model and every removed-field caller

Status: ready-for-agent
Type: build
Blocked by: [16](16-build-layout-kernel.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Perform the coordinated model/caller cutover in the unshipped integration lane.
Replace parallel layout vectors and singular Layout ownership, route production math
to the kernel, and introduce the controlled version-7 gate. Do not promote a user-data
writer before ticket 27. Tests use isolated stores; no compatibility aliases/migration.

## First RED / GREEN slice

Fail a public Core Open/serialize/projection case for one valid grouped-layout v7
matrix plus refusal of the old input. Introduce the smallest complete schema/reader
path to pass, then add the next frame/default/ownership case. A constant-only bump
must not reach the old migration loop's `unreachable!`.

## Coordinated acceptance

- Persist column `rotation` as well as Spread/Stagger/Splay/origin/offset, row records,
  cell Shift/Bend and optional single-key anchor; replace Layout with `matrixIds`.
  Extend the same-board follower link's explicit correspondence/exception owner as
  specified; do not fake a physical pair or infer matrix pairs from list order.
- Remove fields to inventory compiler errors, then update every caller together:
  Core model/validation/materialization, matrix layout/splay/deletion, mirror inverse/
  reconcile, geometry/outline ownership, boundary projection, web tree/selection,
  setup/Inspector/transform consumers and their constructors/fixtures. No old formula
  survives as a fallback. No silent acceptance of an unresolved anchor as world origin.
- Apply finite/default/list-bound checks, sparse defaults, negative/zero step policy,
  growth/truncation and nonempty logical Layout ownership. Preserve unrelated data.
- Gate browser list/load, archive unpack and direct Core Open before migration or
  replacement effects. Keep `boardstudio/v2`, DB version 1 and archive metadata.
  Missing/older/newer/malformed versions fail through the specified controlled paths.
- Route draft/actual projection and materialized parts through the staged kernel.
  Retire the old TypeScript oracle only with replacement coverage active. Regenerate
  the minimum affected baseline schema/scene fixtures deliberately; do not weaken tests.
- Keep current storage transaction, identity, companion, scan/keymap and history
  contracts intact. Remaining feature tickets must complete before this lane ships.

## Checks

Source anchors: `core/src/model.rs`, `migrate.rs`, `matrix.rs`, `matrix/layout.rs`,
`validate.rs`, `geometry.rs`, `outline_versions.rs`, `mirrored_half/`,
`web/crates/host/src/host/storage.rs`, `web/crates/ui-model/src/tree.rs`.
Use compiler-driven inventory, focused public tests and `check.py quick --files` for
all changed callers. Require native/contract and WASM compilation, not just serde tests.
