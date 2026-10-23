# 17c: Kernel production routing

Status: claimed
Type: build
Blocked by: [17a](17a-grouped-model-and-core-callers.md)
Parent: [17](17-cut-over-format-and-model.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Make the staged kernel the production layout engine, with replacement coverage active before the old oracle goes.

## Acceptance

- Route draft/actual projection and materialized parts through the staged kernel.
- Retire the old TypeScript oracle only with replacement coverage active.
- Cover explicit enabled/deleted/omitted production states (ticket 16 Answer).

## Checks

Source anchors: `core/src/geometry.rs`, `matrix/layout.rs`. Use the committed parity poses through actual scenes, draft projections and materialized parts, plus `check.py quick --files`; require native/contract and WASM compilation.

## Progress

### Routing audit (base `d45f30271`)

Every layout pose now comes from `kernel::evaluate`: `matrix/evaluation.rs:195` (`resolve`, anchors) and `matrix.rs:307` (`valid_projection_matrix`) feed `project_matrix` (`matrix.rs:~318`), which `lib.rs:384` (Draft) and `lib.rs:1171` (Actual) call; materialized parts take `evaluation::cell_pose` at `matrix.rs:~640`, and `placement.rs`, `splay.rs`, `layout.rs:1389,1563` and `mirrored_half/inverse.rs:1192` read the kernel's `MatrixLayout`. Nothing was left off the kernel. The remaining `sin`/`cos` sites (`placement.rs:293`, `splay.rs:6`, `layout.rs:1522`, `matrix.rs:659`) are rigid transforms of companions about an already-placed key, not layout formulas. Web crates hold no matrix layout formula (`keycap_resize.rs` and `demo_projects.rs` positions are literal fixtures/starters). The only remaining work was the production-state tests and the oracle.

### Acceptance → proving test

- Route draft/actual projection and materialized parts through the kernel → `matrix_kernel::production_core_actual_draft_and_materialized_keys_match_committed_fixtures` (381 poses, existing since 17a).
- Enabled/deleted/omitted production states → `matrix_kernel::production_enabled_deleted_and_omitted_keys_carry_into_later_keys` (`7e6f022b7`). Honest RED note: routing already existed, so it passed on first run. Mutation proof: making `evaluation::input` drop Shift for disabled/deleted cells fails it ("disabled src r1": x -19.05 vs -17.05); the mutation is reverted (`.evidence/ergogen-17c/mutation1.log`).
- Retire the TypeScript oracle → `aeeab5769` removes `projection_tests::projections_match_captured_typescript_behavior` plus its `grouped_from_legacy`/`ExpectedCell` helpers. Replacement coverage: the 381-pose kernel/production tests above, the new state test, and the retained `actual_projection_uses_saved_poses_and_parametric_empty_cells`. `matrix-projections.json` stays: `core/tests/support/boundary.rs` uses it.

### Test names for the integrator's baseline update

- Added: `matrix_kernel::production_enabled_deleted_and_omitted_keys_carry_into_later_keys`.
- Removed: `matrix::projection_tests::projections_match_captured_typescript_behavior`.
- Renamed: none. WASM: none.

### Verification

`cargo test -p boardstudio_core --lib --test integration`: 801 passed / 4 ignored and 742 passed (`.evidence/ergogen-17c/core.log`).
