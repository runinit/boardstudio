# 176: Speed up CAD layered, PCB, and STEP validation tests and prune STEP oracle self-tests

Status: resolved
Type: build
Blocked by: None (can start immediately)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

The native CAD test suite runs without spending over a minute re-meshing curved solids at 0.1-micron precision, serializing unused evidence JSON on every assertion, or running dozens of string-replacement self-tests on test-only STEP oracle helpers.

## Acceptance criteria

- [x] Guard unconditional pretty-JSON serialization in `retain_evidence` (`cad/monstertruck/src/layered/tests.rs`) behind `LAYERED_013_EVIDENCE_OUTPUT` being set.
- [x] Relax redundant `1e-4` strict tessellation calls in `cad/monstertruck/src/layered/tests.rs` and `cad/monstertruck/src/layered/pcb_tests.rs` to `1e-3` where the volume assertion tolerance is already `0.002`–`0.02`, while keeping all closed-shell, topology, STEP surface-count, and material-probe assertions intact. Where `1e-3` cannot meet the existing volume tolerance the call stays at `1e-4` (see Outcome).
- [x] Deduplicate `rotated_notched_wall_is_one_authored_plane` so it verifies `input == retained` once and runs one B-rep build and STEP export instead of four, remove the duplicate `comparable_build_workload` stub, and remove the duplicate 2D permutation test `adoption_ring_island_retains_disjoint_hole_segment_sources` (covered in 3D by `adoption_ring_island_retains_disjoint_through_holes_in_either_opening_order`).
- [x] Trim the test-only `step_accuracy` string-mutation self-tests at the bottom of `cad/monstertruck/src/export_tests.rs` (`countersink` and `periodic_source_circle` certificate mutation sweeps) to one positive and one negative check per helper, and remove the three redundant single-line wrapper imports in `cad/monstertruck/tests/step_import.rs` (`source_surface_curves_keep_native_3d_bounds_and_associated_pcurves`, `finite_source_lines_share_the_asset_without_becoming_triangle_faces`, `import_invariants_pass_for_valid_step`).
- [x] Merge `thq_source_assemblies_keep_tight_bounds_and_meshes` into `thq_pins_align_with_leads_and_front_back_transform_oracle` in `cad/wasm/src/model/source_validation_tests.rs` and parallelize the three THQ vendor STEP assemblies across `std::thread::scope`.
- [x] `python3 cad/scripts/test-cad.py` passes with its wall-clock time recorded before and after.

## Outcome

`python3 cad/scripts/test-cad.py` exits 0 before and after.

| Run | Wall clock | Monstertruck lib | `step_import` | WASM lib |
| --- | --- | --- | --- | --- |
| before (HEAD sources for the five files) | 115 s | 286 passed / 1 ignored, 62.53 s | 60 passed / 1 ignored | 59 passed, 23.89 s |
| after (this ticket) | 106 s | 280 passed / 1 ignored, 70.05 s | 57 passed / 1 ignored | 58 passed, 24.93 s |

Logs: `.evidence/ticket-176-test-cad.log` (after), `.evidence/ticket-176-test-cad-before.log` (before),
`.evidence/ticket-176-precheck-lib-release.log` (first release lib run). Both full runs overlapped other lanes'
cargo work (load average ~11–14), so per-suite times are noisy and the six-test reduction carries the reliable
signal; the plane-tessellation cost that dominates the layered suite is unchanged because `1e-3` cannot hold the
existing volume tolerances.

### Tessellation tolerance deviation

Relaxing the shared `verify()` path and the tight `0.002` checks to `1e-3` fails the existing independent volume
assertions (outside tolerance by 0.003–0.06 mm³ against `0.002`: e.g. boss through-hole 1185.765 != 1185.772,
countersink 1265.343 != 1265.339, adoption ring 40583.398 != 40583.337). Those calls stay at `1e-4` so the
volume, closed-shell, topology and material probes are untouched. `1e-3` is kept where it still passes:
`outward_material_split_by_opening_remains_two_valid_solids` and `adoption_pcb_slot_preserves_both_material_components`
(planar, `1e-8` tolerance) and `source_vik_splitter_public_prepare_fixture_preserves_all_authored_geometry`
(`0.02` tolerance).

### Other changes

- `retain_evidence`'s pretty-JSON is now computed only when `LAYERED_013_EVIDENCE_OUTPUT` is set.
- `rotated_notched_wall_is_one_authored_plane` asserts `input == retained` once and runs one build plus one export.
- Removed `comparable_build_workload`, the 2D `adoption_ring_island_retains_disjoint_hole_segment_sources`, four
  STEP oracle self-tests and the `source_circle_trim_variant` helper, and the three wrapper tests in `step_import.rs`
  (the tetrahedron test now folds in `model.validate_invariants().is_ok()`).
- THQ: one read and mesh per vendor file inside `std::thread::scope`, with the tight-bounds and pin/oracle checks merged.

No CAD test name is listed in `scripts/check-native-tests-baseline.json` or `scripts/check-wasm-tests-baseline.json`,
so no baseline refresh is needed for the removed names.
