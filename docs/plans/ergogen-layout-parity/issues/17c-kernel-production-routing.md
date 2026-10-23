# 17c: Kernel production routing

Status: resolved
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

- Added: `matrix_kernel::production_enabled_deleted_and_omitted_keys_carry_into_later_keys`, `matrix_kernel::saved_poses_and_member_ids_of_existing_parts_are_read_in_every_mirror_mode`, `matrix::projection_tests::ordinary_parts_match_saved_pose_and_member_id_fixtures_in_every_mirror_mode`.
- Removed: `matrix::projection_tests::projections_match_captured_typescript_behavior`.
- Renamed: none. WASM: none.

### Verification

`cargo test -p boardstudio_core --lib --test integration`: 801 passed / 4 ignored and 742 passed (`.evidence/ergogen-17c/core.log`).

### Fix round: oracle coverage

`core/tests/fixtures/matrix-projections.json` (9 cases, regenerated in 17a) is read again, by the new Rust test via `include_str!`. `core/tests/support/boundary.rs` does not use it (it only asserts the reply kind `matrix-projections`); `scripts/test-ergogen-parity.cjs:97` and `scripts/generate-ergogen-parity-fixtures.cjs:202` reference it as the legacy fixture. The fixture is untouched and the test needs no expected value that differs from it.

The new test opens each case through `CoreEngine` and compares every cell's row, column, enabled flag, `memberId` and pose to the fixture (1e-9). Open refreshes matrix-owned parts from their parametric poses but preserves Local exceptions and parts with `layoutOverride == 1` (`layout.rs`, `refresh_existing`). The public test uses Local exceptions in a mirror-linked layout (with dummy `src`/`mir` matrices) as a sufficient way to preserve saved poses. Projection itself reads existing member poses regardless of Local status (`matrix.rs`, `project_matrix`). Each part there has a distinct pose, so the match proves a read.

| Oracle case | Replacement |
|---|---|
| none-saved-poses, x-saved-poses, y-saved-poses | Direct ordinary-part characterization and retained public Local-path test: exact `matrix/fixture/r{r}c{c}` ids and saved poses for 8 existing parts per mode; disabled r1c1 stays parametric with `memberId` null. |
| none-parametric, x-parametric, y-parametric | Direct characterization and retained public test (null `memberId`, parametric poses), plus the 381-pose production fixtures. |
| none-custom-origins, x-custom-origins, y-custom-origins | Same as the parametric cases, with custom column origins. |

Mutations (each reverted): (a) ignoring the saved pose when `matrix.mirror` is set fails with "x-saved-poses r0c0 pose x: expected 0, got 5.559514408902528"; (b) dropping the member id fails with "none-saved-poses r0c0 memberId: left Null, right String(\"matrix/fixture/r0c0\")".

Standards fixes in `matrix_kernel.rs` (`1ba8106c5`): `states_document` builds its id lists directly (dead `partIds` removed), the helper doc comment names `document`, and the carried-pose assertion pins the exact pose (x -17.05, y 0, rotation 90). `open_and_project` and `scene_cell` stay: no earlier helper does the same (the production fixture test inlines its own flow). The `match state` stays, since a per-case struct would not read better.

Fix verification: `cargo test -p boardstudio_core --lib --test integration -- --test-threads=2`: 801 passed / 4 ignored, then 743 passed (`.evidence/ergogen-17c/fix-core.log`). With default parallelism the run was killed with exit 143 after the lib result; the same tests pass serially and at 2 threads.

### Fix round 2: independent Spec re-review finding and response

The independent Spec re-review failed: the public replacement marked all saved parts Local, narrowing the retired direct oracle's coverage. The retained ordinary-part test covered X only, leaving none and Y saved-pose reads unpinned. This is a coverage gap, not a demonstrated production behavior regression.

Added `matrix::projection_tests::ordinary_parts_match_saved_pose_and_member_id_fixtures_in_every_mirror_mode` alongside the existing direct tests. It intentionally bypasses Open's refresh and passes ordinary fixture parts directly to `project_matrix`, with no Local exceptions or `layoutOverride` additions. The legacy-to-grouped adapter and expected-cell shape follow the retired `d45f30271` oracle and current integration adapter; expected data is unchanged. All 9 cases / 81 cells compare row, column, enabled, memberId and x/y/rotation (1e-9), with guards for all three saved-pose modes. The public CoreEngine/Local test, 381-pose production test, explicit production-state test and prior direct tests are preserved.

