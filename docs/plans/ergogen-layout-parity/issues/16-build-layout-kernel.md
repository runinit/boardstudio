# 16: Build the pure carried-layout kernel and fixture comparison

Status: ready-for-agent
Type: build
Blocked by: [15](15-generate-parity-fixtures.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Implement the single pure geometry module, proposed at `core/src/matrix/kernel.rs`.
Stage layout value types/evaluation without changing persisted v6 documents or making
the application claim the new behavior. Reuse the eventual grouped record types;
keep staging input/adaptation private, not a second public document model or engine.

## First RED / GREEN slice

Through the agreed kernel seam, fail one literal default-step pose test and pass it.
Then add Shift followed by Bend versus offset-after-Bend as the next vertical case.
Feed committed fixtures through that same interface rather than private math helpers.

## Subsequent acceptance

- Implement the spec's column prefix/Splay versus own Column rotation, row Padding,
  summed row/key Shift then Bend, independent offsets/Key rotation and final mirror.
- Implement candidate Origin/Rotation then Follow, including the transported basis;
  expose unambiguous raw-anchor context versus composed-frame types/contract.
- Return exact shift/offset frames, column base/origin/rotation/axes and typed numeric
  contribution facts. Defaults, finite-value checks and no Spread/Padding at axis 0
  obey the agreed shape. Logical disabled/deleted values still carry.
- Compare all relevant upstream/BoardStudio fixtures at the agreed tolerances, including
  nonzero pivots, curved columns, X/Y mirror, key turns and independent offsets.
- Check cumulative Shift in matrix axes against a worked example, not raw vector sums.
  Do not include independent rotations in cumulative Bend.

## Scope and checks

No live schema activation or replacement of production edits in this slice; staging
success is not prepared-route support. Existing seams to replace later are
`core/src/matrix.rs` and `core/src/matrix/splay.rs`. Use focused
`cargo test -p boardstudio_core <new-kernel-test-filter>` after adding the named test,
then repository quick checks for the changed files. Register new integration coverage
in the existing Core integration harness (`autotests = false`) when applicable.
