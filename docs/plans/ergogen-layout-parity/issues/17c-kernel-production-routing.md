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
- Retire the TypeScript oracle → `aeeab5769` removes `projection_tests::projections_match_captured_typescript_behavior` plus its `grouped_from_legacy`/`ExpectedCell` helpers. Replacement coverage: the 381-pose kernel/production tests above, the new state test, and the retained `actual_projection_uses_saved_poses_and_parametric_empty_cells`. The spec review found that lost the saved-pose reads and per-cell `memberId`s, so `matrix_kernel::saved_poses_and_member_ids_of_existing_parts_are_read_in_every_mirror_mode` (`280c212c2`) was added; see the table below.

### Test names for the integrator's baseline update

- Added: `matrix_kernel::production_enabled_deleted_and_omitted_keys_carry_into_later_keys`, `matrix_kernel::saved_poses_and_member_ids_of_existing_parts_are_read_in_every_mirror_mode`.
- Removed: `matrix::projection_tests::projections_match_captured_typescript_behavior`.
- Renamed: none. WASM: none.

### Verification

`cargo test -p boardstudio_core --lib --test integration`: 801 passed / 4 ignored and 742 passed (`.evidence/ergogen-17c/core.log`).

### Fix round: oracle coverage

`core/tests/fixtures/matrix-projections.json` (9 cases, regenerated in 17a) is read again, by the new Rust test via `include_str!`. `core/tests/support/boundary.rs` does not use it (it only asserts the reply kind `matrix-projections`); `scripts/test-ergogen-parity.cjs:97` and `scripts/generate-ergogen-parity-fixtures.cjs:202` reference it as the legacy fixture. The fixture is untouched and the test needs no expected value that differs from it.

The new test opens each case through `CoreEngine` and compares every cell's row, column, enabled flag, `memberId` and pose to the fixture (1e-9). Open rewrites each matrix-owned part from its parametric pose unless a linked layout marks the part Local, so the saved-pose cases mark every part Local in a mirror-linked layout (with dummy `src`/`mir` matrices) to make the saved pose reach the actual projection. Each part there has a distinct pose, so the match proves a read.

| Oracle case | Replacement |
|---|---|
| none-saved-poses, x-saved-poses, y-saved-poses | New test: exact `matrix/fixture/r{r}c{c}` ids and saved poses for the 8 existing parts; the disabled r1c1 stays parametric with `memberId` null. |
| none-parametric, x-parametric, y-parametric | New test (null `memberId`, parametric poses) plus the 381-pose production fixtures. |
| none-custom-origins, x-custom-origins, y-custom-origins | Same as the parametric cases, with custom column origins. |

Mutations (each reverted): (a) ignoring the saved pose when `matrix.mirror` is set fails with "x-saved-poses r0c0 pose x: expected 0, got 5.559514408902528"; (b) dropping the member id fails with "none-saved-poses r0c0 memberId: left Null, right String(\"matrix/fixture/r0c0\")".

Standards fixes in `matrix_kernel.rs` (`1ba8106c5`): `states_document` builds its id lists directly (dead `partIds` removed), the helper doc comment names `document`, and the carried-pose assertion pins the exact pose (x -17.05, y 0, rotation 90). `open_and_project` and `scene_cell` stay: no earlier helper does the same (the production fixture test inlines its own flow). The `match state` stays, since a per-case struct would not read better.

Fix verification: `cargo test -p boardstudio_core --lib --test integration -- --test-threads=2`: 801 passed / 4 ignored, then 743 passed (`.evidence/ergogen-17c/fix-core.log`). With default parallelism the run was killed with exit 143 after the lib result; the same tests pass serially and at 2 threads.
