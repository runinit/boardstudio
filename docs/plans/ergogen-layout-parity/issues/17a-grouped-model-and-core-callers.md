# 17a: Grouped model and Core callers

Status: resolved
Type: build
Blocked by: none
Parent: [17](17-cut-over-format-and-model.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Persist the grouped layout records and cut every removed-field Core caller over to them. The five currently failing integration tests must pass: `core_engine::deleting_matrix_column_compacts_both_halves_and_preserves_local_assemblies`; `outline_linked_resolution::a_failed_split_resolution_keeps_its_last_valid_display_without_a_false_blocker`; `outline_linked_resolution::a_split_linked_refinement_derives_its_region_count_and_never_blocks_as_disconnected`; `outline_linked_resolution::merged_fixed_and_generated_outlines_keep_their_region_count_semantics`; `outlines::manual_connections_respect_linked_split_boundaries`.

## Acceptance

- Persist column `rotation` as well as Spread/Stagger/Splay/origin/offset, row records, cell Shift/Bend and optional single-key anchor; replace Layout with `matrixIds`.
- Extend the same-board follower link's explicit correspondence/exception owner as specified; do not fake a physical pair or infer matrix pairs from list order.
- Remove fields to inventory compiler errors, then update every caller together: Core model/validation/materialization, matrix layout/splay/deletion, mirror inverse/reconcile, geometry/outline ownership, boundary projection, web tree/selection, setup/Inspector/transform consumers and their constructors/fixtures. No old formula survives as a fallback.
- The five failing integration tests listed in the Outcome are fixed.

## Checks

Source anchors: `core/src/model.rs`, `matrix.rs`, `matrix/layout.rs`, `mirrored_half/`, `geometry.rs`, `outline_versions.rs`, `web/crates/ui-model/src/tree.rs`. Use compiler-driven inventory, the focused public tests above and `check.py quick --files` for changed callers.

## Progress

### Verification — 2026-10-23

- [x] Baseline `cargo test -p boardstudio_core --test integration` after `98cad6d0e`:
  **735 passed, 0 failed** (`.evidence/ergogen-17a/00-baseline.log`). All five named
  tests pass; no production change was needed beyond that commit.
- [x] `cargo check --workspace --all-targets` clean (`01-check.log`).
- [x] Model carries column `rotation`, Spread/Stagger/Splay/origin/offset, row records,
  cell Shift/Bend, optional anchor and `Layout.matrix_ids`; no axis vectors or
  `Layout.matrix_id` remain. Matrix pairs come only from `LayoutMirrorLink.correspondences`
  (`populate_new_link` refuses multi-matrix groups without explicit correspondences).
- [ ] Ticket stays claimed; 17b-17e own validation bounds, kernel routing, entry gates and fixtures.

### TypeScript projection oracle regeneration — 2026-10-23

The oracle test keeps the legacy fixture shape through the test-side `grouped_from_legacy` adapter. Only the 90 mismatching expectations changed; every other value is byte-identical.

Cross-check: an independent Python implementation of ticket 04 formula 1 + mirror reproduces every x/y value in r0c1/r0c2 of none/x/y-custom-origins; parity S14-mirrored-heading (upstream) has mirror_m_b_r1 rotation -18 = -15 - (-10+6+7), the same whole-heading negation as r1c1 here (37-32=5, was 31).

| case | cell | field | old | new | spec clause |
|---|---|---|---|---|---|
| none-parametric | r0c0 | memberId | "matrix/fixture/r0c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-parametric | r0c1 | memberId | "matrix/fixture/r0c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-parametric | r0c2 | memberId | "matrix/fixture/r0c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-parametric | r1c0 | memberId | "matrix/fixture/r1c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-parametric | r1c2 | memberId | "matrix/fixture/r1c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-parametric | r2c0 | memberId | "matrix/fixture/r2c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-parametric | r2c1 | memberId | "matrix/fixture/r2c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-parametric | r2c2 | memberId | "matrix/fixture/r2c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-parametric | r0c0 | memberId | "matrix/fixture/r0c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-parametric | r0c1 | memberId | "matrix/fixture/r0c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-parametric | r0c2 | memberId | "matrix/fixture/r0c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-parametric | r1c0 | memberId | "matrix/fixture/r1c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-parametric | r1c1 | rotation | 31 | 5 | mirror reverses whole local heading (ticket 04 formula 3, decision 5; parity S14-mirrored-heading) |
| x-parametric | r1c2 | memberId | "matrix/fixture/r1c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-parametric | r2c0 | memberId | "matrix/fixture/r2c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-parametric | r2c1 | memberId | "matrix/fixture/r2c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-parametric | r2c2 | memberId | "matrix/fixture/r2c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-saved-poses | r1c1 | rotation | 31 | 5 | mirror reverses whole local heading (ticket 04 formula 3, decision 5; parity S14-mirrored-heading) |
| y-parametric | r0c0 | memberId | "matrix/fixture/r0c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-parametric | r0c1 | memberId | "matrix/fixture/r0c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-parametric | r0c2 | memberId | "matrix/fixture/r0c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-parametric | r1c0 | memberId | "matrix/fixture/r1c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-parametric | r1c1 | rotation | 31 | 5 | mirror reverses whole local heading (ticket 04 formula 3, decision 5; parity S14-mirrored-heading) |
| y-parametric | r1c2 | memberId | "matrix/fixture/r1c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-parametric | r2c0 | memberId | "matrix/fixture/r2c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-parametric | r2c1 | memberId | "matrix/fixture/r2c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-parametric | r2c2 | memberId | "matrix/fixture/r2c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-saved-poses | r1c1 | rotation | 31 | 5 | mirror reverses whole local heading (ticket 04 formula 3, decision 5; parity S14-mirrored-heading) |
| none-custom-origins | r0c0 | memberId | "matrix/fixture/r0c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-custom-origins | r0c1 | memberId | "matrix/fixture/r0c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-custom-origins | r0c1 | x | 15.176278150236557 | 18.7181164649605 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r0c1 | y | 9.258620487244208 | 5.776511482766027 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r0c2 | memberId | "matrix/fixture/r0c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-custom-origins | r0c2 | x | 24.021546727465214 | 23.213966838285636 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r0c2 | y | 25.780895693804837 | 25.34283567250104 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r1c0 | memberId | "matrix/fixture/r1c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-custom-origins | r1c1 | x | 0.8322983130187076 | 4.3741366277426525 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r1c1 | y | 23.758628846879308 | 20.27651984240113 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r1c2 | memberId | "matrix/fixture/r1c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-custom-origins | r1c2 | x | 12.726345907067234 | 11.918766017887652 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r1c2 | y | 38.64240565286021 | 38.20434563155642 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r2c0 | memberId | "matrix/fixture/r2c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-custom-origins | r2c1 | memberId | "matrix/fixture/r2c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-custom-origins | r2c1 | x | -14.669074461744938 | -11.127236147021005 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r2c1 | y | 29.38956501219109 | 25.907456007712913 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r2c2 | memberId | "matrix/fixture/r2c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| none-custom-origins | r2c2 | x | -2.7316669897209778 | -3.5392468789005562 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| none-custom-origins | r2c2 | y | 49.86959752272372 | 49.43153750141993 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r0c0 | memberId | "matrix/fixture/r0c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-custom-origins | r0c1 | memberId | "matrix/fixture/r0c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-custom-origins | r0c1 | x | -7.037430009188098 | -4.666474949903154 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r0c1 | y | -7.4806092640917266 | -11.845042087903506 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r0c2 | memberId | "matrix/fixture/r0c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-custom-origins | r0c2 | x | -25.3577467371247 | -24.71405723285899 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r0c2 | y | -11.429070887651328 | -10.773520979734206 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r1c0 | memberId | "matrix/fixture/r1c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-custom-origins | r1c1 | x | -17.02199596186766 | -14.651040902582714 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r1c1 | y | 10.304453084311557 | 5.940020260499772 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r1c1 | rotation | 31 | 5 | mirror reverses whole local heading (ticket 04 formula 3, decision 5; parity S14-mirrored-heading) |
| x-custom-origins | r1c2 | memberId | "matrix/fixture/r1c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-custom-origins | r1c2 | x | -34.60764432513735 | -33.96395482087165 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r1c2 | y | 2.973685605856353 | 3.6292355137734766 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r2c0 | memberId | "matrix/fixture/r2c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-custom-origins | r2c1 | memberId | "matrix/fixture/r2c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-custom-origins | r2c1 | x | -18.1620418066862 | -15.791086747401263 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r2c1 | y | 26.757425322533795 | 22.39299249872202 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r2c2 | memberId | "matrix/fixture/r2c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| x-custom-origins | r2c2 | x | -41.139108021531484 | -40.49541851726578 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| x-custom-origins | r2c2 | y | 20.92751477911868 | 21.583064687035808 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r0c0 | memberId | "matrix/fixture/r0c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-custom-origins | r0c1 | memberId | "matrix/fixture/r0c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-custom-origins | r0c1 | x | 21.037430009188096 | 18.666474949903154 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r0c1 | y | 1.4806092640917266 | 5.845042087903506 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r0c2 | memberId | "matrix/fixture/r0c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-custom-origins | r0c2 | x | 39.3577467371247 | 38.71405723285899 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r0c2 | y | 5.429070887651328 | 4.773520979734206 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r1c0 | memberId | "matrix/fixture/r1c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-custom-origins | r1c1 | x | 31.02199596186766 | 28.651040902582714 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r1c1 | y | -16.304453084311557 | -11.940020260499772 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r1c1 | rotation | 31 | 5 | mirror reverses whole local heading (ticket 04 formula 3, decision 5; parity S14-mirrored-heading) |
| y-custom-origins | r1c2 | memberId | "matrix/fixture/r1c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-custom-origins | r1c2 | x | 48.60764432513735 | 47.96395482087165 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r1c2 | y | -8.973685605856353 | -9.629235513773477 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r2c0 | memberId | "matrix/fixture/r2c0" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-custom-origins | r2c1 | memberId | "matrix/fixture/r2c1" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-custom-origins | r2c1 | x | 32.162041806686204 | 29.791086747401263 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r2c1 | y | -32.7574253225338 | -28.39299249872202 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r2c2 | memberId | "matrix/fixture/r2c2" | null | member id only for existing parts (8b367d8c7; ticket 17) |
| y-custom-origins | r2c2 | x | 55.139108021531484 | 54.49541851726578 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |
| y-custom-origins | r2c2 | y | -26.92751477911868 | -27.583064687035808 | splay origin relative to column base, carried through earlier splays (ticket 04 formula 1, decision 11) |

### Fix round — 2026-10-23

- [x] Step 1: `check.py quick` real state — fails in the `browser` step (`outline::browser_tests::connected_mirrored_outline_gap_controls_edit_distinct_regions`, `host::storage::lifetime_tests::mixed_format_records_are_listed_without_conversion_and_raw_json_is_preserved`); see `.evidence/ergogen-17a/ds-quick1.log`.
- [x] Step 2: RED/GREEN — `unrelated_matrix_edit_never_resurrects_a_tombstoned_member_correspondence` failed on the re-created pair; pairing now walks `cell_members` (deleted slots excluded) and skips slots with a Part exception. `unrelated_matrix_edit_keeps_a_local_exception_unassociated` covers the exception case and fails without that guard.
- [x] Step 3: resize test now checks every row, shows retired r0c2 stays gone after shrinking to 2, and the column-compaction test asserts the exact surviving `(source, mirror)` pair set.
- [x] Step 4: one `layout::member_pairs` helper now backs `populate_new_link`, `sync_member_pairs` and the `CreateMirroredPair` correspondence build; `sync_member_pairs` takes the link in place instead of cloning the Layout; `update_link_members` renamed.
- [x] Step 5: `cargo test -p boardstudio_core --lib --test integration` — 802 passed / 4 ignored and 738 passed, 0 failed.
- [x] Step 6: `check.py test --update-native-baseline` — `all steps passed: test`, 2127 executed; corrected count: `8e2454fe4` added the 2 new Core tests; the baseline change in `aec9237fb` was 29 added and 8 removed (renames) — see the notes below.
- [x] Step 8: browser twin of the synthetic mirrored fixture no longer forces `formatVersion` 6, matching the native fix in `1c26111a5`; `connected_mirrored_outline_gap_controls_edit_distinct_regions` passes.
- [x] Step 9: the mixed-format storage test wrote to the project store with `put_with_key`, but that store uses in-line keys (`keyPath: "id"`); `put` now matches the production save path and the record's own `id`.
- [x] Step 10: the convert-to-stagger browser test pinned the old splay-only refusal text; Core's grouped reason (`core/tests/matrix_placement.rs` already pins it) covers Splay, Column rotation and carried Bend, so only the stale literal changed.
- [x] Step 11: the linked part-placement fixture's mirror link now declares its one matrix correspondence; Core refuses a linked Layout without one.
- [x] Step 7: final `check.py quick` on the five changed files — **all steps passed: repo, lint, typecheck, test, layout-test-css, browser** (`.evidence/ergogen-17a/ds-browser-quick.log`, receipt `ds-browser-receipt.json`). The two named browser failures plus two grouped-fixture failures found behind them are fixed in steps 8–11.

### Consolidated fix round — notes

- `aec9237fb` ("record native baseline after oracle regeneration") also holds unrelated baseline catch-up: 10 test renames tracing to `570bb2672` and `be9714b85`, plus `matrix_kernel` and `document_integrity` additions. Nothing was weakened.
- `3172d6513`'s message names only the mirror heading and member ids; it also changed 18 custom-origin x/y expectations, documented in the oracle regeneration table above.
- Step 6 count corrected above (`8e2454fe4`: 2 tests; `aec9237fb`: 29 added, 8 removed).

## Answer

Commit range: `b159d3d5f..HEAD` (the closing commit of this ticket, after `ec6bb7d16`).

### Delivered

- Grouped records and `Layout.matrixIds`, with explicit correspondences owned by the mirror link.
- The five previously failing tests are fixed.
- Web callers are migrated to the grouped model.
- The oracle is ported to grouped records; mirror-heading and member-id expectations are regenerated and spec-clause'd.
- Tombstones and Local exceptions are respected when pairing.
- Split classification uses the owning matrix.
- Browser fixtures are corrected.

### Evidence

- Standards and Spec reviews of the full range, and of the delta `b475b063a..064deb416`, both passed with no blockers.
- Final quick check last line: `all steps passed: repo, tooling, lint, typecheck, test, layout-test-css, browser`.
- Closure commit `ec6bb7d16` adds `a_part_outside_any_matrix_is_classified_by_its_own_pose`; it fails when `split_x` returns the lowest-id matrix root for loose parts.

### Notes from the delta reviews

1. `populate_new_link` filtering excepted pairs (`39f19ec43`) is a new bug fix, not a refactor.
2. `sync_member_pairs` no longer skips a pair on `exception.mirror_id == source_id`; that match was unreachable because exceptions are keyed on the mirror member.
3. A part directly in a layout's `part_ids` outside any matrix is classified by its own `pose.at.x` in `split_x` (`core/src/geometry.rs`); the old lowest-id-root rule had no spec basis. Pinned by the test above.
4. `CreateMirroredPair` (`core/src/lib.rs`) does not filter by exceptions; that is fine because the link is built fresh.

### Carried forward

- Bullet 2's exception half: exceptions are only pruned and respected, never created or reconciled on resize. Belongs to ticket 21 (exception lifecycle).
- The TypeScript oracle is still active as the ported test; its retirement belongs to 17c.
- `assert_close` in `core/src/mirrored_half/inverse/tests.rs` compares JSON key order and fails under `cargo test --workspace` without `--exclude boardstudio-footprints` (`preserve_order` feature unification).
- Quick checks need `TMPDIR=/home/chris/.cache/tmp` in this sandbox.