Characterization label: routing already exists and this test passes unmutated; no failing-first production claim. Focused mutations below establish sensitivity to regressions. Corrected the Local-only explanation in the public test and above; no override feature or test was added. Status remains claimed, pending independent review and integration.


Fix-round-2 verification (all heavy commands serialized with `/home/chris/.cache/tmp/boardstudio-build.lock`, `TMPDIR=/home/chris/.cache/tmp`):

- Compile first: `cargo test -p boardstudio_core --lib --test integration --no-run` passed (`.evidence/ergogen-17c/fix2-compile.log`).
- Focused characterization: 1 passed, 0 failed (`.evidence/ergogen-17c/fix2-focused.log`).
- Full `cargo test -p boardstudio_core --lib --test integration -- --test-threads=2`: `test result: ok. 802 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out`; `test result: ok. 743 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` (`.evidence/ergogen-17c/fix2-core.log`). Native runs and the mutation script/subprocesses used a 6 GiB MemoryMax, zero swap scope and 900-second outer timeout.
- `scripts/mutation-check.py` killed bypass-none at `none-saved-poses r0c0 pose x` (expected 0, got 7.152894557070724), bypass-Y at `y-saved-poses r0c0 pose x` (expected 0, got 8.440485591097472), and drop-memberId at `none-saved-poses r0c0 memberId`. Standalone logs: `.evidence/ergogen-17c/fix2-mutations/fix2-bypass-none-saved-poses.log`, `.evidence/ergogen-17c/fix2-mutations/fix2-bypass-y-saved-poses-qualified.log`, `.evidence/ergogen-17c/fix2-mutations/fix2-drop-member-id.log`. All three failed the new direct test, no timeouts, exact restoration confirmed. Initial Y mutation used an unqualified enum and failed compilation; it was not counted, then corrected and rerun through the same script (`.evidence/ergogen-17c/fix2-y-mutation.log`).
- Changed-file quick gate: `.evidence/ergogen-17c/fix2-quick.log`, receipt `.evidence/ergogen-17c/fix2-receipt.json`; 22 GiB MemoryMax, zero swap. Baselines remain owned by the integrator; this slice does not edit them.

- Independent Spec + Standards re-review: PASS. The ordinary non-Local direct oracle coverage gap is fixed; no other lost oracle coverage was found. The quick gate's first failure is the deliberately retired baseline name `matrix::projection_tests::projections_match_captured_typescript_behavior` (absent); CSS/browser stages were not reached. Integration baseline remains pending. No additional heavy checks or mutation run in this round. `git diff --check` and `python3 scripts/check-doc-links.py` passed (554 Markdown files).

## Answer

Resolved on the accepted feature commit `4fdafa15de3c8b163c9851c08506e942ca84fa40`, integrated at merge `35063671828e7ccf1fd9c6d01c7f1ce004316734`, with reviewed native baseline commit `57beeac98fee00d28b1d852257413cf6082a5332`. Draft/actual projection and materialized parts use the staged kernel; the retired TypeScript oracle is replaced by direct ordinary-part saved-pose/member-id coverage, production fixtures and enabled/deleted/omitted state coverage. The correction explicitly tests ordinary parts apart from the retained CoreEngine Local path: nine fixture cases / 81 cells are checked, including none, X and Y saved-pose modes. Independent Spec + Standards review passed. Three qualified mutation proofs killed bypass-none, bypass-Y and dropped-memberId; routing was already present, so no behavioral RED is claimed.

Fresh merged-tree quick evidence: `.evidence/ergogen-merge/17c-quick.log` and `.evidence/ergogen-merge/17c-receipt.json`. All scheduled steps passed: repo, tooling, lint, WASM typecheck, native tests, layout CSS build and browser. The native guard confirms all 2,149 baselined names executed; WASM page compilation and all selected browser launches succeeded. No full check was run. Tickets 17e, parent 17, later feature tickets and release gate 27 remain pending; this resolution grants no shipping approval.
