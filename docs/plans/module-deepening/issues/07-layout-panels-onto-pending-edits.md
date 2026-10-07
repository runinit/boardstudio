# 07: Layout panels settle through `PendingEdits`

Status: ready-for-agent
Type: build
Blocked by: 05, 06, 12 (shares `web/src/presentation.rs`)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

Move every remaining Layout settlement site onto `PendingEdits` and delete the
per-panel layer. Files (at `915d305c0`):

- `web/crates/layout/src/objects/`: `board_setup_controller`, `existing_half`,
  `keycap_size_controller`, `layout_align_controller`, `layout_transform_toolbar`,
  `matrix_placement_controller`, `matrix_setup_controller`,
  `matrix_transform_controller`, `mirrored_pair_controller`;
- `web/crates/layout/src/`: `board_inspector.rs`, `part_placement.rs`,
  `matrix_transform_lifecycle.rs`, `mirrored_pair_geometry.rs`, the outline hook from
  [ticket 06](06-split-outline-lifecycle.md);
- `web/crates/ui-shared/src/geometry_scripts.rs`;
- `web/src/presentation/`: `layout_component_edits.rs` (`settle_field`,
  `settle_inspector_edits`), `inspector.rs`; settlement reads in `web/src/presentation.rs`.

## Migration rules (same for 07, 08 and 09)

- Every settlement site uses `PendingEdits` and the `ui-shared` helpers from
  [ticket 05](05-pending-edits-module.md); the panel keeps only its projection, its
  resolvers and its own owner predicate.
- Retirement is silent and there is no "Saved" status
  ([ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)).
  Update mounted tests that assert a removed message or status.
- The panel chooses where a failure message appears; default inline at the field.
- Tests that call `EditResolver::resolve` by hand move to the native Runtime
  ([ticket 03](03-native-test-runtime.md)); native tests that only restated settlement
  are deleted.

## Acceptance criteria

- [ ] No `*Submission` holder or `{Pending, Saved, Failed}` feedback enum remains in
  these files (`TransformSubmission`, `KeycapSizeSubmission`, `PairSubmission`,
  `AlignmentSubmission`, `BoardSubmission`, `MatrixSetupSubmission`,
  `PlacementSubmission`, `ExistingHalfSubmission`, `OutlineSubmission`,
  `MatrixTransformState`, `KeySizeState` and the like).
- [ ] `part_placement.rs`'s `owner_is_live` is gone.
- [ ] The `same_lineage` double match in `existing_half` and `mirrored_pair_controller`
  is gone, with its unreachable `Settlement::Pending` arm.
- [ ] Mounted Layout tests pass.

## Verification

```sh
cargo test -p boardstudio-web-layout -p boardstudio-web-ui-shared --locked
python3 scripts/check.py lint typecheck test browser
```
