# 17a: Grouped model and Core callers

Status: claimed
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
