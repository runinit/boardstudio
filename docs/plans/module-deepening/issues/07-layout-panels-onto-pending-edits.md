# 07: Layout panels settle through `PendingEdits`

Status: resolved
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

- [x] No `*Submission` holder or `{Pending, Saved, Failed}` feedback enum remains in
  these files (`TransformSubmission`, `KeycapSizeSubmission`, `PairSubmission`,
  `AlignmentSubmission`, `BoardSubmission`, `MatrixSetupSubmission`,
  `PlacementSubmission`, `ExistingHalfSubmission`, `OutlineSubmission`,
  `MatrixTransformState`, `KeySizeState` and the like).
- [x] `part_placement.rs`'s `owner_is_live` is gone.
- [x] The `same_lineage` double match in `existing_half` and `mirrored_pair_controller`
  is gone, with its unreachable `Settlement::Pending` arm.
- [x] Mounted Layout tests pass.

## Verification

```sh
cargo test -p boardstudio-web-layout -p boardstudio-web-ui-shared --locked
python3 scripts/check.py lint typecheck test browser
```

## Outcome

Accepted on the integrated source `eebc703d3f1530c654f82fe92fa282c9617fe378`.
Merge `4ce44add6` preserves the external `9bd5ab125` ancestry; `b0a59b3e4`
incorporates its 27-file working diff. The external checkout remains unchanged;
the saved binary patch has SHA-256
`ea16f412c92aecf185cc0b36852726f4da0450f6bfd12109884a73d3196758cf`.

All listed settlement holders, feedback enums, manual lineage matches and the
part-placement owner helper are removed. Controllers use shared keyed observation
and real field bindings. Typed Vec2 key-size and legacy inspector fields retain
newer ordinary drafts, refresh clean fields on accepted changes and Undo/Redo,
and unbind disposed controls. Matrix transform actions bind disabling and reject
retained duplicate admission; field edits remain queueable. Exact Landed selection
and navigation stay with the caller. Retirement is silent and Saved feedback is gone.

Root completed the remaining typed bindings, draft/owner lifecycle corrections,
real pointer-event fixture, and Matrix pending-message cleanup. The recovery test
now waits for the queued Y ticket's actual BlockedByRecovery terminal before manual
refresh; a persistence failure for X cannot admit a Y landing under Runtime's
recovery contract. Separate non-recovery failure coverage retains cross-field
inline failure alongside a successful landing.

Final verification: native Layout 51 passed / 1 ignored, UI-shared 8 passed;
mounted Layout 131 plus setup-guide 1 passed; page groups 51 passed with complete
terminal coverage. Lint, WASM typecheck and full native tests pass, including the
repaired CAD fixture. Final Standards review reports zero documented breaches
(two nonblocking naming/tuple heuristics); Layout Spec review reports no blockers
at `8f013928`, whose Layout/page source is identical to final `eebc703d`.
See the [integration acceptance record](../integration-acceptance-2026-10-08.md)
for shared verification, graph coverage and limits.
